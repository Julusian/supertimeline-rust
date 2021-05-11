extern crate supertimeline;
mod objs;
mod util;

use crate::objs::SimpleKeyframe;
use crate::objs::SimpleTimelineObj;
use crate::util::assert_instances;
use crate::util::assert_obj_on_layer;
use std::collections::HashSet;
use std::rc::Rc;
use supertimeline::get_state;
use supertimeline::TimelineObjectInstance;
use supertimeline::{
    resolve_all_states, resolve_timeline, EventType, Expression, IsTimelineObject, NextEvent,
    ResolveOptions, TimelineEnable,
};

#[test]
fn simple_timeline() {
    let timeline: Vec<Box<dyn IsTimelineObject>> = vec![
        Box::new(SimpleTimelineObj {
            id: "video".to_string(),
            layer: "0".to_string(),
            enable: vec![TimelineEnable {
                enable_start: Some(Expression::Number(0)),
                enable_end: Some(Expression::Number(100)),
                ..Default::default()
            }],
            ..Default::default()
        }),
        Box::new(SimpleTimelineObj {
            id: "graphic0".to_string(),
            layer: "1".to_string(),
            enable: vec![TimelineEnable {
                enable_start: Some(Expression::String("#video.start + 10".to_string())), // 10
                duration: Some(Expression::Number(10)),
                ..Default::default()
            }],
            ..Default::default()
        }),
        Box::new(SimpleTimelineObj {
            id: "graphic1".to_string(),
            layer: "1".to_string(),
            enable: vec![TimelineEnable {
                enable_start: Some(Expression::String("#graphic0.end + 10".to_string())), // 30
                duration: Some(Expression::Number(15)),
                ..Default::default()
            }],
            ..Default::default()
        }),
    ];

    let options = ResolveOptions {
        time: 0,
        limit_count: None,
        limit_time: None,
    };

    let resolved = resolve_timeline(&timeline, options).expect("Resolve timeline failed");
    let states = resolve_all_states(&resolved, None).expect("Resolve states failed");

    assert_eq!(
        &states.next_events,
        &vec![
            NextEvent {
                event_type: EventType::Start,
                object_id: "graphic0".to_string(),
                time: 10,
            },
            NextEvent {
                event_type: EventType::End,
                object_id: "graphic0".to_string(),
                time: 20,
            },
            NextEvent {
                event_type: EventType::Start,
                object_id: "graphic1".to_string(),
                time: 30,
            },
            NextEvent {
                event_type: EventType::End,
                object_id: "graphic1".to_string(),
                time: 45,
            },
            NextEvent {
                event_type: EventType::End,
                object_id: "video".to_string(),
                time: 100,
            },
        ],
    );

    let obj_video = states.objects.get("video").expect("Missing video object");
    let obj_graphics0 = states
        .objects
        .get("graphic0")
        .expect("Missing graphic0 object");
    let obj_graphics1 = states
        .objects
        .get("graphic1")
        .expect("Missing graphic1 object");

    // expect(resolved.statistics.resolvedObjectCount).toEqual(3)
    // expect(resolved.statistics.unresolvedCount).toEqual(0)

    assert_instances(
        &obj_video.instances,
        &vec![Rc::new(TimelineObjectInstance {
            start: 0,
            end: Some(100),
            ..Default::default()
        })],
    );
    assert_instances(
        &obj_graphics0.instances,
        &vec![Rc::new(TimelineObjectInstance {
            start: 10,
            end: Some(20),
            ..Default::default()
        })],
    );
    assert_instances(
        &obj_graphics1.instances,
        &vec![Rc::new(TimelineObjectInstance {
            start: 30,
            end: Some(45),
            ..Default::default()
        })],
    );

    {
        let state0 = get_state(&states, 5, None);
        assert_eq!(state0.time, 5);
        assert_obj_on_layer(&state0, "0", "video");
        assert!(state0.layers.get("1").is_none());
    }

    {
        let state0 = get_state(&states, 15, None);
        assert_obj_on_layer(&state0, "0", "video");
        assert_obj_on_layer(&state0, "1", "graphic0");
        assert_eq!(
            &state0.next_events,
            &vec![
                NextEvent {
                    event_type: EventType::End,
                    object_id: "graphic0".to_string(),
                    time: 20,
                },
                NextEvent {
                    event_type: EventType::Start,
                    object_id: "graphic1".to_string(),
                    time: 30,
                },
                NextEvent {
                    event_type: EventType::End,
                    object_id: "graphic1".to_string(),
                    time: 45,
                },
                NextEvent {
                    event_type: EventType::End,
                    object_id: "video".to_string(),
                    time: 100,
                },
            ],
        );
    }

    {
        let state0 = get_state(&states, 21, None);
        assert_obj_on_layer(&state0, "0", "video");
        assert!(state0.layers.get("1").is_none());
    }

    {
        let state0 = get_state(&states, 31, None);
        assert_obj_on_layer(&state0, "0", "video");
        assert_obj_on_layer(&state0, "1", "graphic1");
    }

    {
        let state0 = get_state(&states, 46, None);
        assert_obj_on_layer(&state0, "0", "video");
        assert!(state0.layers.get("1").is_none());
    }
}

