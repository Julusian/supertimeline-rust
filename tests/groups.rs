extern crate supertimeline;
mod objs;
mod util;

use crate::objs::SimpleTimelineObj;
use crate::util::assert_obj_on_layer;
use crate::util::{assert_instances, assert_instances2};
use std::rc::Rc;
use supertimeline::get_state;
use supertimeline::NextEvent;
use supertimeline::TimelineObjectInstance;
use supertimeline::{
    resolve_all_states, resolve_timeline, EventType, Expression, IsTimelineObject, ResolveOptions,
    TimelineEnable,
};

#[test]
fn simple_group() {
    let timeline: Vec<Box<dyn IsTimelineObject>> = vec![Box::new(SimpleTimelineObj {
        id: "group".to_string(),
        layer: "0".to_string(),
        enable: vec![TimelineEnable {
            enable_start: Some(Expression::Number(10)),
            enable_end: Some(Expression::Number(100)),
            ..Default::default()
        }],
        children: Some(vec![
            Box::new(SimpleTimelineObj {
                id: "child0".to_string(),
                layer: "1".to_string(),
                enable: vec![TimelineEnable {
                    enable_start: Some(Expression::String("5".to_string())), // 15
                    duration: Some(Expression::Number(10)),                  // 25
                    ..Default::default()
                }],
                ..Default::default()
            }),
            Box::new(SimpleTimelineObj {
                id: "child1".to_string(),
                layer: "1".to_string(),
                enable: vec![TimelineEnable {
                    enable_start: Some(Expression::String("#child0.end".to_string())), // 25
                    duration: Some(Expression::Number(10)),                            // 35
                    ..Default::default()
                }],
                ..Default::default()
            }),
            Box::new(SimpleTimelineObj {
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
    })];

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
    let timeline: Vec<Box<dyn IsTimelineObject>> = vec![
        Box::new(SimpleTimelineObj {
            id: "group0".to_string(),
            layer: "".to_string(),
            enable: vec![TimelineEnable {
                enable_start: Some(Expression::Number(10)),
                enable_end: Some(Expression::Number(100)),
                ..Default::default()
            }],
            children: Some(vec![Box::new(SimpleTimelineObj {
                id: "child0".to_string(),
                layer: "1".to_string(),
                enable: vec![TimelineEnable {
                    enable_start: Some(Expression::String("5".to_string())), // 15
                    ..Default::default()
                }],
                ..Default::default()
            })]),
            ..Default::default()
        }),
        Box::new(SimpleTimelineObj {
            id: "group1".to_string(),
            layer: "".to_string(),
            enable: vec![TimelineEnable {
                enable_start: Some(Expression::Number(50)),
                enable_end: Some(Expression::Number(100)),
                ..Default::default()
            }],
            children: Some(vec![Box::new(SimpleTimelineObj {
                id: "child1".to_string(),
                layer: "2".to_string(),
                enable: vec![TimelineEnable {
                    enable_start: Some(Expression::String("5".to_string())), // 55
                    ..Default::default()
                }],
                ..Default::default()
            })]),
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
                    object_id: "child1".to_string(),
                    time: 100,
                },
                NextEvent {
                    event_type: EventType::End,
                    object_id: "child0".to_string(),
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
