use std::time::Duration;

#[derive(Debug, PartialEq, Default, Clone)]
pub struct BeamScript {
    pub camera: Option<Camera>,
    pub scenes: Vec<Scene>,
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Camera {
    pub properties: Vec<Property>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Scene {
    pub name: String,
    pub items: Vec<Object>,
    pub timeline: Option<Timeline>,
    pub duration: Option<Duration>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Object {
    pub r#type: String,
    pub name: String,
    pub properties: Vec<Property>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Property {
    pub name: String,
    pub value: Value,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    String(String),
    Number(f64),
    Color(String),
    Tuple(f64, f64),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Timeline {
    pub animations: Vec<Animation>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Animation {
    pub start: Duration,
    pub end: Option<Duration>,
    pub target_object: String,
    pub property: String,
    pub to: Value,
    pub easing: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beam_script_default() {
        let script = BeamScript::default();
        assert!(script.camera.is_none());
        assert!(script.scenes.is_empty());
    }

    #[test]
    fn test_camera_default() {
        let camera = Camera::default();
        assert!(camera.properties.is_empty());
    }

    #[test]
    fn test_value_variants() {
        assert_eq!(Value::String("test".to_string()), Value::String("test".to_string()));
        assert_eq!(Value::Number(42.0), Value::Number(42.0));
        assert_eq!(Value::Color("#FF0000".to_string()), Value::Color("#FF0000".to_string()));
        assert_eq!(Value::Tuple(10.0, 20.0), Value::Tuple(10.0, 20.0));
        
        assert_ne!(Value::Number(42.0), Value::String("42".to_string()));
        assert_ne!(Value::Tuple(10.0, 20.0), Value::Tuple(20.0, 10.0));
    }

    #[test]
    fn test_scene_construction() {
        let scene = Scene {
            name: "TestScene".to_string(),
            items: vec![],
            timeline: None,
            duration: Some(Duration::from_secs(5)),
        };
        
        assert_eq!(scene.name, "TestScene");
        assert!(scene.items.is_empty());
        assert!(scene.timeline.is_none());
        assert_eq!(scene.duration, Some(Duration::from_secs(5)));
    }

    #[test]
    fn test_object_construction() {
        let object = Object {
            r#type: "circle".to_string(),
            name: "my_circle".to_string(),
            properties: vec![
                Property {
                    name: "radius".to_string(),
                    value: Value::Number(50.0),
                }
            ],
        };
        
        assert_eq!(object.r#type, "circle");
        assert_eq!(object.name, "my_circle");
        assert_eq!(object.properties.len(), 1);
    }

    #[test]
    fn test_animation_construction() {
        let animation = Animation {
            start: Duration::from_secs(0),
            end: Some(Duration::from_secs(2)),
            target_object: "square".to_string(),
            property: "position".to_string(),
            to: Value::Tuple(100.0, 100.0),
            easing: Some("ease_in_out".to_string()),
        };
        
        assert_eq!(animation.start, Duration::from_secs(0));
        assert_eq!(animation.end, Some(Duration::from_secs(2)));
        assert_eq!(animation.target_object, "square");
        assert_eq!(animation.property, "position");
        assert_eq!(animation.to, Value::Tuple(100.0, 100.0));
        assert_eq!(animation.easing, Some("ease_in_out".to_string()));
    }

    #[test] 
    fn test_timeline_construction() {
        let timeline = Timeline {
            animations: vec![
                Animation {
                    start: Duration::from_secs(0),
                    end: None,
                    target_object: "obj1".to_string(),
                    property: "color".to_string(),
                    to: Value::Color("#FF0000".to_string()),
                    easing: None,
                }
            ],
        };
        
        assert_eq!(timeline.animations.len(), 1);
        assert_eq!(timeline.animations[0].target_object, "obj1");
    }

    #[test]
    fn test_property_construction() {
        let property = Property {
            name: "fill".to_string(),
            value: Value::Color("#00FF00".to_string()),
        };
        
        assert_eq!(property.name, "fill");
        match property.value {
            Value::Color(color) => assert_eq!(color, "#00FF00"),
            _ => panic!("Expected color value"),
        }
    }

    #[test]
    fn test_beam_script_with_complex_structure() {
        let script = BeamScript {
            camera: Some(Camera {
                properties: vec![
                    Property {
                        name: "width".to_string(),
                        value: Value::Number(1920.0),
                    }
                ],
            }),
            scenes: vec![
                Scene {
                    name: "Scene1".to_string(),
                    items: vec![
                        Object {
                            r#type: "square".to_string(),
                            name: "square1".to_string(),
                            properties: vec![
                                Property {
                                    name: "size".to_string(),
                                    value: Value::Number(100.0),
                                }
                            ],
                        }
                    ],
                    timeline: Some(Timeline {
                        animations: vec![
                            Animation {
                                start: Duration::from_secs(0),
                                end: Some(Duration::from_secs(1)),
                                target_object: "square1".to_string(),
                                property: "size".to_string(),
                                to: Value::Number(200.0),
                                easing: Some("linear".to_string()),
                            }
                        ],
                    }),
                    duration: Some(Duration::from_secs(2)),
                }
            ],
        };
        
        assert!(script.camera.is_some());
        assert_eq!(script.scenes.len(), 1);
        assert!(script.scenes[0].timeline.is_some());
    }
} 