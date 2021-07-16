use crate::arena::{ArenaKey, Structure, Value};
use crate::camera::Camera;
use crate::spatial_bound::SpatialBound;
use crate::spatial_tree::SpatialTreeData;
use cgmath::Vector3;
use slotmap::SlotMap;
use wgpu_glyph::{GlyphBrush, GlyphCruncher};

pub struct TextRenderer {
    constraints: Vec<SpatialTreeData>,
    instances_cache: Vec<TextConstraintInstance>,
    instances_cache_stale: bool,
    glyph_brush: wgpu_glyph::GlyphBrush<()>,
    staging_belt: wgpu::util::StagingBelt,
    local_pool: futures::executor::LocalPool,
    local_spawner: futures::executor::LocalSpawner,
}

impl TextRenderer {
    pub fn new<'a>(device: &'a wgpu::Device, sc_desc: &'a wgpu::SwapChainDescriptor) -> Self {
        // Not exactly sure what size to set here. Smaller sizes (~1024) seem to
        // cause lag. Larger sizes (~4096) seem to cause less lag. Ideally, we'd
        // base this number on an estimate of how much data we would upload into
        // it. See https://docs.rs/wgpu/0.7.0/wgpu/util/struct.StagingBelt.html
        // for more information.
        let staging_belt = wgpu::util::StagingBelt::new(2u64.pow(16));

        let local_pool = futures::executor::LocalPool::new();
        let local_spawner = local_pool.spawner();

        let glyph_brush = {
            let font = wgpu_glyph::ab_glyph::FontArc::try_from_slice(include_bytes!(
                "../resources/fonts/CooperHewitt-OTF-public/CooperHewitt-Book.otf"
            ))
            .unwrap();
            wgpu_glyph::GlyphBrushBuilder::using_font(font).build(&device, sc_desc.format)
        };

        Self {
            constraints: Vec::new(),
            instances_cache: Vec::new(),
            instances_cache_stale: true,
            glyph_brush,
            staging_belt,
            local_pool,
            local_spawner,
        }
    }

    pub fn with_instance<'a>(&mut self, spatial_tree_data: SpatialTreeData) {
        self.constraints.push(spatial_tree_data);
    }

    pub fn resize<'a>(&mut self) {
        self.instances_cache_stale = true;
    }

    pub fn render<'a>(
        &mut self,
        store: &'a SlotMap<ArenaKey, Value>,
        device: &'a wgpu::Device,
        sc_desc: &'a wgpu::SwapChainDescriptor,
        encoder: &'a mut wgpu::CommandEncoder,
        texture_view: &'a wgpu::TextureView,
        camera: &'a mut Camera,
    ) {
        Self::build_instances(
            store,
            &mut self.instances_cache,
            self.instances_cache_stale,
            &self.constraints,
            &mut self.glyph_brush,
            camera.view_projection_matrix(),
            sc_desc,
        );
        self.instances_cache_stale = false;
        for instance in &self.instances_cache {
            let text = match &store.get(instance.key).unwrap().structure {
                Structure::String(s) => s,
                _ => panic!(),
            };
            let section = wgpu_glyph::Section {
                screen_position: (-instance.width * 0.5, -instance.height * 0.5),
                bounds: (f32::INFINITY, f32::INFINITY),
                text: vec![wgpu_glyph::Text::new(text.as_ref())
                    .with_color([1.0, 1.0, 1.0, 1.0])
                    .with_scale(instance.text_scale)],
                ..wgpu_glyph::Section::default()
            };
            self.glyph_brush.queue(&section);
            self.glyph_brush
                .draw_queued_with_transform(
                    device,
                    &mut self.staging_belt,
                    encoder,
                    texture_view,
                    instance.transformation,
                )
                .unwrap(); // It seems like this function always returns Ok(())...?
        }

        self.staging_belt.finish();
    }

    pub fn post_render(&mut self) {
        use futures::task::SpawnExt;

        self.local_spawner
            .spawn(self.staging_belt.recall())
            .expect("Recall staging belt");

        self.local_pool.run_until_stalled();
    }

    pub fn invalidate(&mut self) {
        self.constraints.clear();
        self.instances_cache_stale = true;
    }

    fn build_instances<'a, 'b>(
        store: &'b SlotMap<ArenaKey, Value>,
        instances_cache: &'a mut Vec<TextConstraintInstance>,
        instances_cache_stale: bool,
        constraints: &'a Vec<SpatialTreeData>,
        glyph_brush: &'b mut wgpu_glyph::GlyphBrush<()>,
        view_projection_matrix: &'a cgmath::Matrix4<f32>,
        sc_desc: &'b wgpu::SwapChainDescriptor,
    ) {
        if instances_cache_stale {
            instances_cache.clear();
            for SpatialTreeData { key, bounds: bound } in constraints {
                instances_cache.push(TextConstraintInstance::new(
                    store,
                    key,
                    glyph_brush,
                    bound,
                    view_projection_matrix,
                    sc_desc.width as f32,
                    sc_desc.height as f32,
                ));
            }
        } else {
            for instance in instances_cache {
                instance.set_view_projection_matrix(view_projection_matrix);
            }
        }
    }
}