#[test]
fn repeating_object() {
    let timeline: Vec<Box<dyn IsTimelineObject>> = vec![
        Box::new(SimpleTimelineObj {
            id: "video".to_string(),
            layer: "0".to_string(),
            enable: vec![TimelineEnable {
                enable_start: Some(Expression::Number(0)),
                enable_end: Some(Expression::Number(40)),
                repeating: Some(Expression::Number(50)),
                ..Default::default()
            }],
            ..Default::default()
        }),
        Box::new(SimpleTimelineObj {
            id: "graphic0".to_string(),
            layer: "1".to_string(),
            enable: vec![TimelineEnable {
                enable_start: Some(Expression::String("#video.start + 20".to_string())), // 20
                duration: Some(Expression::Number(19)),                                  // 39
                ..Default::default()
            }],
            ..Default::default()
        }),
    ];

    let options = ResolveOptions {
        time: 0,
        limit_count: Some(99),
        limit_time: Some(145),
    };

    let resolved = resolve_timeline(&timeline, options).expect("Resolve timeline failed");
    let states = resolve_all_states(&resolved, None).expect("Resolve states failed");

    // assert_eq!(
    //     &states.next_events,
    //     &vec![
    //         NextEvent {
    //             event_type: EventType::Start,
    //             object_id: "graphic0".to_string(),
    //             time: 10,
    //         },
    //         NextEvent {
    //             event_type: EventType::End,
    //             object_id: "graphic0".to_string(),
    //             time: 20,
    //         },
    //         NextEvent {
    //             event_type: EventType::Start,
    //             object_id: "graphic1".to_string(),
    //             time: 30,
    //         },
    //         NextEvent {
    //             event_type: EventType::End,
    //             object_id: "graphic1".to_string(),
    //             time: 45,
    //         },
    //         NextEvent {
    //             event_type: EventType::End,
    //             object_id: "video".to_string(),
    //             time: 100,
    //         },
    //     ],
    // );

    // expect(resolved.statistics.resolvedObjectCount).toEqual(2)
    // expect(resolved.statistics.unresolvedCount).toEqual(0)

    let obj_video = states.objects.get("video").expect("Missing video object");
    let obj_graphics0 = states
        .objects
        .get("graphic0")
        .expect("Missing graphic0 object");

    assert_instances(
        &obj_video.instances,
        &vec![
            Rc::new(TimelineObjectInstance {
                start: 0,
                end: Some(40),
                ..Default::default()
            }),
            Rc::new(TimelineObjectInstance {
                start: 50,
                end: Some(90),
                ..Default::default()
            }),
            Rc::new(TimelineObjectInstance {
                start: 100,
                end: Some(140),
                ..Default::default()
            }),
        ],
    );
    assert_instances(
        &obj_graphics0.instances,
        &vec![
            Rc::new(TimelineObjectInstance {
                start: 20,
                end: Some(39),
                ..Default::default()
            }),
            Rc::new(TimelineObjectInstance {
                start: 70,
                end: Some(89),
                ..Default::default()
            }),
            Rc::new(TimelineObjectInstance {
                start: 120,
                end: Some(139),
                ..Default::default()
            }),
        ],
    );

    {
        let state0 = get_state(&states, 15, None);
        assert_eq!(state0.time, 15);
        assert!(state0.layers.get("1").is_none());
        assert_obj_on_layer(&state0, "0", "video");
        assert_eq!(
            &state0.next_events,
            &vec![
                NextEvent {
                    event_type: EventType::Start,
                    object_id: "graphic0".to_string(),
                    time: 20,
                },
                NextEvent {
                    event_type: EventType::End,
                    object_id: "graphic0".to_string(),
                    time: 39,
                },
                NextEvent {
                    event_type: EventType::End,
                    object_id: "video".to_string(),
                    time: 40,
                },
                // next repeat:
                NextEvent {
                    event_type: EventType::Start,
                    object_id: "video".to_string(),
                    time: 50,
                },
                NextEvent {
                    time: 70,
                    event_type: EventType::Start,
                    object_id: "graphic0".to_string(),
                },
                NextEvent {
                    time: 89,
                    event_type: EventType::End,
                    object_id: "graphic0".to_string(),
                },
                NextEvent {
                    time: 90,
                    event_type: EventType::End,
                    object_id: "video".to_string(),
                },
                NextEvent {
                    time: 100,
                    event_type: EventType::Start,
                    object_id: "video".to_string(),
                },
                NextEvent {
                    time: 120,
                    event_type: EventType::Start,
                    object_id: "graphic0".to_string(),
                },
                NextEvent {
                    time: 139,
                    event_type: EventType::End,
                    object_id: "graphic0".to_string(),
                },
                NextEvent {
                    time: 140,
                    event_type: EventType::End,
                    object_id: "video".to_string(),
                }
            ],
        );
    }

    {
        let state0 = get_state(&states, 21, None);
        assert_obj_on_layer(&state0, "0", "video");
        assert_obj_on_layer(&state0, "1", "graphic0");
    }

    {
        let state0 = get_state(&states, 39, None);
        assert_obj_on_layer(&state0, "0", "video");
        assert!(state0.layers.get("1").is_none());
    }

    {
        let state0 = get_state(&states, 51, None);
        assert_obj_on_layer(&state0, "0", "video");
    }

    {
        let state0 = get_state(&states, 72, None);
        assert_obj_on_layer(&state0, "0", "video");
        assert_obj_on_layer(&state0, "1", "graphic0");
    }
}

