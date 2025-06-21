use crate::ast::{Camera, Object, Scene, Value};
use image::{RgbaImage, Rgba};
use imageproc::drawing::{
    draw_filled_circle_mut, draw_filled_ellipse_mut, draw_filled_rect_mut,
    draw_hollow_circle_mut, draw_hollow_ellipse_mut, draw_hollow_polygon_mut,
    draw_hollow_rect_mut, draw_line_segment_mut, draw_polygon_mut,
};
use imageproc::geometric_transformations::{rotate, Interpolation};
use imageproc::point::Point;
use imageproc::rect::Rect;
use std::collections::HashMap;

const DEFAULT_WIDTH: u32 = 1920;
const DEFAULT_HEIGHT: u32 = 1080;
const DEFAULT_BG_COLOR: Rgba<u8> = Rgba([25, 25, 25, 255]);

pub fn render_scene(scene: &Scene, camera: &Option<Camera>) -> RgbaImage {
    let width = get_camera_property_number(camera, "width")
        .map(|w| w as u32)
        .unwrap_or(DEFAULT_WIDTH);
    let height = get_camera_property_number(camera, "height")
        .map(|h| h as u32)
        .unwrap_or(DEFAULT_HEIGHT);
    let bg_color = get_camera_property_color(camera, "background_color").unwrap_or(DEFAULT_BG_COLOR);

    let mut image = RgbaImage::from_pixel(width, height, bg_color);

    for item in &scene.items {
        draw_object(&mut image, item);
    }

    image
}

fn draw_object(image: &mut RgbaImage, object: &Object) {
    let properties: HashMap<_, _> = object
        .properties
        .iter()
        .map(|p| (p.name.as_str(), &p.value))
        .collect();

    let rotation = get_property_number(&properties, "rotation").unwrap_or(0.0);

    // Create a temporary transparent canvas for the object
    let mut object_canvas = RgbaImage::from_pixel(image.width(), image.height(), Rgba([0, 0, 0, 0]));

    match object.r#type.as_str() {
        "circle" => draw_circle(&mut object_canvas, &properties),
        "square" => draw_square(&mut object_canvas, &properties),
        "triangle" => draw_triangle(&mut object_canvas, &properties),
        "rectangle" => draw_rectangle(&mut object_canvas, &properties),
        "ellipse" => draw_ellipse(&mut object_canvas, &properties),
        "line" => draw_line(&mut object_canvas, &properties),
        "arrow" | "vector" => draw_arrow(&mut object_canvas, &properties, false),
        "double_arrow" => draw_arrow(&mut object_canvas, &properties, true),
        _ => eprintln!("Warning: Unknown object type '{}'", object.r#type),
    }

    if rotation != 0.0 {
        let center = if object.r#type == "triangle" {
            // For triangles, allow overriding the center with 'position',
            // otherwise calculate the centroid.
            if let Some(pos) = get_property_tuple(&properties, "position") {
                (pos.0 as f32, pos.1 as f32)
            } else {
                let p1 = get_property_tuple(&properties, "p1").unwrap_or((0.0, 0.0));
                let p2 = get_property_tuple(&properties, "p2").unwrap_or((50.0, 50.0));
                let p3 = get_property_tuple(&properties, "p3").unwrap_or((0.0, 50.0));
                let cx = (p1.0 + p2.0 + p3.0) / 3.0;
                let cy = (p1.1 + p2.1 + p3.1) / 3.0;
                (cx as f32, cy as f32)
            }
        } else {
            // For other shapes, we assume 'position' is the center.
            get_property_tuple(&properties, "position")
                .map(|(x, y)| (x as f32, y as f32))
                .unwrap_or((0.0, 0.0))
        };

        let rotated = rotate(
            &object_canvas,
            center,
            (rotation as f32).to_radians(),
            Interpolation::Nearest,
            Rgba([0, 0, 0, 0]),
        );
        object_canvas = rotated;
    }

    // Overlay the (possibly rotated) object canvas onto the main image
    for y in 0..object_canvas.height() {
        for x in 0..object_canvas.width() {
            let pixel = object_canvas.get_pixel(x, y);
            if pixel[3] > 0 { // if not transparent
                image.put_pixel(x, y, Rgba([pixel[0], pixel[1], pixel[2], pixel[3]]));
            }
        }
    }
}

