use crate::ast::{Camera, Object, Property, Value};
use vello::{kurbo, peniko, Renderer, RendererOptions, Scene};
use image::{ImageBuffer, Rgba};

pub struct GpuRendererState {
    device: vello::wgpu::Device,
    queue: vello::wgpu::Queue,
    renderer: Renderer,
}

impl GpuRendererState {
    pub async fn new() -> Self {
        let instance = vello::wgpu::Instance::new(&vello::wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&vello::wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&vello::wgpu::DeviceDescriptor::default(), None)
            .await
            .unwrap();
        let renderer = Renderer::new(
            &device,
            RendererOptions {
                use_cpu: false,
                num_init_threads: None,
                antialiasing_support: vello::AaSupport::all(),
                pipeline_cache: None,
            },
        )
        .unwrap();
        Self {
            device,
            queue,
            renderer,
        }
    }
}

fn get_property<'a>(properties: &'a [Property], name: &str) -> Option<&'a Value> {
    properties
        .iter()
        .find(|p| p.name == name)
        .map(|p| &p.value)
}

fn get_position(properties: &[Property]) -> (f64, f64) {
    if let Some(Value::Tuple(x, y)) = get_property(properties, "position") {
        (*x, *y)
    } else {
        (0.0, 0.0)
    }
}

fn get_radius(properties: &[Property]) -> f64 {
    if let Some(Value::Number(r)) = get_property(properties, "radius") {
        *r
    } else {
        0.0
    }
}

fn get_fill_color(properties: &[Property]) -> Option<peniko::Color> {
    if let Some(Value::Color(c)) = get_property(properties, "fill").or_else(|| get_property(properties, "color")) {
        let c = c.trim_start_matches('#');
        let r = u8::from_str_radix(&c[0..2], 16).unwrap();
        let g = u8::from_str_radix(&c[2..4], 16).unwrap();
        let b = u8::from_str_radix(&c[4..6], 16).unwrap();
        Some(peniko::Color::from_rgb8(r, g, b))
    } else {
        None
    }
}

fn get_stroke_color(properties: &[Property]) -> Option<peniko::Color> {
    if let Some(Value::Color(c)) = get_property(properties, "border_color") {
        let c = c.trim_start_matches('#');
        let r = u8::from_str_radix(&c[0..2], 16).unwrap();
        let g = u8::from_str_radix(&c[2..4], 16).unwrap();
        let b = u8::from_str_radix(&c[4..6], 16).unwrap();
        Some(peniko::Color::from_rgb8(r, g, b))
    } else {
        None
    }
}

fn get_width(properties: &[Property]) -> f64 {
    if let Some(Value::Number(w)) = get_property(properties, "width") {
        *w
    } else {
        0.0
    }
}

fn get_height(properties: &[Property]) -> f64 {
    if let Some(Value::Number(h)) = get_property(properties, "height") {
        *h
    } else {
        0.0
    }
}

fn get_size(properties: &[Property]) -> f64 {
    if let Some(Value::Number(s)) = get_property(properties, "size") {
        *s
    } else {
        0.0
    }
}

fn get_rx(properties: &[Property]) -> f64 {
    if let Some(Value::Number(r)) = get_property(properties, "rx") {
        *r
    } else {
        0.0
    }
}

fn get_ry(properties: &[Property]) -> f64 {
    if let Some(Value::Number(r)) = get_property(properties, "ry") {
        *r
    } else {
        0.0
    }
}


fn get_p1(properties: &[Property]) -> (f64, f64) {
    if let Some(Value::Tuple(x, y)) = get_property(properties, "p1") {
        (*x, *y)
    } else {
        (0.0, 0.0)
    }
}

fn get_p2(properties: &[Property]) -> (f64, f64) {
    if let Some(Value::Tuple(x, y)) = get_property(properties, "p2") {
        (*x, *y)
    } else {
        (0.0, 0.0)
    }
}

fn get_p3(properties: &[Property]) -> (f64, f64) {
    if let Some(Value::Tuple(x, y)) = get_property(properties, "p3") {
        (*x, *y)
    } else {
        (0.0, 0.0)
    }
}