#[test]
fn classes() {
    let timeline: Vec<Box<dyn IsTimelineObject>> = vec![
        Box::new(SimpleTimelineObj {
            id: "video0".to_string(),
            layer: "0".to_string(),
            enable: vec![TimelineEnable {
                enable_start: Some(Expression::Number(0)),
                enable_end: Some(Expression::Number(10)),
                repeating: Some(Expression::Number(50)),
                ..Default::default()
            }],
            classes: vec!["class0".to_string()],
            ..Default::default()
        }),
        Box::new(SimpleTimelineObj {
            id: "video1".to_string(),
            layer: "0".to_string(),
            enable: vec![TimelineEnable {
                enable_start: Some(Expression::String("#video0.end + 15".to_string())), // 25
                duration: Some(Expression::Number(10)),
                repeating: Some(Expression::Number(50)),
                ..Default::default()
            }],
            classes: vec!["class0".to_string(), "class1".to_string()],
            ..Default::default()
        }),
        Box::new(SimpleTimelineObj {
            id: "graphic0".to_string(),
            layer: "1".to_string(),
            enable: vec![TimelineEnable {
                enable_while: Some(Expression::String(".class0".to_string())),
                ..Default::default()
            }],
            ..Default::default()
        }),
        Box::new(SimpleTimelineObj {
            id: "graphic1".to_string(),
            layer: "2".to_string(),
            enable: vec![TimelineEnable {
                enable_while: Some(Expression::String(".class1 + 1".to_string())),
                ..Default::default()
            }],
            ..Default::default()
        }),
    ];

    let options = ResolveOptions {
        time: 0,
        limit_count: None,
        limit_time: Some(100),
    };

    let resolved = resolve_timeline(&timeline, options).expect("Resolve timeline failed");
    let states = resolve_all_states(&resolved, None).expect("Resolve states failed");

    // // expect(resolved.statistics.resolvedObjectCount).toEqual(2)
    // // expect(resolved.statistics.unresolvedCount).toEqual(0)

    let obj_video0 = states.objects.get("video0").expect("Missing video0 object");
    let obj_video1 = states.objects.get("video1").expect("Missing video1 object");
    let obj_graphics0 = states
        .objects
        .get("graphic0")
        .expect("Missing graphic0 object");
    let obj_graphics1 = states
        .objects
        .get("graphic1")
        .expect("Missing graphic1 object");

    assert_instances(
        &obj_video0.instances,
        &vec![
            Rc::new(TimelineObjectInstance {
                start: 0,
                end: Some(10),
                ..Default::default()
            }),
            Rc::new(TimelineObjectInstance {
                start: 50,
                end: Some(60),
                ..Default::default()
            }),
        ],
    );
    assert_instances(
        &obj_video1.instances,
        &vec![
            Rc::new(TimelineObjectInstance {
                start: 25,
                end: Some(35),
                ..Default::default()
            }),
            Rc::new(TimelineObjectInstance {
                start: 75,
                end: Some(85),
                ..Default::default()
            }),
        ],
    );
    assert_instances(
        &obj_graphics0.instances,
        &vec![
            Rc::new(TimelineObjectInstance {
                start: 0,
                end: Some(10),
                ..Default::default()
            }),
            Rc::new(TimelineObjectInstance {
                start: 25,
                end: Some(35),
                ..Default::default()
            }),
            Rc::new(TimelineObjectInstance {
                start: 50,
                end: Some(60),
                ..Default::default()
            }),
            Rc::new(TimelineObjectInstance {
                start: 75,
                end: Some(85),
                ..Default::default()
            }),
        ],
    );
    assert_instances(
        &obj_graphics1.instances,
        &vec![
            Rc::new(TimelineObjectInstance {
                start: 26,
                end: Some(36),
                ..Default::default()
            }),
            Rc::new(TimelineObjectInstance {
                start: 76,
                end: Some(86),
                ..Default::default()
            }),
        ],
    );

    {
        let state0 = get_state(&states, 5, None);
        assert_obj_on_layer(&state0, "0", "video0");
        assert_obj_on_layer(&state0, "1", "graphic0");
        assert!(state0.layers.get("2").is_none());
    }

    {
        let state0 = get_state(&states, 25, None);
        assert_obj_on_layer(&state0, "0", "video1");
        assert_obj_on_layer(&state0, "1", "graphic0");
        assert!(state0.layers.get("2").is_none());
    }

    {
        let state0 = get_state(&states, 26, None);
        assert_obj_on_layer(&state0, "0", "video1");
        assert_obj_on_layer(&state0, "1", "graphic0");
        assert_obj_on_layer(&state0, "2", "graphic1");
    }

    {
        let state0 = get_state(&states, 76, None);
        assert_obj_on_layer(&state0, "0", "video1");
        assert_obj_on_layer(&state0, "1", "graphic0");
        assert_obj_on_layer(&state0, "2", "graphic1");
    }
}