fn draw_circle(
    image: &mut RgbaImage,
    properties: &HashMap<&str, &Value>,
) {
    let position = get_property_tuple(properties, "position").unwrap_or((0.0, 0.0));
    let radius = get_property_number(properties, "radius").unwrap_or(50.0);
    let center_x = position.0 as i32;
    let center_y = position.1 as i32;

    // Handle fill
    if let Some(fill_hex) = get_property_color_str(properties, "fill") {
        draw_filled_circle_mut(image, (center_x, center_y), radius as i32, hex_to_rgba(&fill_hex));
    }

    // Handle border
    if let Some(border_hex) = get_property_color_str(properties, "border_color") {
        draw_hollow_circle_mut(
            image,
            (center_x, center_y),
            radius as i32,
            hex_to_rgba(&border_hex),
        );
    }
}

fn draw_triangle(
    image: &mut RgbaImage,
    properties: &HashMap<&str, &Value>,
) {
    let p1 = get_property_tuple(properties, "p1").unwrap_or((0.0, 0.0));
    let p2 = get_property_tuple(properties, "p2").unwrap_or((50.0, 50.0));
    let p3 = get_property_tuple(properties, "p3").unwrap_or((0.0, 50.0));

    // Handle fill
    if let Some(fill_hex) = get_property_color_str(properties, "fill") {
        let points_i32 = &[
            Point::new(p1.0 as i32, p1.1 as i32),
            Point::new(p2.0 as i32, p2.1 as i32),
            Point::new(p3.0 as i32, p3.1 as i32),
        ];
        draw_polygon_mut(image, points_i32, hex_to_rgba(&fill_hex));
    }

    // Handle border
    if let Some(border_hex) = get_property_color_str(properties, "border_color") {
        let points_f32 = &[
            Point::new(p1.0 as f32, p1.1 as f32),
            Point::new(p2.0 as f32, p2.1 as f32),
            Point::new(p3.0 as f32, p3.1 as f32),
        ];
        draw_hollow_polygon_mut(image, points_f32, hex_to_rgba(&border_hex));
    }
}

fn draw_square(
    image: &mut RgbaImage,
    properties: &HashMap<&str, &Value>,
) {
    let position = get_property_tuple(properties, "position").unwrap_or((0.0, 0.0));
    let size = get_property_number(properties, "size").unwrap_or(100.0);
    let half_size = size / 2.0;
    let top_left_x = (position.0 - half_size) as i32;
    let top_left_y = (position.1 - half_size) as i32;
    let rect = Rect::at(top_left_x, top_left_y).of_size(size as u32, size as u32);

    // Handle fill
    if let Some(fill_hex) = get_property_color_str(properties, "fill") {
        draw_filled_rect_mut(image, rect, hex_to_rgba(&fill_hex));
    }

    // Handle border
    if let Some(border_hex) = get_property_color_str(properties, "border_color") {
        draw_hollow_rect_mut(image, rect, hex_to_rgba(&border_hex));
    }
}

fn draw_rectangle(
    image: &mut RgbaImage,
    properties: &HashMap<&str, &Value>,
) {
    let position = get_property_tuple(properties, "position").unwrap_or((0.0, 0.0));
    let width = get_property_number(properties, "width").unwrap_or(100.0);
    let height = get_property_number(properties, "height").unwrap_or(50.0);

    let half_width = width / 2.0;
    let half_height = height / 2.0;
    let top_left_x = (position.0 - half_width) as i32;
    let top_left_y = (position.1 - half_height) as i32;
    let rect = Rect::at(top_left_x, top_left_y).of_size(width as u32, height as u32);

    // Handle fill
    if let Some(fill_hex) = get_property_color_str(properties, "fill") {
        draw_filled_rect_mut(image, rect, hex_to_rgba(&fill_hex));
    }

    // Handle border
    if let Some(border_hex) = get_property_color_str(properties, "border_color") {
        draw_hollow_rect_mut(image, rect, hex_to_rgba(&border_hex));
    }
}

fn draw_ellipse(
    image: &mut RgbaImage,
    properties: &HashMap<&str, &Value>,
) {
    let position = get_property_tuple(properties, "position").unwrap_or((0.0, 0.0));
    let rx = get_property_number(properties, "rx").unwrap_or(50.0);
    let ry = get_property_number(properties, "ry").unwrap_or(25.0);
    let center_x = position.0 as i32;
    let center_y = position.1 as i32;

    // Handle fill
    if let Some(fill_hex) = get_property_color_str(properties, "fill") {
        draw_filled_ellipse_mut(
            image,
            (center_x, center_y),
            rx as i32,
            ry as i32,
            hex_to_rgba(&fill_hex),
        );
    }

    // Handle border
    if let Some(border_hex) = get_property_color_str(properties, "border_color") {
        draw_hollow_ellipse_mut(
            image,
            (center_x, center_y),
            rx as i32,
            ry as i32,
            hex_to_rgba(&border_hex),
        );
    }
}

