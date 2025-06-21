use crate::ast::{BeamScript, Scene, Value};
use crate::{gpu_renderer, renderer};
use image::RgbaImage;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::fs;
use std::process::Command;
use std::time::Duration;

const FRAME_RATE: u64 = 60;

pub fn animate_script(script: &BeamScript, output_base: &str, gpu: bool) {
    if script.scenes.is_empty() {
        println!("No scenes to render.");
        return;
    }

    let temp_dir = "temp_frames";
    if fs::metadata(temp_dir).is_ok() {
        fs::remove_dir_all(temp_dir).expect("Failed to remove old temp directory");
    }
    fs::create_dir_all(temp_dir).expect("Failed to create temp directory");

    let mut gpu_state = if gpu {
        Some(pollster::block_on(gpu_renderer::GpuRendererState::new()))
    } else {
        None
    };

    let total_frames: u64 = script
        .scenes
        .iter()
        .map(|scene| {
            let duration = scene
                .duration
                .unwrap_or_else(|| Duration::from_secs(2));
            (duration.as_secs_f64() * FRAME_RATE as f64).ceil() as u64
        })
        .sum();

    println!(
        "Rendering a total of {} frames from {} scene(s)...",
        total_frames,
        script.scenes.len()
    );
    let bar = ProgressBar::new(total_frames);
    bar.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap(),
    );

    let mut frame_offset = 0;
    for scene in &script.scenes {
        let duration = scene
            .duration
            .unwrap_or_else(|| Duration::from_secs(2));
        let num_frames_for_scene = (duration.as_secs_f64() * FRAME_RATE as f64).ceil() as u64;

        if let Some(timeline) = &scene.timeline {
            // Animated scene
            if gpu {
                for i in 0..num_frames_for_scene {
                    let current_time = Duration::from_secs_f64(i as f64 / FRAME_RATE as f64);
                    let mut frame_scene = scene.clone();
                    apply_animations(&mut frame_scene, timeline, current_time);

                    let image: RgbaImage = pollster::block_on(gpu_renderer::render_scene_gpu(
                        gpu_state.as_mut().unwrap(),
                        &frame_scene.items,
                        &script.camera,
                    ));
                    let frame_path = format!("{}/frame_{:05}.png", temp_dir, frame_offset + i);
                    image.save(frame_path).expect("Failed to save frame");
                }
            } else {
                (0..num_frames_for_scene)
                    .into_par_iter()
                    .for_each(|i| {
                        let current_time = Duration::from_secs_f64(i as f64 / FRAME_RATE as f64);
                        let mut frame_scene = scene.clone();
                        apply_animations(&mut frame_scene, timeline, current_time);

                        let image: RgbaImage = renderer::render_scene(&frame_scene, &script.camera);
                        let frame_path = format!("{}/frame_{:05}.png", temp_dir, frame_offset + i);
                        image.save(frame_path).expect("Failed to save frame");
                    });
            }
        } else {
            // Static scene
            if gpu {
                let image: RgbaImage = pollster::block_on(gpu_renderer::render_scene_gpu(
                    gpu_state.as_mut().unwrap(),
                    &scene.items,
                    &script.camera,
                ));
                for i in 0..num_frames_for_scene {
                    let frame_path = format!("{}/frame_{:05}.png", temp_dir, frame_offset + i);
                    image.save(&frame_path).expect("Failed to save frame");
                }
            } else {
                let image: RgbaImage = renderer::render_scene(scene, &script.camera);
                (0..num_frames_for_scene)
                    .into_par_iter()
                    .for_each(|i| {
                        let frame_path = format!("{}/frame_{:05}.png", temp_dir, frame_offset + i);
                        image.save(&frame_path).expect("Failed to save frame");
                    });
            }
        }

        bar.inc(num_frames_for_scene);
        frame_offset += num_frames_for_scene;
    }
    bar.finish_with_message("All frames rendered");

    println!("🎬 Assembling video with ffmpeg...");

    let output_path = format!("{}.mp4", output_base);
    let output = Command::new("ffmpeg")
        .arg("-r")
        .arg(FRAME_RATE.to_string())
        .arg("-s")
        .arg(format!(
            "{}x{}",
            script
                .camera
                .as_ref()
                .and_then(|c| c
                    .properties
                    .iter()
                    .find(|p| p.name == "width")
                    .and_then(|p| match p.value {
                        Value::Number(n) => Some(n as u32),
                        _ => None,
                    }))
                .unwrap_or(1920),
            script
                .camera
                .as_ref()
                .and_then(|c| c
                    .properties
                    .iter()
                    .find(|p| p.name == "height")
                    .and_then(|p| match p.value {
                        Value::Number(n) => Some(n as u32),
                        _ => None,
                    }))
                .unwrap_or(1080)
        ))
        .arg("-i")
        .arg(format!("{}/frame_%05d.png", temp_dir))
        .arg("-c:v")
        .arg("libx264")
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg("-y") // Overwrite output file if it exists
        .arg(&output_path)
        .output()
        .expect("Failed to execute ffmpeg");

    if !output.status.success() {
        eprintln!("ffmpeg error: {}", String::from_utf8_lossy(&output.stderr));
    } else {
        println!("✅ Video saved to {}", output_path);
    }

    fs::remove_dir_all(temp_dir).expect("Failed to remove temp directory");
}