#[test]
fn unique_instance_ids() {
    let timeline: Vec<Box<dyn IsTimelineObject>> = vec![
        Box::new(SimpleTimelineObj {
            id: "video0".to_string(),
            layer: "0".to_string(),
            enable: vec![TimelineEnable {
                enable_start: Some(Expression::Number(10)),
                enable_end: Some(Expression::Number(80)),
                ..Default::default()
            }],
            ..Default::default()
        }),
        Box::new(SimpleTimelineObj {
            id: "video1".to_string(),
            layer: "0".to_string(),
            enable: vec![TimelineEnable {
                enable_start: Some(Expression::Number(10)),
                duration: Some(Expression::Number(20)),
                ..Default::default()
            }],
            priority: 1,
            ..Default::default()
        }),
    ];

    let options = ResolveOptions {
        time: 0,
        limit_count: None,
        limit_time: None,
    };

    let resolved = resolve_timeline(&timeline, options).expect("Resolve timeline failed");
    let states = resolve_all_states(&resolved, None).expect("Resolve states failed");

    states.objects.get("video0").expect("Missing video0 object");
    states.objects.get("video1").expect("Missing video1 object");

    // expect(resolved.statistics.resolvedObjectCount).toEqual(2)
    // expect(resolved.statistics.unresolvedCount).toEqual(0)

    let mut instance_ids = HashSet::new();
    let mut instance_count = 0;

    for obj in states.objects {
        for (id, instance) in obj.1.instances {
            let locked = instance.lock().unwrap();
            assert_eq!(locked.id, id);
            instance_ids.insert(id);
            instance_count += 1;
        }
    }

    assert_eq!(instance_count, 3);
    assert_eq!(instance_ids.len(), 3);
}

