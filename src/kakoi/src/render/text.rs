use crate::arena::{ArenaKey, Structure, Value};
use crate::spatial_bound::SpatialBound;
use crate::spatial_tree::SpatialTreeData;
use crate::{camera::Camera, sphere::Sphere};
use slotmap::SlotMap;
use wgpu_glyph::GlyphCruncher;

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
                    .with_scale(instance.scale)],
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

pub struct TextConstraintInstance {
    key: ArenaKey,
    scale: f32,
    width: f32,
    height: f32,
    bound: Sphere,
    scaled_radius: f32,
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
        let mut section = wgpu_glyph::Section {
            screen_position: (0.0, 0.0),
            bounds: (f32::INFINITY, f32::INFINITY),
            text: vec![wgpu_glyph::Text::new(&text)
                .with_color([0.0, 0.0, 0.0, 1.0])
                .with_scale(20.0)],
            ..wgpu_glyph::Section::default()
        };
        // TODO: make this work with cuboid_inside_bound instead:
        let bound_sphere = SpatialBound::sphere_inside_bound(bound);
        let scaled_radius = if viewport_width > viewport_height {
            viewport_width * bound_sphere.radius
        } else {
            viewport_height * bound_sphere.radius
        };
        let (width, height) =
            Self::binary_search_for_text_scale(glyph_brush, &mut section, scaled_radius);
        let scale = section.text[0].scale.y;
        Self {
            key: *key,
            width,
            height,
            scale,
            bound: bound_sphere,
            scaled_radius,
            transformation: Self::calculate_transformation(
                view_projection_matrix,
                &bound_sphere,
                scaled_radius,
            ),
        }
    }

    fn set_view_projection_matrix(&mut self, view_projection_matrix: &cgmath::Matrix4<f32>) {
        self.transformation =
            Self::calculate_transformation(view_projection_matrix, &self.bound, self.scaled_radius)
    }

    fn calculate_transformation(
        view_projection_matrix: &cgmath::Matrix4<f32>,
        sphere: &Sphere,
        scaled_radius: f32,
    ) -> [f32; 16] {
        // TODO: possible division by zero error?
        let transformation = cgmath::Matrix4::from_nonuniform_scale(
            sphere.radius / scaled_radius,
            -sphere.radius / scaled_radius,
            1.0,
        );
        let transformation = cgmath::Matrix4::from_translation(sphere.center) * transformation;
        *(view_projection_matrix * transformation).as_mut()
    }

    fn binary_search_for_text_scale(
        glyph_brush: &mut wgpu_glyph::GlyphBrush<()>,
        section: &mut wgpu_glyph::Section,
        scaled_radius: f32,
    ) -> (f32, f32) {
        use wgpu_glyph::ab_glyph::PxScale;

        const SCALE_TOLERENCE: f32 = 1.0;

        let mut min_scale: PxScale = 0.0.into();
        let mut max_scale: PxScale = (scaled_radius * 2.0).into();
        let mut previous_scale: Option<PxScale> = None;
        let mut current_scale = (min_scale.y * 0.5 + max_scale.y * 0.5).into();
        let mut width = 0.0;
        let mut height = 0.0;
        let mut target = None;

        section.text[0].scale = current_scale;

        loop {
            match glyph_brush.glyph_bounds(&section.clone()) {
                Some(rect) => {
                    let old_ps = previous_scale;
                    previous_scale = Some(current_scale);
                    let rect_width = rect.width();
                    let rect_height = rect.height();
                    width = rect_width;
                    height = rect_height;
                    if let Some(ps) = old_ps {
                        if (ps.y - current_scale.y).abs() < SCALE_TOLERENCE {
                            break;
                        }
                    }
                    let max_dimension = rect_width.max(rect_height);
                    if target.is_none() {
                        let aspect_ratio = width / height;
                        let (scale_x, scale_y) = Sphere {
                            radius: scaled_radius,
                            // It doesn't matter what radius we choose here.
                            center: cgmath::vec3(0.0, 0.0, 0.0),
                        }
                        .as_rectangle_bounds(aspect_ratio);
                        target = Some(scale_x.max(scale_y));
                    }
                    if max_dimension > target.unwrap() {
                        max_scale = current_scale;
                    } else {
                        min_scale = current_scale;
                    }
                    current_scale = (min_scale.y * 0.5 + max_scale.y * 0.5).into();
                    section.text[0].scale = current_scale;
                }
                None => break,
            }
        }

        (width, height)
    }
}