fn apply_animations(scene: &mut Scene, timeline: &crate::ast::Timeline, current_time: Duration) {
    // Create a list of all unique properties that are animated in this timeline.
    let mut animated_properties = std::collections::HashMap::new();
    for anim in &timeline.animations {
        animated_properties.insert((anim.target_object.clone(), anim.property.clone()), ());
    }

    for (object_name, property_name) in animated_properties.keys() {
        // For each unique property, find its state at `current_time`.
        // First, find all animations for this property, sorted by start time.
        let mut relevant_animations: Vec<_> = timeline
            .animations
            .iter()
            .filter(|a| &a.target_object == object_name && &a.property == property_name)
            .collect();
        relevant_animations.sort_by_key(|a| a.start);

        // Find the initial value from the scene definition to start with.
        let initial_value = scene
            .items
            .iter()
            .find(|o| &o.name == object_name)
            .and_then(|o| o.properties.iter().find(|p| &p.name == property_name))
            .map(|p| p.value.clone())
            .expect("Animated property not found in scene object");

        let mut final_value = initial_value;

        // Chronologically apply animations to find the value at `current_time`.
        for anim in relevant_animations {
            if current_time >= anim.start {
                let start_value = final_value.clone();
                let end_value = anim.to.clone();

                // Check if the animation is currently active and interpolating.
                if anim.end.is_some() && current_time < anim.end.unwrap() {
                    let animation_duration = anim.end.unwrap() - anim.start;
                    let elapsed = current_time - anim.start;

                    // Avoid division by zero for zero-duration animations.
                    let mut factor = if animation_duration.as_secs_f64() > 0.0 {
                        elapsed.as_secs_f64() / animation_duration.as_secs_f64()
                    } else {
                        1.0
                    };

                    if let Some(easing) = &anim.easing {
                        factor = apply_easing(factor, easing);
                    }

                    final_value = lerp(&start_value, &end_value, factor);
                    // This is the dominant state, so we're done with this property for this frame.
                    break;
                } else {
                    // This is either a finished animation or an instant `at X` animation.
                    // Its end value becomes the new base state for subsequent animations.
                    final_value = end_value;
                }
            } else {
                // This animation (and all subsequent ones) are in the future, so we can stop.
                break;
            }
        }

        // Find the property in the scene and update it with the final calculated value.
        if let Some(object) = scene.items.iter_mut().find(|o| &o.name == object_name) {
            if let Some(property) = object.properties.iter_mut().find(|p| &p.name == property_name)
            {
                property.value = final_value;
            }
        }
    }
}

// Linear interpolation
fn lerp(start: &Value, end: &Value, factor: f64) -> Value {
    match (start, end) {
        (Value::Number(s), Value::Number(e)) => Value::Number(s + (e - s) * factor),
        (Value::Tuple(sx, sy), Value::Tuple(ex, ey)) => {
            Value::Tuple(sx + (ex - sx) * factor, sy + (ey - sy) * factor)
        }
        (Value::Color(s_hex), Value::Color(e_hex)) => {
            let s_rgb = hex_to_rgb(s_hex);
            let e_rgb = hex_to_rgb(e_hex);
            let r = s_rgb[0] as f64 + (e_rgb[0] as f64 - s_rgb[0] as f64) * factor;
            let g = s_rgb[1] as f64 + (e_rgb[1] as f64 - s_rgb[1] as f64) * factor;
            let b = s_rgb[2] as f64 + (e_rgb[2] as f64 - s_rgb[2] as f64) * factor;
            Value::Color(format!("#{:02x}{:02x}{:02x}", r as u8, g as u8, b as u8))
        }
        _ => end.clone(), // No interpolation for mismatched or unsupported types
    }
}

fn apply_easing(t: f64, easing_type: &str) -> f64 {
    match easing_type {
        "ease_in" => t * t,
        "ease_out" => t * (2.0 - t),
        "ease_in_out" => {
            if t < 0.5 {
                2.0 * t * t
            } else {
                -1.0 + (4.0 - 2.0 * t) * t
            }
        }
        _ => t, // Default to linear
    }
}