#[test]
fn repeating_many() {
    let timeline: Vec<Box<dyn IsTimelineObject>> = vec![Box::new(SimpleTimelineObj {
        id: "video0".to_string(),
        layer: "0".to_string(),
        enable: vec![TimelineEnable {
            enable_start: Some(Expression::Number(0)),
            enable_end: Some(Expression::Number(8)),
            repeating: Some(Expression::Number(10)),
            ..Default::default()
        }],
        ..Default::default()
    })];

    let options = ResolveOptions {
        time: 0,
        limit_count: Some(100),
        limit_time: Some(99999),
    };

    let resolved = resolve_timeline(&timeline, options).expect("Resolve timeline failed");
    let states = resolve_all_states(&resolved, None).expect("Resolve states failed");

    let obj_video0 = states.objects.get("video0").expect("Missing video0 object");
    assert_eq!(obj_video0.instances.len(), 100);
}

#[test]
fn class_not_defined() {
    let timeline: Vec<Box<dyn IsTimelineObject>> = vec![Box::new(SimpleTimelineObj {
        id: "video0".to_string(),
        layer: "0".to_string(),
        enable: vec![TimelineEnable {
            enable_while: Some(Expression::String("!.class0".to_string())),
            ..Default::default()
        }],
        ..Default::default()
    })];

    let options = ResolveOptions {
        time: 0,
        limit_count: Some(10),
        limit_time: Some(999),
    };

    let resolved = resolve_timeline(&timeline, options).expect("Resolve timeline failed");
    let states = resolve_all_states(&resolved, None).expect("Resolve states failed");

    let obj_video0 = states.objects.get("video0").expect("Missing video0 object");
    assert_eq!(obj_video0.instances.len(), 1);

    let state0 = get_state(&states, 10, Some(10));
    assert_obj_on_layer(&state0, "0", "video0");
}

#[test]
fn reference_duration() {
    let timeline: Vec<Box<dyn IsTimelineObject>> = vec![
        Box::new(SimpleTimelineObj {
            id: "video0".to_string(),
            layer: "0".to_string(),
            enable: vec![TimelineEnable {
                enable_start: Some(Expression::Number(10)),
                enable_end: Some(Expression::Number(100)),
                ..Default::default()
            }],
            ..Default::default()
        }),
        Box::new(SimpleTimelineObj {
            id: "video1".to_string(),
            layer: "1".to_string(),
            enable: vec![TimelineEnable {
                enable_start: Some(Expression::Number(20)),
                enable_end: Some(Expression::String("#video0".to_string())),
                ..Default::default()
            }],
            ..Default::default()
        }),
    ];

    let options = ResolveOptions {
        time: 0,
        limit_count: Some(10),
        limit_time: Some(999),
    };

    let resolved = resolve_timeline(&timeline, options).expect("Resolve timeline failed");
    let states = resolve_all_states(&resolved, None).expect("Resolve states failed");

    let obj_video0 = states.objects.get("video0").expect("Missing video0 object");
    let obj_video1 = states.objects.get("video1").expect("Missing video1 object");
    assert_eq!(obj_video0.instances.len(), 1);
    assert_eq!(obj_video1.instances.len(), 1);

    assert_instances(
        &obj_video1.instances,
        &vec![Rc::new(TimelineObjectInstance {
            start: 20,
            end: Some(100),
            ..Default::default()
        })],
    );
}

