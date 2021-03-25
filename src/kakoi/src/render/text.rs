use crate::{sphere::Sphere, state};
use bitvec::view;
use std::collections::HashMap;
use wgpu_glyph::GlyphCruncher;

pub struct TextConstraintBuilder {
    constraints: HashMap<String, Vec<Sphere>>,
    instances_cache: Option<Vec<TextConstraintInstance>>,
    glyph_brush: wgpu_glyph::GlyphBrush<()>,
    staging_belt: wgpu::util::StagingBelt,
    local_pool: futures::executor::LocalPool,
    local_spawner: futures::executor::LocalSpawner,
    view_projection_matrix: cgmath::Matrix4<f32>,
}

impl TextConstraintBuilder {
    pub fn new<'a>(device: &'a wgpu::Device, sc_desc: &'a wgpu::SwapChainDescriptor) -> Self {
        // Not exactly sure what size to set here. Smaller sizes (~1024) seem to
        // cause lag. Larger sizes (~4096) seem to cause less lag. Ideally, we'd
        // base this number on an estimate of how much data we would upload into
        // it. See https://docs.rs/wgpu/0.7.0/wgpu/util/struct.StagingBelt.html
        // for more information.
        let staging_belt = wgpu::util::StagingBelt::new(4096);

        let local_pool = futures::executor::LocalPool::new();
        let local_spawner = local_pool.spawner();

        let glyph_brush = {
            let font = wgpu_glyph::ab_glyph::FontArc::try_from_slice(include_bytes!(
                "../resources/fonts/CooperHewitt-OTF-public/CooperHewitt-Book.otf"
            ))
            .unwrap();
            wgpu_glyph::GlyphBrushBuilder::using_font(font).build(&device, sc_desc.format)
        };

        let view_projection_matrix = crate::camera::Camera::new(1.0).build_view_projection_matrix();

