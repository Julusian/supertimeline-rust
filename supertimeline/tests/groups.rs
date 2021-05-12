extern crate supertimeline;
mod objs;
mod util;

use crate::objs::SimpleTimelineObj;
use crate::objs::TimelineObjectInstanceLight;
use crate::util::assert_obj_on_layer;
use crate::util::{assert_instances, assert_instances2};
use std::rc::Rc;
use supertimeline::get_state;
use supertimeline::NextEvent;
use supertimeline::TimelineObjectInstance;
use supertimeline::{
    resolve_all_states, resolve_timeline, EventType, Expression, ResolveOptions,
    TimelineEnable,
};

#[test]
fn simple_group() {
    let timeline: Vec<SimpleTimelineObj> = vec![
        (SimpleTimelineObj {
            id: "group".to_string(),
            layer: "0".to_string(),
            enable: vec![TimelineEnable {
                enable_start: Some(Expression::Number(10)),
                enable_end: Some(Expression::Number(100)),
                ..Default::default()
            }],
            children: Some(vec![
                (SimpleTimelineObj {
                    id: "child0".to_string(),
                    layer: "1".to_string(),
                    enable: vec![TimelineEnable {
                        enable_start: Some(Expression::String("5".to_string())), // 15
                        duration: Some(Expression::Number(10)),                  // 25
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
                (SimpleTimelineObj {
                    id: "child1".to_string(),
                    layer: "1".to_string(),
                    enable: vec![TimelineEnable {
                        enable_start: Some(Expression::String("#child0.end".to_string())), // 25
                        duration: Some(Expression::Number(10)),                            // 35
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
                (SimpleTimelineObj {
                    id: "child2".to_string(),
                    layer: "2".to_string(),
                    enable: vec![TimelineEnable {
                        enable_start: Some(Expression::String("-1".to_string())), // 9, will be capped in parent
                        duration: Some(Expression::Number(150)), // 160, will be capped in parent
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
            ]),
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

    states.objects.get("group").expect("Missing group object");
    let obj_child0 = states.objects.get("child0").expect("Missing child0 object");
    let obj_child1 = states.objects.get("child1").expect("Missing child1 object");
    let obj_child2 = states.objects.get("child2").expect("Missing child2 object");

    // expect(resolved.statistics.resolvedObjectCount).toEqual(3)
    // expect(resolved.statistics.unresolvedCount).toEqual(0)

    assert_instances(
        &obj_child0.instances,
        &vec![Rc::new(TimelineObjectInstance {
            start: 15,
            end: Some(25),
            ..Default::default()
        })],
    );
    assert_instances(
        &obj_child1.instances,
        &vec![Rc::new(TimelineObjectInstance {
            start: 25,
            end: Some(35),
            ..Default::default()
        })],
    );
    assert_instances(
        &obj_child2.instances,
        &vec![Rc::new(TimelineObjectInstance {
            start: 10,
            end: Some(100),
            ..Default::default()
        })],
    );

    {
        let state0 = get_state(&states, 11, None);
        assert_obj_on_layer(&state0, "0", "group");
        assert!(state0.layers.get("1").is_none());
        assert_obj_on_layer(&state0, "2", "child2");
    }

    {
        let state0 = get_state(&states, 15, None);
        assert_obj_on_layer(&state0, "0", "group");
        assert_obj_on_layer(&state0, "1", "child0");
        assert_obj_on_layer(&state0, "2", "child2");
    }

    {
        let state0 = get_state(&states, 30, None);
        assert_obj_on_layer(&state0, "0", "group");
        assert_obj_on_layer(&state0, "1", "child1");
        assert_obj_on_layer(&state0, "2", "child2");
    }
}

#[test]
fn etheral_groups() {
    let timeline: Vec<SimpleTimelineObj> = vec![
        (SimpleTimelineObj {
            id: "group0".to_string(),
            layer: "".to_string(),
            enable: vec![TimelineEnable {
                enable_start: Some(Expression::Number(10)),
                enable_end: Some(Expression::Number(100)),
                ..Default::default()
            }],
            children: Some(vec![
                (SimpleTimelineObj {
                    id: "child0".to_string(),
                    layer: "1".to_string(),
                    enable: vec![TimelineEnable {
                        enable_start: Some(Expression::String("5".to_string())), // 15
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
            ]),
            ..Default::default()
        }),
        (SimpleTimelineObj {
            id: "group1".to_string(),
            layer: "".to_string(),
            enable: vec![TimelineEnable {
                enable_start: Some(Expression::Number(50)),
                enable_end: Some(Expression::Number(100)),
                ..Default::default()
            }],
            children: Some(vec![
                (SimpleTimelineObj {
                    id: "child1".to_string(),
                    layer: "2".to_string(),
                    enable: vec![TimelineEnable {
                        enable_start: Some(Expression::String("5".to_string())), // 55
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
            ]),
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

    let obj_group0 = resolved
        .objects
        .get("group0")
        .expect("Missing group0 object");
    let obj_group1 = resolved
        .objects
        .get("group1")
        .expect("Missing group1 object");
    let obj_child0 = resolved
        .objects
        .get("child0")
        .expect("Missing child0 object");
    let obj_child1 = resolved
        .objects
        .get("child1")
        .expect("Missing child1 object");

    // expect(resolved.statistics.resolvedObjectCount).toEqual(3)
    // expect(resolved.statistics.unresolvedCount).toEqual(0)

    assert_instances2(
        &obj_group0.resolved.instances,
        &vec![Rc::new(TimelineObjectInstance {
            start: 10,
            end: Some(100),
            ..Default::default()
        })],
    );
    assert_instances2(
        &obj_child0.resolved.instances,
        &vec![Rc::new(TimelineObjectInstance {
            start: 15,
            end: Some(100),
            ..Default::default()
        })],
    );
    assert_instances2(
        &obj_group1.resolved.instances,
        &vec![Rc::new(TimelineObjectInstance {
            start: 50,
            end: Some(100),
            ..Default::default()
        })],
    );
    assert_instances2(
        &obj_child1.resolved.instances,
        &vec![Rc::new(TimelineObjectInstance {
            start: 55,
            end: Some(100),
            ..Default::default()
        })],
    );

    {
        let state0 = get_state(&states, 16, None);
        assert_obj_on_layer(&state0, "1", "child0");
        assert!(state0.layers.get("2").is_none());
    }

    {
        let state0 = get_state(&states, 56, None);
        assert_obj_on_layer(&state0, "1", "child0");
        assert_obj_on_layer(&state0, "2", "child1");
        assert_eq!(
            &state0.next_events,
            &vec![
                NextEvent {
                    event_type: EventType::End,
                    object_id: "child0".to_string(),
                    time: 100,
                },
                NextEvent {
                    event_type: EventType::End,
                    object_id: "child1".to_string(),
                    time: 100,
                },
            ],
        );
    }

    {
        // objects should be capped inside their parent:
        let state0 = get_state(&states, 120, None);
        assert!(state0.layers.get("1").is_none());
        assert!(state0.layers.get("2").is_none());
    }
}

#[test]
fn solid_groups() {
    // "solid groups" are groups with a layer
    let timeline: Vec<SimpleTimelineObj> = vec![
        (SimpleTimelineObj {
            id: "group0".to_string(),
            layer: "g0".to_string(),
            enable: vec![TimelineEnable {
                enable_start: Some(Expression::Number(10)),
                enable_end: Some(Expression::Number(100)),
                ..Default::default()
            }],
            children: Some(vec![
                (SimpleTimelineObj {
                    id: "child0".to_string(),
                    layer: "1".to_string(),
                    enable: vec![TimelineEnable {
                        enable_start: Some(Expression::String("5".to_string())), // 15
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
            ]),
            ..Default::default()
        }),
        (SimpleTimelineObj {
            id: "group1".to_string(),
            layer: "g0".to_string(),
            enable: vec![TimelineEnable {
                enable_start: Some(Expression::Number(50)),
                enable_end: Some(Expression::Number(100)),
                ..Default::default()
            }],
            children: Some(vec![
                (SimpleTimelineObj {
                    id: "child1".to_string(),
                    layer: "2".to_string(),
                    enable: vec![TimelineEnable {
                        enable_start: Some(Expression::String("5".to_string())), // 55
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
            ]),
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

    let obj_group0 = states.objects.get("group0").expect("Missing group0 object");
    let obj_group1 = states.objects.get("group1").expect("Missing group1 object");
    let obj_child0 = states.objects.get("child0").expect("Missing child0 object");
    let obj_child1 = states.objects.get("child1").expect("Missing child1 object");

    // expect(resolved.statistics.resolvedObjectCount).toEqual(3)
    // expect(resolved.statistics.unresolvedCount).toEqual(0)

    assert_instances(
        &obj_group0.instances,
        &vec![Rc::new(TimelineObjectInstance {
            start: 10,
            end: Some(50), // because group 1 started
            ..Default::default()
        })],
    );
    assert_instances(
        &obj_child0.instances,
        &vec![Rc::new(TimelineObjectInstance {
            start: 15,
            end: Some(100),
            ..Default::default()
        })],
    );
    assert_instances(
        &obj_group1.instances,
        &vec![Rc::new(TimelineObjectInstance {
            start: 50,
            end: Some(100),
            ..Default::default()
        })],
    );
    assert_instances(
        &obj_child1.instances,
        &vec![Rc::new(TimelineObjectInstance {
            start: 55,
            end: Some(100),
            ..Default::default()
        })],
    );

    {
        let state0 = get_state(&states, 16, None);
        assert_obj_on_layer(&state0, "g0", "group0");
        assert_obj_on_layer(&state0, "1", "child0");
        assert!(state0.layers.get("2").is_none());
        assert_eq!(
            &state0.next_events,
            &vec![
                NextEvent {
                    event_type: EventType::End,
                    object_id: "group0".to_string(),
                    time: 50,
                },
                NextEvent {
                    event_type: EventType::Start,
                    object_id: "group1".to_string(),
                    time: 50,
                },
                NextEvent {
                    event_type: EventType::Start,
                    object_id: "child1".to_string(),
                    time: 55,
                },
                NextEvent {
                    event_type: EventType::End,
                    object_id: "child0".to_string(),
                    time: 100,
                },
                NextEvent {
                    event_type: EventType::End,
                    object_id: "child1".to_string(),
                    time: 100,
                },
                NextEvent {
                    event_type: EventType::End,
                    object_id: "group1".to_string(),
                    time: 100,
                },
            ],
        );
    }

    {
        let state0 = get_state(&states, 56, None);
        assert_obj_on_layer(&state0, "g0", "group1");
        assert_obj_on_layer(&state0, "2", "child1");
        // assert!(state0.layers.get("1").is_none());
        assert_eq!(
            &state0.next_events,
            &vec![
                NextEvent {
                    event_type: EventType::End,
                    object_id: "child0".to_string(),
                    time: 100,
                },
                NextEvent {
                    event_type: EventType::End,
                    object_id: "child1".to_string(),
                    time: 100,
                },
                NextEvent {
                    event_type: EventType::End,
                    object_id: "group1".to_string(),
                    time: 100,
                },
            ],
        );
    }

    {
        // objects should be capped inside their parent:
        let state0 = get_state(&states, 120, None);
        assert!(state0.layers.get("g0").is_none());
        assert!(state0.layers.get("1").is_none());
        assert!(state0.layers.get("2").is_none());
    }
}

#[test]
fn cap_in_repeating_parent_group() {
    let timeline: Vec<SimpleTimelineObj> = vec![
        (SimpleTimelineObj {
            id: "group0".to_string(),
            layer: "g0".to_string(),
            enable: vec![TimelineEnable {
                enable_start: Some(Expression::Number(0)), // 0, 100
                enable_end: Some(Expression::Number(80)),  // 80, 180
                repeating: Some(Expression::Number(100)),
                ..Default::default()
            }],
            children: Some(vec![
                (SimpleTimelineObj {
                    id: "child0".to_string(),
                    layer: "1".to_string(),
                    enable: vec![TimelineEnable {
                        enable_start: Some(Expression::Number(50)), // 50, 150
                        duration: Some(Expression::Number(20)),     // 70, 170
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
                (SimpleTimelineObj {
                    id: "child1".to_string(),
                    layer: "2".to_string(),
                    enable: vec![TimelineEnable {
                        enable_start: Some(Expression::String("#child0.end".to_string())), // 70, 170
                        duration: Some(Expression::Number(50)), // 120 (to be capped at 100), 220 (to be capped at 200)
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
            ]),
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

    let obj_group0 = states.objects.get("group0").expect("Missing group0 object");
    let obj_child0 = states.objects.get("child0").expect("Missing child0 object");
    let obj_child1 = states.objects.get("child1").expect("Missing child1 object");

    // expect(resolved.statistics.resolvedObjectCount).toEqual(3)
    // expect(resolved.statistics.unresolvedCount).toEqual(0)

    assert_instances(
        &obj_group0.instances,
        &vec![
            Rc::new(TimelineObjectInstance {
                start: 0,
                end: Some(80),
                ..Default::default()
            }),
            Rc::new(TimelineObjectInstance {
                start: 100,
                end: Some(180),
                ..Default::default()
            }),
        ],
    );
    assert_instances(
        &obj_child0.instances,
        &vec![
            Rc::new(TimelineObjectInstance {
                start: 50,
                end: Some(70),
                ..Default::default()
            }),
            Rc::new(TimelineObjectInstance {
                start: 150,
                end: Some(170),
                ..Default::default()
            }),
        ],
    );
    assert_instances(
        &obj_child1.instances,
        &vec![
            Rc::new(TimelineObjectInstance {
                start: 70,
                end: Some(80),
                ..Default::default()
            }),
            Rc::new(TimelineObjectInstance {
                start: 170,
                end: Some(180),
                ..Default::default()
            }),
        ],
    );

    {
        let state0 = get_state(&states, 10, None);
        assert_obj_on_layer(&state0, "g0", "group0");
        assert!(state0.layers.get("1").is_none());
        assert!(state0.layers.get("2").is_none());
    }

    {
        let state0 = get_state(&states, 55, None);
        assert_obj_on_layer(&state0, "g0", "group0");
        assert_obj_on_layer(&state0, "1", "child0");
        assert!(state0.layers.get("2").is_none());
    }

    {
        let state0 = get_state(&states, 78, None);
        assert_obj_on_layer(&state0, "g0", "group0");
        assert!(state0.layers.get("1").is_none());
        assert_obj_on_layer(&state0, "2", "child1");
    }

    {
        let state0 = get_state(&states, 85, None);
        assert!(state0.layers.get("g0").is_none());
        assert!(state0.layers.get("1").is_none());
        assert!(state0.layers.get("2").is_none());
    }

    {
        let state0 = get_state(&states, 110, None);
        assert_obj_on_layer(&state0, "g0", "group0");
        assert!(state0.layers.get("1").is_none());
        assert!(state0.layers.get("2").is_none());
    }

    {
        let state0 = get_state(&states, 155, None);
        assert_obj_on_layer(&state0, "g0", "group0");
        assert_obj_on_layer(&state0, "1", "child0");
        assert!(state0.layers.get("2").is_none());
    }

    {
        let state0 = get_state(&states, 178, None);
        assert_obj_on_layer(&state0, "g0", "group0");
        assert!(state0.layers.get("1").is_none());
        assert_obj_on_layer(&state0, "2", "child1");
    }

    {
        let state0 = get_state(&states, 185, None);
        assert!(state0.layers.get("g0").is_none());
        assert!(state0.layers.get("1").is_none());
        assert!(state0.layers.get("2").is_none());
    }
}

#[test]
fn referencing_child_in_parent_group() {
    // This shouldn't change the outcome, since it's changing from a reference that resolves to { while: '1' }
    let timeline = |child0_while: &str| -> Vec<SimpleTimelineObj> {
        vec![
            (SimpleTimelineObj {
                id: "group0".to_string(),
                layer: "g0".to_string(),
                enable: vec![TimelineEnable {
                    enable_start: Some(Expression::Number(0)),
                    enable_end: Some(Expression::Number(80)),
                    ..Default::default()
                }],
                children: Some(vec![
                    (SimpleTimelineObj {
                        id: "child0".to_string(),
                        layer: "1".to_string(),
                        enable: vec![TimelineEnable {
                            enable_while: Some(Expression::String(child0_while.to_string())),
                            ..Default::default()
                        }],
                        ..Default::default()
                    }),
                ]),
                ..Default::default()
            }),
            (SimpleTimelineObj {
                id: "other".to_string(),
                layer: "other".to_string(),
                enable: vec![TimelineEnable {
                    enable_while: Some(Expression::String("1".to_string())),
                    ..Default::default()
                }],
                ..Default::default()
            }),
            (SimpleTimelineObj {
                id: "refChild0".to_string(),
                layer: "42".to_string(),
                enable: vec![TimelineEnable {
                    enable_while: Some(Expression::String("#child0".to_string())),
                    ..Default::default()
                }],
                ..Default::default()
            }),
        ]
    };

    let options = ResolveOptions {
        time: 0,
        limit_count: Some(99),
        limit_time: Some(199),
    };

    // Try with the reference enable
    let resolved0 =
        resolve_timeline(&timeline("#other"), options.clone()).expect("Resolve timeline failed");
    let states0 = resolve_all_states(&resolved0, None).expect("Resolve states failed");

    // Try with a 'always' enable
    let resolved1 = resolve_timeline(&timeline("1"), options).expect("Resolve timeline failed");
    let states1 = resolve_all_states(&resolved1, None).expect("Resolve states failed");

    states0.layers.get("other").expect("Missing other layer");
    states1.layers.get("other").expect("Missing other layer");
    states0.layers.get("42").expect("Missing 42 layer");
    states1.layers.get("42").expect("Missing 42 layer");

    let obj0_child0 = resolved0
        .objects
        .get("refChild0")
        .expect("Missing refChild0 object");
    let obj1_child0 = resolved1
        .objects
        .get("refChild0")
        .expect("Missing refChild0 object");

    let obj0_instances: Vec<TimelineObjectInstanceLight> = obj0_child0
        .resolved
        .instances
        .iter()
        .map(|i| TimelineObjectInstanceLight::from(i))
        .collect();
    let obj1_instances: Vec<TimelineObjectInstanceLight> = obj1_child0
        .resolved
        .instances
        .iter()
        .map(|i| TimelineObjectInstanceLight::from(i))
        .collect();

    assert_eq!(obj0_instances, obj1_instances);
}

#[test]
fn content_start_time_in_capped_object() {
    let timeline: Vec<SimpleTimelineObj> = vec![
        (SimpleTimelineObj {
            id: "extRef".to_string(),
            layer: "e0".to_string(),
            enable: vec![TimelineEnable {
                enable_start: Some(Expression::Number(10)),
                duration: Some(Expression::Number(20)),
                ..Default::default()
            }],
            ..Default::default()
        }),
        (SimpleTimelineObj {
            id: "myGroup".to_string(),
            layer: "g0".to_string(),
            enable: vec![TimelineEnable {
                enable_start: Some(Expression::Number(50)),
                enable_end: Some(Expression::Number(100)),
                ..Default::default()
            }],
            children: Some(vec![
                (SimpleTimelineObj {
                    id: "video".to_string(),
                    layer: "1".to_string(),
                    enable: vec![TimelineEnable {
                        enable_start: Some(Expression::String("#extRef".to_string())),
                        duration: Some(Expression::Number(200)),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
                (SimpleTimelineObj {
                    id: "interrupting".to_string(),
                    layer: "1".to_string(),
                    enable: vec![TimelineEnable {
                        enable_start: Some(Expression::Number(10)), // 60, will interrupt video in the middle of it
                        duration: Some(Expression::Number(10)),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
                (SimpleTimelineObj {
                    id: "video2".to_string(),
                    layer: "2".to_string(),
                    enable: vec![TimelineEnable {
                        enable_start: Some(Expression::String("-10".to_string())), // 40
                        duration: Some(Expression::Number(200)),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
                (SimpleTimelineObj {
                    id: "interrupting2".to_string(),
                    layer: "2".to_string(),
                    enable: vec![TimelineEnable {
                        enable_while: Some(Expression::String("#interrupting".to_string())),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
            ]),
            ..Default::default()
        }),
    ];

    let options = ResolveOptions {
        time: 0,
        limit_count: Some(100),
        limit_time: Some(99999),
    };

    let resolved = resolve_timeline(&timeline, options).expect("Resolve timeline failed");
    let states = resolve_all_states(&resolved, None).expect("Resolve states failed");

    let obj_my_group = states
        .objects
        .get("myGroup")
        .expect("Missing myGroup object");
    assert_instances(
        &obj_my_group.instances,
        &vec![Rc::new(TimelineObjectInstance {
            start: 50,
            end: Some(100),
            ..Default::default()
        })],
    );

    let obj_interrupting = states
        .objects
        .get("interrupting")
        .expect("Missing interrupting object");
    assert_instances(
        &obj_interrupting.instances,
        &vec![Rc::new(TimelineObjectInstance {
            start: 60,
            end: Some(70),
            ..Default::default()
        })],
    );

    let obj_video = states.objects.get("video").expect("Missing video object");
    assert_instances(
        &obj_video.instances,
        &vec![
            Rc::new(TimelineObjectInstance {
                start: 50,
                end: Some(60),
                original_start: Some(10),
                original_end: Some(210),
                ..Default::default()
            }),
            Rc::new(TimelineObjectInstance {
                start: 70,
                end: Some(100),
                original_start: Some(10),
                original_end: Some(210),
                ..Default::default()
            }),
        ],
    );

    let obj_video2 = states.objects.get("video2").expect("Missing video2 object");
    assert_instances(
        &obj_video2.instances,
        &vec![
            Rc::new(TimelineObjectInstance {
                start: 50,
                end: Some(60),
                original_start: Some(50), // TODO - 40?
                original_end: Some(250),  // TODO - 250?
                ..Default::default()
            }),
            Rc::new(TimelineObjectInstance {
                start: 70,
                end: Some(100),
                original_start: Some(50), // TODO - 40?
                original_end: Some(250),  // TODO - 250?
                ..Default::default()
            }),
        ],
    );
}

#[test]
fn parent_references() {
    let timeline: Vec<SimpleTimelineObj> = vec![
        (SimpleTimelineObj {
            id: "parent".to_string(),
            layer: "p0".to_string(),
            priority: 0,
            enable: vec![TimelineEnable {
                enable_start: Some(Expression::Number(100)),
                ..Default::default()
            }],
            children: Some(vec![
                (SimpleTimelineObj {
                    id: "video0".to_string(),
                    layer: "0".to_string(),
                    priority: 0,
                    enable: vec![TimelineEnable {
                        enable_start: Some(Expression::Number(20 + 30)),
                        duration: Some(Expression::Number(10)),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
                (SimpleTimelineObj {
                    id: "video1".to_string(),
                    layer: "1".to_string(),
                    priority: 0,
                    enable: vec![TimelineEnable {
                        enable_start: Some(Expression::String("20 + 30".to_string())),
                        duration: Some(Expression::Number(10)),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
            ]),
            ..Default::default()
        }),
        (SimpleTimelineObj {
            id: "video2".to_string(),
            layer: "2".to_string(),
            priority: 0,
            enable: vec![TimelineEnable {
                enable_start: Some(Expression::Number(150)),
                duration: Some(Expression::Number(10)),
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
    let obj_video2 = states.objects.get("video2").expect("Missing video2 object");

    assert_instances(
        &obj_video0.instances,
        &vec![Rc::new(TimelineObjectInstance {
            start: 150,
            end: Some(160),
            ..Default::default()
        })],
    );
    assert_instances(
        &obj_video1.instances,
        &vec![Rc::new(TimelineObjectInstance {
            start: 150,
            end: Some(160),
            ..Default::default()
        })],
    );
    assert_instances(
        &obj_video2.instances,
        &vec![Rc::new(TimelineObjectInstance {
            start: 150,
            end: Some(160),
            ..Default::default()
        })],
    );
}