#[test]
fn reference_own_layer() {
    let mut timeline: Vec<Box<dyn IsTimelineObject>> = vec![
        Box::new(SimpleTimelineObj {
            id: "video0".to_string(),
            layer: "0".to_string(),
            enable: vec![TimelineEnable {
                enable_start: Some(Expression::Number(0)),
                enable_end: Some(Expression::Number(8)),
                ..Default::default()
            }],
            ..Default::default()
        }),
        Box::new(SimpleTimelineObj {
            id: "video1".to_string(),
            layer: "0".to_string(),
            enable: vec![TimelineEnable {
                // Play for 2 after each other object on layer 0
                enable_start: Some(Expression::String("$0.end".to_string())),
                duration: Some(Expression::Number(2)),
                ..Default::default()
            }],
            ..Default::default()
        }),
        Box::new(SimpleTimelineObj {
            id: "video2".to_string(),
            layer: "0".to_string(),
            enable: vec![TimelineEnable {
                // Play for 2 after each other object on layer 0
                enable_start: Some(Expression::String("$0.end + 1".to_string())),
                duration: Some(Expression::Number(2)),
                ..Default::default()
            }],
            ..Default::default()
        }),
    ];

    for _ in 0..2 {
        timeline.reverse();
        assert_eq!(timeline.len(), 3);

        let options = ResolveOptions {
            time: 0,
            limit_count: Some(100),
            limit_time: Some(99999),
        };

        let resolved = resolve_timeline(&timeline, options).expect("Resolve timeline failed");
        let states = resolve_all_states(&resolved, None).expect("Resolve states failed");

        let obj_video0 = states.objects.get("video0").expect("Missing video0 object");
        let obj_video1 = states.objects.get("video1").expect("Missing video1 object");
        let obj_video2 = states.objects.get("video2").expect("Missing video2 object");

        assert_instances(
            &obj_video0.instances,
            &vec![Rc::new(TimelineObjectInstance {
                start: 0,
                end: Some(8),
                ..Default::default()
            })],
        );
        assert_instances(
            &obj_video1.instances,
            &vec![Rc::new(TimelineObjectInstance {
                start: 8,
                end: Some(9), // becuse it's overridden by video2
                original_end: Some(10),
                ..Default::default()
            })],
        );
        assert_instances(
            &obj_video2.instances,
            &vec![Rc::new(TimelineObjectInstance {
                start: 9,
                end: Some(11),
                ..Default::default()
            })],
        );
    }
}

#[test]
fn reference_own_class() {
    let mut timeline: Vec<Box<dyn IsTimelineObject>> = vec![
        Box::new(SimpleTimelineObj {
            id: "video0".to_string(),
            layer: "0".to_string(),
            enable: vec![TimelineEnable {
                enable_start: Some(Expression::Number(0)),
                duration: Some(Expression::Number(8)),
                ..Default::default()
            }],
            classes: vec!["insert_after".to_string()],
            ..Default::default()
        }),
        Box::new(SimpleTimelineObj {
            id: "video1".to_string(),
            layer: "1".to_string(),
            enable: vec![TimelineEnable {
                // Play for 2 after each other object with class 'insert_after'
                enable_start: Some(Expression::String(".insert_after.end".to_string())),
                duration: Some(Expression::Number(2)),
                ..Default::default()
            }],
            classes: vec!["insert_after".to_string()],
            ..Default::default()
        }),
        Box::new(SimpleTimelineObj {
            id: "video2".to_string(),
            layer: "1".to_string(),
            enable: vec![TimelineEnable {
                // Play for 2 after each other object on layer 0
                enable_start: Some(Expression::String(".insert_after.end + 1".to_string())),
                duration: Some(Expression::Number(2)),
                ..Default::default()
            }],
            classes: vec!["insert_after".to_string()],
            ..Default::default()
        }),
    ];

    for _ in 0..2 {
        timeline.reverse();
        assert_eq!(timeline.len(), 3);

        let options = ResolveOptions {
            time: 0,
            limit_count: Some(100),
            limit_time: Some(99999),
        };

        let resolved = resolve_timeline(&timeline, options).expect("Resolve timeline failed");
        let states = resolve_all_states(&resolved, None).expect("Resolve states failed");

        let obj_video0 = states.objects.get("video0").expect("Missing video0 object");
        let obj_video1 = states.objects.get("video1").expect("Missing video1 object");
        let obj_video2 = states.objects.get("video2").expect("Missing video2 object");

        assert_instances(
            &obj_video0.instances,
            &vec![Rc::new(TimelineObjectInstance {
                start: 0,
                end: Some(8),
                ..Default::default()
            })],
        );
        assert_instances(
            &obj_video1.instances,
            &vec![Rc::new(TimelineObjectInstance {
                start: 8,
                end: Some(9), // becuse it's overridden by video2
                original_end: Some(10),
                ..Default::default()
            })],
        );
        assert_instances(
            &obj_video2.instances,
            &vec![Rc::new(TimelineObjectInstance {
                start: 9,
                end: Some(11),
                ..Default::default()
            })],
        );
    }
}