fn get_camera_width(camera: &Option<Camera>) -> u32 {
    if let Some(camera) = camera {
        if let Some(Value::Number(w)) = get_property(&camera.properties, "width") {
            *w as u32
        } else {
            1280
        }
    } else {
        1280
    }
}

fn get_camera_height(camera: &Option<Camera>) -> u32 {
    if let Some(camera) = camera {
        if let Some(Value::Number(h)) = get_property(&camera.properties, "height") {
            *h as u32
        } else {
            720
        }
    } else {
        720
    }
}

pub async fn render_scene_gpu(
    state: &mut GpuRendererState,
    items: &[Object],
    camera: &Option<Camera>,
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let width = get_camera_width(camera);
    let height = get_camera_height(camera);

    let mut scene = Scene::new();
    for item in items {
        if item.r#type == "circle" {
            let position = get_position(&item.properties);
            let radius = get_radius(&item.properties);
            let fill_color = get_fill_color(&item.properties);
            let stroke_color = get_stroke_color(&item.properties);

            let circle = kurbo::Circle::new((position.0, position.1), radius);
            if let Some(color) = fill_color {
                scene.fill(peniko::Fill::NonZero, kurbo::Affine::IDENTITY, &color, None, &circle);
            }
            if let Some(color) = stroke_color {
                scene.stroke(&kurbo::Stroke::new(1.0), kurbo::Affine::IDENTITY, &color, None, &circle);
            }
        } else if item.r#type == "square" {
            let position = get_position(&item.properties);
            let size = get_size(&item.properties);
            let fill_color = get_fill_color(&item.properties);
            let stroke_color = get_stroke_color(&item.properties);

            let rect = kurbo::Rect::new(
                position.0,
                position.1,
                position.0 + size,
                position.1 + size,
            );
            if let Some(color) = fill_color {
                scene.fill(peniko::Fill::NonZero, kurbo::Affine::IDENTITY, &color, None, &rect);
            }
            if let Some(color) = stroke_color {
                scene.stroke(&kurbo::Stroke::new(1.0), kurbo::Affine::IDENTITY, &color, None, &rect);
            }
        } else if item.r#type == "rectangle" {
            let position = get_position(&item.properties);
            let width = get_width(&item.properties);
            let height = get_height(&item.properties);
            let fill_color = get_fill_color(&item.properties);
            let stroke_color = get_stroke_color(&item.properties);

            let rect = kurbo::Rect::new(
                position.0,
                position.1,
                position.0 + width,
                position.1 + height,
            );
            if let Some(color) = fill_color {
                scene.fill(peniko::Fill::NonZero, kurbo::Affine::IDENTITY, &color, None, &rect);
            }
            if let Some(color) = stroke_color {
                scene.stroke(&kurbo::Stroke::new(1.0), kurbo::Affine::IDENTITY, &color, None, &rect);
            }
        } else if item.r#type == "ellipse" {
            let position = get_position(&item.properties);
            let rx = get_rx(&item.properties);
            let ry = get_ry(&item.properties);
            let fill_color = get_fill_color(&item.properties);
            let stroke_color = get_stroke_color(&item.properties);

            let ellipse = kurbo::Ellipse::new(
                (position.0, position.1),
                (rx, ry),
                0.0,
            );
            if let Some(color) = fill_color {
                scene.fill(peniko::Fill::NonZero, kurbo::Affine::IDENTITY, &color, None, &ellipse);
            }
            if let Some(color) = stroke_color {
                scene.stroke(&kurbo::Stroke::new(1.0), kurbo::Affine::IDENTITY, &color, None, &ellipse);
            }
        } else if item.r#type == "line" {
            let start = get_p1(&item.properties);
            let end = get_p2(&item.properties);
            let stroke_color = get_stroke_color(&item.properties);

            if let Some(color) = stroke_color {
                scene.stroke(
                    &kurbo::Stroke::new(1.0),
                    kurbo::Affine::IDENTITY,
                    &color,
                    None,
                    &kurbo::Line::new(start, end),
                );
            }
        } else if item.r#type == "triangle" {
            let p1 = get_p1(&item.properties);
            let p2 = get_p2(&item.properties);
            let p3 = get_p3(&item.properties);
            let fill_color = get_fill_color(&item.properties);
            let stroke_color = get_stroke_color(&item.properties);

            let mut path = kurbo::BezPath::new();
            path.move_to(p1);
            path.line_to(p2);
            path.line_to(p3);
            path.close_path();

            if let Some(color) = fill_color {
                scene.fill(peniko::Fill::NonZero, kurbo::Affine::IDENTITY, &color, None, &path);
            }
            if let Some(color) = stroke_color {
                scene.stroke(&kurbo::Stroke::new(1.0), kurbo::Affine::IDENTITY, &color, None, &path);
            }
        } else if item.r#type == "arrow" || item.r#type == "double_arrow" {
            let p1 = get_p1(&item.properties);
            let p2 = get_p2(&item.properties);
            let stroke_color = get_stroke_color(&item.properties);

            if let Some(color) = stroke_color {
                // Draw the main line
                scene.stroke(
                    &kurbo::Stroke::new(1.0),
                    kurbo::Affine::IDENTITY,
                    &color,
                    None,
                    &kurbo::Line::new(p1, p2),
                );

                // Draw arrowhead at p2
                draw_arrowhead(&mut scene, p1, p2, &color);

                if item.r#type == "double_arrow" {
                    // Draw arrowhead at p1
                    draw_arrowhead(&mut scene, p2, p1, &color);
                }
            }
        }
    }

    let size = vello::wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture_desc = vello::wgpu::TextureDescriptor {
        label: Some("vello_texture"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: vello::wgpu::TextureDimension::D2,
        format: vello::wgpu::TextureFormat::Rgba8Unorm,
        usage: vello::wgpu::TextureUsages::STORAGE_BINDING | vello::wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    };
    let texture = state.device.create_texture(&texture_desc);
    let view = texture.create_view(&vello::wgpu::TextureViewDescriptor::default());

    state.renderer
        .render_to_texture(
            &state.device,
            &state.queue,
            &scene,
            &view,
            &vello::RenderParams {
                base_color: peniko::Color::TRANSPARENT,
                width,
                height,
                antialiasing_method: vello::AaConfig::Msaa16,
            },
        )
        .unwrap();

    let output_buffer_desc = vello::wgpu::BufferDescriptor {
        label: Some("output_buffer"),
        size: (width * height * 4) as u64,
        usage: vello::wgpu::BufferUsages::MAP_READ | vello::wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    };
    let output_buffer = state.device.create_buffer(&output_buffer_desc);

    let mut encoder = state.device.create_command_encoder(&vello::wgpu::CommandEncoderDescriptor::default());

    encoder.copy_texture_to_buffer(
        vello::wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: vello::wgpu::Origin3d::ZERO,
            aspect: vello::wgpu::TextureAspect::All,
        },
        vello::wgpu::TexelCopyBufferInfo {
            buffer: &output_buffer,
            layout: vello::wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
        },
        size,
    );

    state.queue.submit(Some(encoder.finish()));

    let buffer_slice = output_buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer_slice.map_async(vello::wgpu::MapMode::Read, move |result| {
        tx.send(result).unwrap();
    });

    state.device.poll(vello::wgpu::Maintain::Wait);
    rx.recv().unwrap().unwrap();

    let data = buffer_slice.get_mapped_range();
    ImageBuffer::from_raw(width, height, data.to_vec()).unwrap()
}

fn draw_arrowhead(scene: &mut Scene, from: (f64, f64), to: (f64, f64), color: &peniko::Color) {
    let length = 10.0;
    let angle = std::f64::consts::PI / 6.0; // 30 degrees

    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let line_angle = dy.atan2(dx);

    let angle1 = line_angle + std::f64::consts::PI - angle;
    let angle2 = line_angle + std::f64::consts::PI + angle;

    let p1 = (to.0 + length * angle1.cos(), to.1 + length * angle1.sin());
    let p2 = to;
    let p3 = (to.0 + length * angle2.cos(), to.1 + length * angle2.sin());

    let mut path = kurbo::BezPath::new();
    path.move_to(p1);
    path.line_to(p2);
    path.line_to(p3);
    path.close_path();

    scene.fill(peniko::Fill::NonZero, kurbo::Affine::IDENTITY, color, None, &path);
} 