fn draw_line(
    image: &mut RgbaImage,
    properties: &HashMap<&str, &Value>,
) {
    let p1 = get_property_tuple(properties, "p1").unwrap_or((0.0, 0.0));
    let p2 = get_property_tuple(properties, "p2").unwrap_or((50.0, 50.0));

    if let Some(color_hex) = get_property_color_str(properties, "border_color") {
        draw_line_segment_mut(
            image,
            (p1.0 as f32, p1.1 as f32),
            (p2.0 as f32, p2.1 as f32),
            hex_to_rgba(&color_hex),
        );
    }
}

fn draw_arrow(
    image: &mut RgbaImage,
    properties: &HashMap<&str, &Value>,
    is_double: bool,
) {
    let p1 = get_property_tuple(properties, "p1").unwrap_or((0.0, 0.0));
    let p2 = get_property_tuple(properties, "p2").unwrap_or((50.0, 50.0));
    let color_hex =
        get_property_color_str(properties, "border_color").unwrap_or("#FFFFFF".to_string());
    let color = hex_to_rgba(&color_hex);

    // Draw the line segment
    draw_line_segment_mut(
        image,
        (p1.0 as f32, p1.1 as f32),
        (p2.0 as f32, p2.1 as f32),
        color,
    );

    // Draw arrowhead at p2
    draw_arrowhead(image, (p1.0, p1.1), (p2.0, p2.1), &properties, color);

    if is_double {
        // Draw arrowhead at p1
        draw_arrowhead(image, (p2.0, p2.1), (p1.0, p1.1), &properties, color);
    }
}

fn draw_arrowhead(
    image: &mut RgbaImage,
    from: (f64, f64),
    to: (f64, f64),
    properties: &HashMap<&str, &Value>,
    color: Rgba<u8>,
) {
    let tip_length = get_property_number(properties, "tip_length").unwrap_or(15.0);
    let tip_angle = get_property_number(properties, "tip_angle").unwrap_or(30.0);
    let angle_rad = tip_angle.to_radians();

    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let line_angle = dy.atan2(dx);

    let angle1 = line_angle + std::f64::consts::PI - angle_rad;
    let angle2 = line_angle + std::f64::consts::PI + angle_rad;

    let p1 = (
        to.0 + tip_length * angle1.cos(),
        to.1 + tip_length * angle1.sin(),
    );
    let p2 = (
        to.0 + tip_length * angle2.cos(),
        to.1 + tip_length * angle2.sin(),
    );

    let points = &[
        Point::new(to.0 as i32, to.1 as i32),
        Point::new(p1.0 as i32, p1.1 as i32),
        Point::new(p2.0 as i32, p2.1 as i32),
    ];

    draw_polygon_mut(image, points, color);
}

fn get_camera_property_number(camera: &Option<Camera>, name: &str) -> Option<f64> {
    camera.as_ref().and_then(|c| {
        c.properties
            .iter()
            .find(|p| p.name == name)
            .and_then(|p| match p.value {
                Value::Number(n) => Some(n),
                _ => None,
            })
    })
}

fn get_property_number(properties: &HashMap<&str, &Value>, name: &str) -> Option<f64> {
    properties.get(name).and_then(|v| match v {
        Value::Number(n) => Some(*n),
        _ => None,
    })
}

fn get_property_tuple(properties: &HashMap<&str, &Value>, name: &str) -> Option<(f64, f64)> {
    properties.get(name).and_then(|v| match v {
        Value::Tuple(x, y) => Some((*x, *y)),
        _ => None,
    })
}

fn get_camera_property_color(camera: &Option<Camera>, name: &str) -> Option<Rgba<u8>> {
    camera.as_ref().and_then(|c| {
        c.properties
            .iter()
            .find(|p| p.name == name)
            .and_then(|p| match &p.value {
                Value::Color(hex) => Some(hex_to_rgba(hex)),
                _ => None,
            })
    })
}

fn get_property_color_str<'a>(properties: &'a HashMap<&str, &Value>, name: &str) -> Option<String> {
    properties.get(name).and_then(|v| match v {
        Value::Color(hex) => Some(hex.clone()),
        _ => None,
    })
}

