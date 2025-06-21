use pest::iterators::Pair;
use pest::Parser;
use std::time::Duration;

use crate::ast::{Animation, BeamScript, Camera, Object, Property, Scene, Timeline, Value};

#[derive(pest_derive::Parser)]
#[grammar = "beam.pest"]
pub struct BeamParser;

struct ParsedTimeline {
    scene_name: String,
    animations: Vec<Animation>,
}

pub fn parse_str(input: &str) -> Result<BeamScript, Box<dyn std::error::Error>> {
    let mut pairs = BeamParser::parse(Rule::file, input)?;
    let file = pairs.next().unwrap();

    let mut camera: Option<Camera> = None;
    let mut scenes: Vec<Scene> = Vec::new();
    let mut temp_timelines: Vec<ParsedTimeline> = Vec::new();

    for pair in file.into_inner() {
        match pair.as_rule() {
            Rule::camera => camera = Some(parse_camera(pair)),
            Rule::scene => scenes.push(parse_scene(pair)),
            Rule::timeline => temp_timelines.push(parse_temp_timeline(pair)),
            Rule::EOI | Rule::COMMENT => (),
            _ => {
                println!("Unexpected rule: {:?}", pair.as_rule());
                unreachable!();
            }
        }
    }

    // Link timelines to scenes
    for temp_timeline in temp_timelines {
        if let Some(scene) = scenes
            .iter_mut()
            .find(|s| s.name == temp_timeline.scene_name)
        {
            scene.timeline = Some(Timeline {
                animations: temp_timeline.animations,
            });
        } else {
            eprintln!(
                "Warning: Timeline found for non-existent scene '{}'",
                temp_timeline.scene_name
            );
        }
    }

    Ok(BeamScript { camera, scenes })
}

fn parse_scene(pair: Pair<Rule>) -> Scene {
    let mut inner = pair.into_inner();
    let name = parse_string_literal(inner.next().unwrap());

    let mut items = Vec::new();
    let mut duration: Option<Duration> = None;

    for content in inner {
        match content.as_rule() {
            Rule::object => items.push(parse_object(content)),
            Rule::scene_duration => {
                duration = Some(parse_time_value(content.into_inner().next().unwrap()));
            }
            _ => (), // Skip comments
        }
    }

    Scene {
        name,
        items,
        timeline: None,
        duration,
    }
}

fn parse_object(pair: Pair<Rule>) -> Object {
    let mut inner = pair.into_inner();
    let r#type = inner.next().unwrap().as_str().to_string();
    let name = parse_string_literal(inner.next().unwrap());

    let properties = inner.map(parse_property).collect();

    Object {
        r#type,
        name,
        properties,
    }
}

fn parse_property(pair: Pair<Rule>) -> Property {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let value = parse_value(inner.next().unwrap());

    Property { name, value }
}

fn parse_value(pair: Pair<Rule>) -> Value {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::string_literal => Value::String(parse_string_literal(inner)),
        Rule::number => Value::Number(inner.as_str().parse().unwrap()),
        Rule::hex_color => Value::Color(inner.as_str().to_string()),
        Rule::tuple => {
            let mut inner = inner.into_inner();
            let x = inner.next().unwrap().as_str().parse().unwrap();
            let y = inner.next().unwrap().as_str().parse().unwrap();
            Value::Tuple(x, y)
        }
        _ => unreachable!(),
    }
}

fn parse_string_literal(pair: Pair<Rule>) -> String {
    pair.as_str().trim_matches('"').to_string()
}

fn parse_temp_timeline(pair: Pair<Rule>) -> ParsedTimeline {
    let mut inner = pair.into_inner();
    let scene_name = parse_string_literal(inner.next().unwrap());
    let animations = inner.map(parse_animation).collect();
    ParsedTimeline {
        scene_name,
        animations,
    }
}

fn parse_animation(pair: Pair<Rule>) -> Animation {
    let mut inner = pair.into_inner();
    let time_pair = inner.next().unwrap();
    let (start, end) = parse_animation_time(time_pair);

    let target_pair = inner.next().unwrap();
    let (target_object, property) = parse_target_property(target_pair);

    let to = parse_value(inner.next().unwrap());

    let easing = inner.next().map(|p| {
        p.into_inner().next().unwrap().as_str().to_string()
    });

    Animation {
        start,
        end,
        target_object,
        property,
        to,
        easing,
    }
}

fn parse_animation_time(pair: Pair<Rule>) -> (Duration, Option<Duration>) {
    let mut inner = pair.into_inner();
    let kind = inner.next().unwrap();
    match kind.as_rule() {
        Rule::animation_instant => {
            let start = parse_time_value(kind.into_inner().next().unwrap());
            (start, None)
        }
        Rule::animation_range => {
            let mut inner = kind.into_inner();
            let start = parse_time_value(inner.next().unwrap());
            let end = parse_time_value(inner.next().unwrap());
            (start, Some(end))
        }
        _ => unreachable!(),
    }
}