#[test]
fn continuous_combined_negated_and_normal_classes_on_different_objects() {
    let timeline: Vec<Box<dyn IsTimelineObject>> = vec![
        Box::new(SimpleTimelineObj {
            id: "parent".to_string(),
            layer: "p0".to_string(),
            priority: 0,
            enable: vec![TimelineEnable {
                enable_while: Some(Expression::Number(1)),
                ..Default::default()
            }],
            keyframes: vec![Box::new(SimpleKeyframe {
                id: "kf0".to_string(),
                enable: vec![TimelineEnable {
                    enable_while: Some(Expression::String(".playout & !.muted".to_string())),
                    ..Default::default()
                }],
                ..Default::default()
            })],
            ..Default::default()
        }),
        Box::new(SimpleTimelineObj {
            id: "muted_playout1".to_string(),
            layer: "2".to_string(),
            priority: 0,
            enable: vec![TimelineEnable {
                enable_start: Some(Expression::String("100".to_string())),
                duration: Some(Expression::Number(100)),
                ..Default::default()
            }],
            classes: vec!["playout".to_string(), "muted".to_string()],
            ..Default::default()
        }),
        Box::new(SimpleTimelineObj {
            id: "muted_playout2".to_string(),
            layer: "2".to_string(),
            priority: 0,
            enable: vec![TimelineEnable {
                enable_start: Some(Expression::String("200".to_string())),
                duration: Some(Expression::Number(100)),
                ..Default::default()
            }],
            classes: vec!["playout".to_string(), "muted".to_string()],
            ..Default::default()
        }),
        Box::new(SimpleTimelineObj {
            id: "unmuted_playout1".to_string(),
            layer: "2".to_string(),
            priority: 0,
            enable: vec![TimelineEnable {
                enable_start: Some(Expression::String("300".to_string())),
                duration: Some(Expression::Number(100)),
                ..Default::default()
            }],
            classes: vec!["playout".to_string()],
            ..Default::default()
        }),
    ];

    let options = ResolveOptions {
        time: 0,
        limit_count: Some(10),
        limit_time: Some(999),
    };

    let resolved = resolve_timeline(&timeline, options).expect("Resolve timeline failed");
    let states = resolve_all_states(&resolved, None).expect("Resolve states failed");

    states.objects.get("parent").expect("Missing parent object");
    states
        .objects
        .get("muted_playout1")
        .expect("Missing muted_playout1 object");
    states
        .objects
        .get("muted_playout2")
        .expect("Missing muted_playout2 object");
    states
        .objects
        .get("unmuted_playout1")
        .expect("Missing unmuted_playout1 object");

    {
        let state0 = get_state(&states, 50, None);
        let layer = state0.layers.get("p0").expect("Missing state for layer p0");
        assert_eq!(layer.object_id, "parent");
        assert_eq!(layer.keyframes.len(), 0);
    }

    {
        let state0 = get_state(&states, 150, None);
        let layer = state0.layers.get("p0").expect("Missing state for layer p0");
        assert_eq!(layer.object_id, "parent");
        assert_eq!(layer.keyframes.len(), 0);
    }

    {
        let state0 = get_state(&states, 250, None);
        let layer = state0.layers.get("p0").expect("Missing state for layer p0");
        assert_eq!(layer.object_id, "parent");
        assert_eq!(layer.keyframes.len(), 0);
    }

    {
        let state0 = get_state(&states, 350, None);
        let layer = state0.layers.get("p0").expect("Missing state for layer p0");
        assert_eq!(layer.object_id, "parent");
        assert_eq!(layer.keyframes.len(), 1);
    }
}