        Self {
            constraints: HashMap::new(),
            instances_cache: None,
            glyph_brush,
            staging_belt,
            local_pool,
            local_spawner,
            view_projection_matrix,
        }
    }

    pub fn with_constraint(&mut self, text: String, sphere: Sphere) {
        self.constraints
            .entry(text)
            .or_insert_with(|| Vec::with_capacity(1))
            .push(sphere);
    }

    pub fn resize(&mut self, view_projection_matrix: cgmath::Matrix4<f32>) {
        self.view_projection_matrix = view_projection_matrix;
    }

    pub fn build_instances<'a, 'b>(
        instances_cache: &'a mut Option<Vec<TextConstraintInstance>>,
        constraints: &'a HashMap<String, Vec<Sphere>>,
        glyph_brush: &'b mut wgpu_glyph::GlyphBrush<()>,
        view_projection_matrix: &'a cgmath::Matrix4<f32>,
        sc_desc: &'b wgpu::SwapChainDescriptor,
        refresh_cache: bool,
    ) -> &'a Vec<TextConstraintInstance> {
        if instances_cache.is_none() || refresh_cache {
            let mut instances: Vec<TextConstraintInstance> = Vec::new();

            let mut build_onekey_instances = |text: String, spheres| {
                for sphere in spheres {
                    instances.push(TextConstraintInstance::new(
                        text.clone(),
                        glyph_brush,
                        sphere,
                        view_projection_matrix,
                        sc_desc.width as f32,
                        sc_desc.height as f32,
                    ));
                }
            };

            for (text, spheres) in constraints {
                build_onekey_instances(text.clone(), spheres);
            }

            *instances_cache = Some(instances);
        } else {
            for instance in instances_cache.as_mut().unwrap() {
                instance.set_view_projection_matrix(view_projection_matrix);
            }
        }

        instances_cache.as_ref().unwrap()
    }

    pub fn render<'a>(
        &mut self,
        sc_desc: &'a wgpu::SwapChainDescriptor,
        device: &'a wgpu::Device,
        encoder: &'a mut wgpu::CommandEncoder,
        texture_view: &'a wgpu::TextureView,
    ) {
        let text_constraint_instances = Self::build_instances(
            &mut self.instances_cache,
            &self.constraints,
            &mut self.glyph_brush,
            &self.view_projection_matrix,
            sc_desc,
            false,
        );
        for instance in text_constraint_instances {
            // Don't draw text that is too small to be seen clearly.
            if instance.scale > 5.0 {
                let section = wgpu_glyph::Section {
                    screen_position: (-instance.width * 0.5, -instance.height * 0.5),
                    bounds: (f32::INFINITY, f32::INFINITY),
                    text: vec![wgpu_glyph::Text::new(&instance.text)
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
}

pub struct TextConstraintInstance {
    text: String,
    scale: f32,
    width: f32,
    height: f32,
    sphere: Sphere,
    scaled_radius: f32,
    transformation: [f32; 16],
}

impl TextConstraintInstance {
    pub fn new(
        text: String,
        _glyph_brush: &mut wgpu_glyph::GlyphBrush<()>,
        _sphere: &Sphere,
        _view_projection_matrix: &cgmath::Matrix4<f32>,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Self {
        let mut section = wgpu_glyph::Section {
            screen_position: (0.0, 0.0),
            bounds: (f32::INFINITY, f32::INFINITY),
            text: vec![wgpu_glyph::Text::new(&text)
                .with_color([0.0, 0.0, 0.0, 1.0])
                .with_scale(20.0)],
            ..wgpu_glyph::Section::default()
        };
        let scaled_radius = if viewport_width > viewport_height {
            viewport_width * _sphere.radius
        } else {
            viewport_height * _sphere.radius
        };
        let (width, height) =
            Self::binary_search_for_text_scale(_glyph_brush, &mut section, scaled_radius);
        let scale = section.text[0].scale.y;
        Self {
            text: text,
            width,
            height,
            scale,
            sphere: *_sphere,
            scaled_radius,
            transformation: Self::calculate_transformation(
                _view_projection_matrix,
                _sphere,
                scaled_radius,
            ),
        }
    }

    fn set_view_projection_matrix(&mut self, view_projection_matrix: &cgmath::Matrix4<f32>) {
        self.transformation =
            Self::calculate_transformation(view_projection_matrix, &self.sphere, self.scaled_radius)
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

        let mut min_scale: PxScale = 0.0.into();
        let mut max_scale: PxScale = scaled_radius.into();
        let mut previous_scale: Option<PxScale> = None;
        let mut current_scale = (min_scale.y * 0.5 + max_scale.y * 0.5).into();
        let mut width = 0.0;
        let mut height = 0.0;
        let target = ((2.0 * scaled_radius).powf(2.0) * 0.5).sqrt();

        section.text[0].scale = current_scale;

        // Perform a binary search between [min_scale, max_scale] for the
        // correct text scale. We stop our search when the difference between
        // our previous and current text scale is small enough to not effect its
        // bounding box (i.e., the bounding box drawn from the current text
        // scale has the same dimensions as the bounding box drawn from the
        // previous text scale).
        while Some(current_scale) != previous_scale {
            match glyph_brush.glyph_bounds(&section.clone()) {
                Some(rect) => {
                    previous_scale = Some(current_scale);
                    let rect_width = rect.width();
                    let rect_height = rect.height();
                    width = rect_width;
                    height = rect_height;
                    let max_dimension = rect_width.max(rect_height);
                    if max_dimension > target {
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

pub struct TextConstraintRenderer<'b> {
    pub text_constraint_instances: &'b Vec<TextConstraintInstance>,
    pub device: &'b mut wgpu::Device,
    pub glyph_brush: &'b mut wgpu_glyph::GlyphBrush<()>,
    pub encoder: &'b mut wgpu::CommandEncoder,
    pub staging_belt: &'b mut wgpu::util::StagingBelt,
    pub texture_view: &'b wgpu::TextureView,
}

impl<'b> TextConstraintRenderer<'b> {}