#[derive(Debug)]
pub struct TextConstraintInstance {
    /// Location of the text we want to render in an [Arena].
    key: ArenaKey,

    /// Point scale of the text.
    text_scale: f32,

    /// Scalar used to convert a transformation in our coordinate system to
    /// glyph_brush's coordinate system.
    transform_scale: f32,

    /// The width, in pixels, that the text will be rendered at. Note: this is
    /// not necessarily the width of the text on screen.
    width: f32,

    /// The height, in pixels, that the text will be rendered at. Note: this is
    /// not necessarily the height of the text on screen.
    height: f32,

    /// The center of the text's bounding box (the actual center, not the top
    /// left corner).
    center: Vector3<f32>,

    transformation: [f32; 16],
}

impl TextConstraintInstance {
    pub fn new(
        store: &SlotMap<ArenaKey, Value>,
        key: &ArenaKey,
        glyph_brush: &mut wgpu_glyph::GlyphBrush<()>,
        bound: &SpatialBound,
        view_projection_matrix: &cgmath::Matrix4<f32>,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Self {
        let text = match &store.get(*key).unwrap().structure {
            Structure::String(s) => s,
            _ => panic!(),
        };

        // Use an arbitrary default scale (20.0) to determine the aspect ratio
        // of the text's bounding box.
        let mut section = wgpu_glyph::Section {
            screen_position: (0.0, 0.0),
            bounds: (f32::INFINITY, f32::INFINITY),
            text: vec![wgpu_glyph::Text::new(&text)
                .with_color([0.0, 0.0, 0.0, 1.0])
                .with_scale(20.0)],
            ..wgpu_glyph::Section::default()
        };
        let (tw, th) = Self::text_dimensions(glyph_brush, &section);
        // The true aspect ratio (what you would see on screen) is (tw / th).
        // Since our spatial bound parameter comes from virtual coordinate space
        // (which goes from -1..1 in all dimensions), we need to squish /
        // stretch our aspect ratio to account for later transformations.
        let aspect_ratio = (tw / th) * (viewport_height / viewport_width);

        // Now that we've got our adjusted aspect ratio, we need to calculate
        // the desired size (in virtual coordinate space) of our text (as
        // opposed to the arbitrary, 20pt one we've got now).
        let cuboid = SpatialBound::cuboid_inside_bound(bound, aspect_ratio);
        let (width, height) = {
            let (w, h) = cuboid.dimensions_2d();
            (w * 0.5 * viewport_width, h * 0.5 * viewport_height)
        };
        // 'diff' gives the amount to scale our (currently 20pt) text so that it
        // fits nicely in our desired bounding box.
        let diff = width / tw;
        section.text[0].scale = (section.text[0].scale.x * diff).into();

        let virtual_height = SpatialBound::cuboid_inside_bound(bound, width / height).height();

        let text_scale = section.text[0].scale.x;
        let transform_scale = virtual_height / height;

        Self {
            key: *key,
            width: width,
            height: height,
            text_scale,
            transform_scale,
            center: cuboid.center,
            transformation: Self::calculate_transformation(
                view_projection_matrix,
                cuboid.center,
                transform_scale,
            ),
        }
    }

    fn text_dimensions(
        glyph_brush: &mut GlyphBrush<()>,
        section: &wgpu_glyph::Section,
    ) -> (f32, f32) {
        match glyph_brush.glyph_bounds(section.clone()) {
            Some(rect) => (rect.width(), rect.height()),
            None => (1.0, 1.0),
        }
    }

    fn set_view_projection_matrix(&mut self, view_projection_matrix: &cgmath::Matrix4<f32>) {
        self.transformation = Self::calculate_transformation(
            view_projection_matrix,
            self.center,
            self.transform_scale,
        )
    }

    fn calculate_transformation(
        view_projection_matrix: &cgmath::Matrix4<f32>,
        center: Vector3<f32>,
        scale: f32,
    ) -> [f32; 16] {
        // glyph_brush's coordinate system is (annoyingly) different from ours.
        // To be perfectly honest I forget why this code works. I'm scared to
        // touch it any more though. From what I recall, glyph brush assumes
        // that the screen's top left corner is (0,0) and its width and height
        // are the actual width and height of the screen (in pixels). This is
        // not our coordinate system (where (0,0) is in the center of the
        // screen, with x and y ranging from -1 to 1). Anyway, the important
        // part is that the transformation passed to glyph_brush's
        // draw_queued_with_transform function operates in THEIR coordinate
        // system, not ours, so we adjust for that here (with 'scale').
        let transformation = cgmath::Matrix4::from_nonuniform_scale(scale, -scale, 1.0);
        let transformation = cgmath::Matrix4::from_translation(center) * transformation;
        *(view_projection_matrix * transformation).as_mut()
    }
}