fn hex_to_rgba(hex: &str) -> Rgba<u8> {
    let rgb = hex_to_rgb(hex).unwrap_or(Rgba([255, 255, 255, 255]));
    Rgba([rgb[0], rgb[1], rgb[2], 255])
}

fn hex_to_rgb(hex: &str) -> Option<Rgba<u8>> {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Rgba([r, g, b, 255]))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Property, Value};

    #[test]
    fn test_object_without_fill_uses_background_color() {
        let camera = Some(Camera {
            properties: vec![
                Property {
                    name: "width".to_string(),
                    value: Value::Number(100.0),
                },
                Property {
                    name: "height".to_string(),
                    value: Value::Number(100.0),
                },
                Property {
                    name: "background_color".to_string(),
                    value: Value::Color("#112233".to_string()),
                },
            ],
        });

        let scene = Scene {
            name: "TestScene".to_string(),
            items: vec![Object {
                r#type: "square".to_string(),
                name: "test_square".to_string(),
                properties: vec![
                    Property {
                        name: "position".to_string(),
                        value: Value::Tuple(50.0, 50.0),
                    },
                    Property {
                        name: "size".to_string(),
                        value: Value::Number(20.0),
                    },
                    // No fill property
                ],
            }],
            timeline: None,
            duration: None,
        };

        let image = render_scene(&scene, &camera);
        let bg_color = Rgba([0x11, 0x22, 0x33, 255]);

        // Pixel inside the square
        let pixel_inside = image.get_pixel(50, 50);
        // Pixel outside the square
        let pixel_outside = image.get_pixel(10, 10);

        assert_eq!(*pixel_inside, bg_color);
        assert_eq!(*pixel_outside, bg_color);
    }

    #[test]
    fn test_hex_to_rgba() {
        assert_eq!(hex_to_rgba("#FF0000"), Rgba([255, 0, 0, 255]));
        assert_eq!(hex_to_rgba("#00FF00"), Rgba([0, 255, 0, 255]));
        assert_eq!(hex_to_rgba("#0000FF"), Rgba([0, 0, 255, 255]));
        assert_eq!(hex_to_rgba("#FFFFFF"), Rgba([255, 255, 255, 255]));
        assert_eq!(hex_to_rgba("#000000"), Rgba([0, 0, 0, 255]));
    }

    #[test]
    fn test_hex_to_rgb() {
        assert_eq!(hex_to_rgb("#FF0000"), Some(Rgba([255, 0, 0, 255])));
        assert_eq!(hex_to_rgb("#00FF00"), Some(Rgba([0, 255, 0, 255])));
        assert_eq!(hex_to_rgb("#0000FF"), Some(Rgba([0, 0, 255, 255])));
        assert_eq!(hex_to_rgb("invalid"), None);
        assert_eq!(hex_to_rgb("#FFF"), None);
    }

    #[test]
    fn test_get_property_number() {
        let mut properties = std::collections::HashMap::new();
        let value = Value::Number(42.0);
        properties.insert("test", &value);
        
        assert_eq!(get_property_number(&properties, "test"), Some(42.0));
        assert_eq!(get_property_number(&properties, "nonexistent"), None);
        
        let string_value = Value::String("not_a_number".to_string());
        properties.insert("string", &string_value);
        assert_eq!(get_property_number(&properties, "string"), None);
    }

    #[test]
    fn test_get_property_tuple() {
        let mut properties = std::collections::HashMap::new();
        let value = Value::Tuple(10.0, 20.0);
        properties.insert("position", &value);
        
        assert_eq!(get_property_tuple(&properties, "position"), Some((10.0, 20.0)));
        assert_eq!(get_property_tuple(&properties, "nonexistent"), None);
        
        let number_value = Value::Number(42.0);
        properties.insert("number", &number_value);
        assert_eq!(get_property_tuple(&properties, "number"), None);
    }

    #[test]
    fn test_get_property_color_str() {
        let mut properties = std::collections::HashMap::new();
        let value = Value::Color("#FF0000".to_string());
        properties.insert("fill", &value);
        
        assert_eq!(get_property_color_str(&properties, "fill"), Some("#FF0000".to_string()));
        assert_eq!(get_property_color_str(&properties, "nonexistent"), None);
        
        let number_value = Value::Number(42.0);
        properties.insert("number", &number_value);
        assert_eq!(get_property_color_str(&properties, "number"), None);
    }

    #[test]
    fn test_get_camera_property_number() {
        let camera = Some(Camera {
            properties: vec![
                Property {
                    name: "width".to_string(),
                    value: Value::Number(1920.0),
                },
                Property {
                    name: "height".to_string(),
                    value: Value::Number(1080.0),
                },
            ],
        });
        
        assert_eq!(get_camera_property_number(&camera, "width"), Some(1920.0));
        assert_eq!(get_camera_property_number(&camera, "height"), Some(1080.0));
        assert_eq!(get_camera_property_number(&camera, "nonexistent"), None);
        
        let no_camera: Option<Camera> = None;
        assert_eq!(get_camera_property_number(&no_camera, "width"), None);
    }

    #[test]
    fn test_get_camera_property_color() {
        let camera = Some(Camera {
            properties: vec![
                Property {
                    name: "background_color".to_string(),
                    value: Value::Color("#FF0000".to_string()),
                },
            ],
        });
        
        assert_eq!(get_camera_property_color(&camera, "background_color"), Some(Rgba([255, 0, 0, 255])));
        assert_eq!(get_camera_property_color(&camera, "nonexistent"), None);
        
        let no_camera: Option<Camera> = None;
        assert_eq!(get_camera_property_color(&no_camera, "background_color"), None);
    }

    #[test]
    fn test_render_scene_with_custom_camera() {
        let camera = Some(Camera {
            properties: vec![
                Property {
                    name: "width".to_string(),
                    value: Value::Number(200.0),
                },
                Property {
                    name: "height".to_string(),
                    value: Value::Number(100.0),
                },
                Property {
                    name: "background_color".to_string(),
                    value: Value::Color("#FFFF00".to_string()),
                },
            ],
        });

        let scene = Scene {
            name: "TestScene".to_string(),
            items: vec![],
            timeline: None,
            duration: None,
        };

        let image = render_scene(&scene, &camera);
        assert_eq!(image.width(), 200);
        assert_eq!(image.height(), 100);
        
        let pixel = image.get_pixel(50, 50);
        assert_eq!(*pixel, Rgba([255, 255, 0, 255]));
    }

    #[test]
    fn test_render_scene_with_default_camera() {
        let scene = Scene {
            name: "TestScene".to_string(),
            items: vec![],
            timeline: None,
            duration: None,
        };

        let image = render_scene(&scene, &None);
        assert_eq!(image.width(), DEFAULT_WIDTH);
        assert_eq!(image.height(), DEFAULT_HEIGHT);
        
        let pixel = image.get_pixel(100, 100);
        assert_eq!(*pixel, DEFAULT_BG_COLOR);
    }

    #[test]
    fn test_render_scene_with_circle() {
        let scene = Scene {
            name: "TestScene".to_string(),
            items: vec![Object {
                r#type: "circle".to_string(),
                name: "test_circle".to_string(),
                properties: vec![
                    Property {
                        name: "position".to_string(),
                        value: Value::Tuple(50.0, 50.0),
                    },
                    Property {
                        name: "radius".to_string(),
                        value: Value::Number(20.0),
                    },
                    Property {
                        name: "fill".to_string(),
                        value: Value::Color("#FF0000".to_string()),
                    },
                ],
            }],
            timeline: None,
            duration: None,
        };

        let camera = Some(Camera {
            properties: vec![
                Property {
                    name: "width".to_string(),
                    value: Value::Number(100.0),
                },
                Property {
                    name: "height".to_string(),
                    value: Value::Number(100.0),
                },
                Property {
                    name: "background_color".to_string(),
                    value: Value::Color("#000000".to_string()),
                },
            ],
        });

        let image = render_scene(&scene, &camera);
        assert_eq!(image.width(), 100);
        assert_eq!(image.height(), 100);
        
        let center_pixel = image.get_pixel(50, 50);
        assert_eq!(*center_pixel, Rgba([255, 0, 0, 255]));
    }

    #[test]
    fn test_render_unknown_object_type() {
        let scene = Scene {
            name: "TestScene".to_string(),
            items: vec![Object {
                r#type: "unknown_shape".to_string(),
                name: "test_unknown".to_string(),
                properties: vec![],
            }],
            timeline: None,
            duration: None,
        };

        let image = render_scene(&scene, &None);
        assert_eq!(image.width(), DEFAULT_WIDTH);
        assert_eq!(image.height(), DEFAULT_HEIGHT);
    }
}