fn parse_time_value(pair: Pair<Rule>) -> Duration {
    let mut inner = pair.into_inner();
    let value: u64 = inner.next().unwrap().as_str().parse().unwrap();
    let unit = inner.next().unwrap().as_str();

    match unit {
        "s" => Duration::from_secs(value),
        "ms" => Duration::from_millis(value),
        _ => unreachable!(),
    }
}

fn parse_target_property(pair: Pair<Rule>) -> (String, String) {
    let mut inner = pair.into_inner();
    let target_object = parse_string_literal(inner.next().unwrap());
    let property = inner.next().unwrap().as_str().to_string();
    (target_object, property)
}

fn parse_camera(pair: Pair<Rule>) -> Camera {
    let properties = pair.into_inner().map(parse_property).collect();
    Camera { properties }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_scene() {
        let input = r#"
            scene "MyFirstAnimation" {
                circle "logo" {
                    radius: 50,
                    fill: #00A0D8,
                    position: (0, 0)
                }
            }
        "#;
        let expected = BeamScript {
            scenes: vec![Scene {
                name: "MyFirstAnimation".to_string(),
                items: vec![Object {
                    r#type: "circle".to_string(),
                    name: "logo".to_string(),
                    properties: vec![
                        Property {
                            name: "radius".to_string(),
                            value: Value::Number(50.0),
                        },
                        Property {
                            name: "fill".to_string(),
                            value: Value::Color("#00A0D8".to_string()),
                        },
                        Property {
                            name: "position".to_string(),
                            value: Value::Tuple(0.0, 0.0),
                        },
                    ],
                }],
                timeline: None,
                duration: None,
            }],
            ..Default::default()
        };
        let ast = parse_str(input).unwrap();
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_scene_with_timeline() {
        let input = r#"
            scene "MyAnimation" {
                square "box" { size: 100 }
            }
            timeline for "MyAnimation" {
                at 1s, "box".color -> #FF0000;
            }
        "#;
        let expected = BeamScript {
            scenes: vec![Scene {
                name: "MyAnimation".to_string(),
                items: vec![Object {
                    r#type: "square".to_string(),
                    name: "box".to_string(),
                    properties: vec![Property {
                        name: "size".to_string(),
                        value: Value::Number(100.0),
                    }],
                }],
                timeline: Some(Timeline {
                    animations: vec![Animation {
                        start: Duration::from_secs(1),
                        end: None,
                        target_object: "box".to_string(),
                        property: "color".to_string(),
                        to: Value::Color("#FF0000".to_string()),
                        easing: None,
                    }],
                }),
                duration: None,
            }],
            ..Default::default()
        };
        let ast = parse_str(input).unwrap();
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_camera() {
        let input = r#"
            camera {
                width: 1280,
                height: 720,
                background_color: #333333,
            }
        "#;
        let expected = BeamScript {
            camera: Some(Camera {
                properties: vec![
                    Property {
                        name: "width".to_string(),
                        value: Value::Number(1280.0),
                    },
                    Property {
                        name: "height".to_string(),
                        value: Value::Number(720.0),
                    },
                    Property {
                        name: "background_color".to_string(),
                        value: Value::Color("#333333".to_string()),
                    },
                ],
            }),
            ..Default::default()
        };
        let ast = parse_str(input).unwrap();
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_triangle_scene() {
        let input = r#"
            scene "MyTriangleAnimation" {
                triangle "tri" {
                    p1: (10, 10),
                    p2: (100, 10),
                    p3: (55, 100),
                    fill: #00FF00,
                }
            }
        "#;
        let expected = BeamScript {
            scenes: vec![Scene {
                name: "MyTriangleAnimation".to_string(),
                items: vec![Object {
                    r#type: "triangle".to_string(),
                    name: "tri".to_string(),
                    properties: vec![
                        Property {
                            name: "p1".to_string(),
                            value: Value::Tuple(10.0, 10.0),
                        },
                        Property {
                            name: "p2".to_string(),
                            value: Value::Tuple(100.0, 10.0),
                        },
                        Property {
                            name: "p3".to_string(),
                            value: Value::Tuple(55.0, 100.0),
                        },
                        Property {
                            name: "fill".to_string(),
                            value: Value::Color("#00FF00".to_string()),
                        },
                    ],
                }],
                timeline: None,
                duration: None,
            }],
            ..Default::default()
        };
        let ast = parse_str(input).unwrap();
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_rotation_animation() {
        let input = r#"
            scene "Test" {
                square "s" { position: (100, 100) }
            }
            timeline for "Test" {
                at 0s to 1s, "s".rotation -> 360;
            }
        "#;

        let script = parse_str(input).unwrap();
        let scene = &script.scenes[0];
        let timeline = scene.timeline.as_ref().unwrap();
        let animation = &timeline.animations[0];

        assert_eq!(animation.property, "rotation");
        assert_eq!(animation.to, Value::Number(360.0));
    }

    #[test]
    fn test_parse_border_color() {
        let input = r#"
            scene "Test" {
                square "s" {
                    border_color: #FF0000,
                }
            }
        "#;
        let script = parse_str(input).unwrap();
        let scene = &script.scenes[0];
        let object = &scene.items[0];
        let property = &object.properties[0];

        assert_eq!(property.name, "border_color");
        assert_eq!(property.value, Value::Color("#FF0000".to_string()));
    }

    #[test]
    fn test_parse_rectangle() {
        let input = r#"
            scene "Test" {
                rectangle "r" {
                    width: 100,
                    height: 50,
                }
            }
        "#;
        let script = parse_str(input).unwrap();
        let scene = &script.scenes[0];
        let object = &scene.items[0];
        assert_eq!(object.r#type, "rectangle");
    }

    #[test]
    fn test_parse_ellipse() {
        let input = r#"
            scene "Test" {
                ellipse "e" {
                    rx: 50,
                    ry: 25,
                }
            }
        "#;
        let script = parse_str(input).unwrap();
        let scene = &script.scenes[0];
        let object = &scene.items[0];
        assert_eq!(object.r#type, "ellipse");
    }

    #[test]
    fn test_parse_line() {
        let input = r#"
            scene "Test" {
                line "l" {
                    p1: (0, 0),
                    p2: (100, 100),
                }
            }
        "#;
        let script = parse_str(input).unwrap();
        let scene = &script.scenes[0];
        let object = &scene.items[0];
        assert_eq!(object.r#type, "line");
    }

    #[test]
    fn test_parse_arrow() {
        let input = r#"
            scene "Test" {
                arrow "a" {}
            }
        "#;
        let script = parse_str(input).unwrap();
        assert_eq!(script.scenes[0].items[0].r#type, "arrow");
    }

    #[test]
    fn test_parse_double_arrow() {
        let input = r#"
            scene "Test" {
                double_arrow "da" {}
            }
        "#;
        let script = parse_str(input).unwrap();
        assert_eq!(script.scenes[0].items[0].r#type, "double_arrow");
    }

    #[test]
    fn test_parse_vector() {
        let input = r#"
            scene "Test" {
                vector "v" {}
            }
        "#;
        let script = parse_str(input).unwrap();
        assert_eq!(script.scenes[0].items[0].r#type, "vector");
    }

    #[test]
    fn test_parse_empty_scene() {
        let input = r#"
            scene "EmptyScene" {}
        "#;
        let script = parse_str(input).unwrap();
        assert_eq!(script.scenes.len(), 1);
        assert_eq!(script.scenes[0].name, "EmptyScene");
        assert!(script.scenes[0].items.is_empty());
    }

    #[test]
    fn test_parse_scene_with_duration() {
        let input = r#"
            scene "TimedScene" {
                duration: 5s
                circle "c" { radius: 25 }
            }
        "#;
        let script = parse_str(input).unwrap();
        assert_eq!(script.scenes[0].duration, Some(Duration::from_secs(5)));
    }

    #[test]
    fn test_parse_duration_milliseconds() {
        let input = r#"
            scene "Test" {
                duration: 1500ms
            }
        "#;
        let script = parse_str(input).unwrap();
        assert_eq!(script.scenes[0].duration, Some(Duration::from_millis(1500)));
    }

    #[test]
    fn test_parse_animation_range_with_easing() {
        let input = r#"
            scene "Test" {
                square "s" { position: (0, 0) }
            }
            timeline for "Test" {
                at 1s to 3s, "s".position -> (100, 100), with ease_in;
            }
        "#;
        let script = parse_str(input).unwrap();
        let timeline = script.scenes[0].timeline.as_ref().unwrap();
        let animation = &timeline.animations[0];
        
        assert_eq!(animation.start, Duration::from_secs(1));
        assert_eq!(animation.end, Some(Duration::from_secs(3)));
        assert_eq!(animation.easing, Some("ease_in".to_string()));
    }

    #[test]
    fn test_parse_multiple_scenes() {
        let input = r#"
            scene "Scene1" {
                circle "c1" { radius: 10 }
            }
            scene "Scene2" {
                square "s1" { size: 50 }
            }
        "#;
        let script = parse_str(input).unwrap();
        assert_eq!(script.scenes.len(), 2);
        assert_eq!(script.scenes[0].name, "Scene1");
        assert_eq!(script.scenes[1].name, "Scene2");
    }

    #[test]
    fn test_parse_invalid_input_fails() {
        let input = "invalid beam syntax";
        let result = parse_str(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_timeline_for_nonexistent_scene() {
        let input = r#"
            scene "Scene1" {
                circle "c" { radius: 10 }
            }
            timeline for "NonexistentScene" {
                at 1s, "c".radius -> 20;
            }
        "#;
        let script = parse_str(input).unwrap();
        assert_eq!(script.scenes.len(), 1);
        assert!(script.scenes[0].timeline.is_none());
    }
}