fn hex_to_rgb(hex: &str) -> [u8; 3] {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
        [r, g, b]
    } else {
        [0, 0, 0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Animation, Object, Property, Timeline};

    #[test]
    fn test_ease_in_animation() {
        let mut scene = Scene {
            name: "TestScene".to_string(),
            items: vec![Object {
                r#type: "square".to_string(),
                name: "test_square".to_string(),
                properties: vec![Property {
                    name: "position".to_string(),
                    value: Value::Tuple(0.0, 0.0),
                }],
            }],
            timeline: None,
            duration: Some(Duration::from_secs(1)),
        };

        let timeline = Timeline {
            animations: vec![Animation {
                start: Duration::from_secs(0),
                end: Some(Duration::from_secs(1)),
                target_object: "test_square".to_string(),
                property: "position".to_string(),
                to: Value::Tuple(100.0, 0.0),
                easing: Some("ease_in".to_string()),
            }],
        };

        let current_time = Duration::from_millis(500); // 0.5s
        apply_animations(&mut scene, &timeline, current_time);

        let final_pos = scene.items[0]
            .properties
            .iter()
            .find(|p| p.name == "position")
            .unwrap()
            .value
            .clone();

        assert_eq!(final_pos, Value::Tuple(25.0, 0.0));
    }

    #[test]
    fn test_ease_out_animation() {
        let mut scene = Scene {
            name: "TestScene".to_string(),
            items: vec![Object {
                r#type: "square".to_string(),
                name: "test_square".to_string(),
                properties: vec![Property {
                    name: "position".to_string(),
                    value: Value::Tuple(0.0, 0.0),
                }],
            }],
            timeline: None,
            duration: Some(Duration::from_secs(1)),
        };

        let timeline = Timeline {
            animations: vec![Animation {
                start: Duration::from_secs(0),
                end: Some(Duration::from_secs(1)),
                target_object: "test_square".to_string(),
                property: "position".to_string(),
                to: Value::Tuple(100.0, 0.0),
                easing: Some("ease_out".to_string()),
            }],
        };

        let current_time = Duration::from_millis(500); // 0.5s
        apply_animations(&mut scene, &timeline, current_time);

        let final_pos = scene.items[0]
            .properties
            .iter()
            .find(|p| p.name == "position")
            .unwrap()
            .value
            .clone();

        assert_eq!(final_pos, Value::Tuple(75.0, 0.0));
    }

    #[test]
    fn test_ease_in_out_animation() {
        let mut scene = Scene {
            name: "TestScene".to_string(),
            items: vec![Object {
                r#type: "square".to_string(),
                name: "test_square".to_string(),
                properties: vec![Property {
                    name: "position".to_string(),
                    value: Value::Tuple(0.0, 0.0),
                }],
            }],
            timeline: None,
            duration: Some(Duration::from_secs(1)),
        };

        let timeline = Timeline {
            animations: vec![Animation {
                start: Duration::from_secs(0),
                end: Some(Duration::from_secs(1)),
                target_object: "test_square".to_string(),
                property: "position".to_string(),
                to: Value::Tuple(100.0, 0.0),
                easing: Some("ease_in_out".to_string()),
            }],
        };

        let current_time = Duration::from_millis(500); // 0.5s
        apply_animations(&mut scene, &timeline, current_time);

        let final_pos = scene.items[0]
            .properties
            .iter()
            .find(|p| p.name == "position")
            .unwrap()
            .value
            .clone();

        assert_eq!(final_pos, Value::Tuple(50.0, 0.0));
    }

    #[test]
    fn test_lerp_number() {
        let start = Value::Number(0.0);
        let end = Value::Number(100.0);
        let result = lerp(&start, &end, 0.5);
        assert_eq!(result, Value::Number(50.0));
    }

    #[test]
    fn test_lerp_tuple() {
        let start = Value::Tuple(0.0, 0.0);
        let end = Value::Tuple(100.0, 200.0);
        let result = lerp(&start, &end, 0.5);
        assert_eq!(result, Value::Tuple(50.0, 100.0));
    }

    #[test]
    fn test_lerp_color() {
        let start = Value::Color("#000000".to_string());
        let end = Value::Color("#ffffff".to_string());
        let result = lerp(&start, &end, 0.5);
        assert_eq!(result, Value::Color("#7f7f7f".to_string()));
    }

    #[test]
    fn test_lerp_unsupported_types() {
        let start = Value::String("start".to_string());
        let end = Value::String("end".to_string());
        let result = lerp(&start, &end, 0.5);
        assert_eq!(result, Value::String("end".to_string()));
    }

    #[test]
    fn test_hex_to_rgb() {
        assert_eq!(hex_to_rgb("#FF0000"), [255, 0, 0]);
        assert_eq!(hex_to_rgb("#00FF00"), [0, 255, 0]);
        assert_eq!(hex_to_rgb("#0000FF"), [0, 0, 255]);
        assert_eq!(hex_to_rgb("#FFFFFF"), [255, 255, 255]);
        assert_eq!(hex_to_rgb("#000000"), [0, 0, 0]);
        assert_eq!(hex_to_rgb("invalid"), [0, 0, 0]);
    }

    #[test]
    fn test_apply_easing_ease_in() {
        assert_eq!(apply_easing(0.0, "ease_in"), 0.0);
        assert_eq!(apply_easing(0.5, "ease_in"), 0.25);
        assert_eq!(apply_easing(1.0, "ease_in"), 1.0);
    }

    #[test]
    fn test_apply_easing_ease_out() {
        assert_eq!(apply_easing(0.0, "ease_out"), 0.0);
        assert_eq!(apply_easing(0.5, "ease_out"), 0.75);
        assert_eq!(apply_easing(1.0, "ease_out"), 1.0);
    }

    #[test]
    fn test_apply_easing_ease_in_out() {
        assert_eq!(apply_easing(0.0, "ease_in_out"), 0.0);
        assert_eq!(apply_easing(0.25, "ease_in_out"), 0.125);
        assert_eq!(apply_easing(0.5, "ease_in_out"), 0.5);
        assert_eq!(apply_easing(0.75, "ease_in_out"), 0.875);
        assert_eq!(apply_easing(1.0, "ease_in_out"), 1.0);
    }

    #[test]
    fn test_apply_easing_unknown() {
        assert_eq!(apply_easing(0.5, "unknown"), 0.5);
    }

    #[test]
    fn test_multiple_animations_same_property() {
        let mut scene = Scene {
            name: "TestScene".to_string(),
            items: vec![Object {
                r#type: "square".to_string(),
                name: "test_square".to_string(),
                properties: vec![Property {
                    name: "position".to_string(),
                    value: Value::Tuple(0.0, 0.0),
                }],
            }],
            timeline: None,
            duration: Some(Duration::from_secs(3)),
        };

        let timeline = Timeline {
            animations: vec![
                Animation {
                    start: Duration::from_secs(0),
                    end: Some(Duration::from_secs(1)),
                    target_object: "test_square".to_string(),
                    property: "position".to_string(),
                    to: Value::Tuple(50.0, 0.0),
                    easing: None,
                },
                Animation {
                    start: Duration::from_secs(1),
                    end: Some(Duration::from_secs(2)),
                    target_object: "test_square".to_string(),
                    property: "position".to_string(),
                    to: Value::Tuple(100.0, 0.0),
                    easing: None,
                },
            ],
        };

        let current_time = Duration::from_millis(1500);
        apply_animations(&mut scene, &timeline, current_time);

        let final_pos = scene.items[0]
            .properties
            .iter()
            .find(|p| p.name == "position")
            .unwrap()
            .value
            .clone();

        assert_eq!(final_pos, Value::Tuple(75.0, 0.0));
    }

    #[test]
    fn test_instant_animation() {
        let mut scene = Scene {
            name: "TestScene".to_string(),
            items: vec![Object {
                r#type: "circle".to_string(),
                name: "test_circle".to_string(),
                properties: vec![Property {
                    name: "radius".to_string(),
                    value: Value::Number(10.0),
                }],
            }],
            timeline: None,
            duration: Some(Duration::from_secs(2)),
        };

        let timeline = Timeline {
            animations: vec![Animation {
                start: Duration::from_secs(1),
                end: None,
                target_object: "test_circle".to_string(),
                property: "radius".to_string(),
                to: Value::Number(50.0),
                easing: None,
            }],
        };

        let current_time = Duration::from_millis(1500);
        apply_animations(&mut scene, &timeline, current_time);

        let final_radius = scene.items[0]
            .properties
            .iter()
            .find(|p| p.name == "radius")
            .unwrap()
            .value
            .clone();

        assert_eq!(final_radius, Value::Number(50.0));
    }

    #[test]
    fn test_animation_before_start_time() {
        let mut scene = Scene {
            name: "TestScene".to_string(),
            items: vec![Object {
                r#type: "square".to_string(),
                name: "test_square".to_string(),
                properties: vec![Property {
                    name: "size".to_string(),
                    value: Value::Number(100.0),
                }],
            }],
            timeline: None,
            duration: Some(Duration::from_secs(2)),
        };

        let timeline = Timeline {
            animations: vec![Animation {
                start: Duration::from_secs(1),
                end: Some(Duration::from_secs(2)),
                target_object: "test_square".to_string(),
                property: "size".to_string(),
                to: Value::Number(200.0),
                easing: None,
            }],
        };

        let current_time = Duration::from_millis(500);
        apply_animations(&mut scene, &timeline, current_time);

        let final_size = scene.items[0]
            .properties
            .iter()
            .find(|p| p.name == "size")
            .unwrap()
            .value
            .clone();

        assert_eq!(final_size, Value::Number(100.0));
    }
} 