use crate::instance::TimelineEnable;
use crate::instance::TimelineObjectInfo;
use crate::resolver::ResolveError;
use crate::resolver::ResolverContext;
use crate::resolver::{ResolvingTimelineObject, TimelineObjectResolvingStatus};
use crate::state::ResolvedTimelineObject;
use crate::util::Time;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::RwLock;

pub const DEFAULT_LIMIT_COUNT: usize = 2;

/*
pub trait IsTimelineObjectChildren {
    fn children (&self) -> Option<Vec<Box<IsTimelineObject>>>;
}
*/

pub trait IsTimelineObject /*: IsTimelineObjectChildren */ {
    fn id(&self) -> &str;
    fn enable(&self) -> &Vec<TimelineEnable>;
    fn layer(&self) -> &str;
    fn keyframes(&self) -> Option<&Vec<Box<dyn IsTimelineKeyframe>>>;
    fn classes(&self) -> Option<&Vec<String>>;
    fn disabled(&self) -> bool;
    //fn is_group (&self) -> bool;
    fn children(&self) -> Option<&Vec<Box<dyn IsTimelineObject>>>;
    fn priority(&self) -> u64;
}

pub trait IsTimelineKeyframe {
    fn id(&self) -> &str;
    fn enable(&self) -> &Vec<TimelineEnable>;
    //fn duration (&self) -> Option<TimelineKeyframeDuration>;
    fn classes(&self) -> Option<&Vec<String>>;
    fn disabled(&self) -> bool;
}

#[derive(Debug, Clone)]
pub struct ResolveOptions {
    /** The base time to use when resolving. Usually you want to input the current time (Date.now()) here. */
    pub time: Time,
    /** Limits the number of repeating objects in the future.
     * Defaults to 2, which means that the current one and the next will be resolved.
     */
    pub limit_count: Option<usize>,
    /** Limits the repeating objects to a time in the future */
    pub limit_time: Option<Time>,
    // /** If set to true, the resolver will go through the instances of the objects and fix collisions, so that the instances more closely resembles the end state. */
    // pub resolve_instance_collisions: bool, // /** A cache thet is to persist data between resolves. If provided, will increase performance of resolving when only making small changes to the timeline. */
    //                                        // cache?: ResolverCache
}

fn add_object_to_resolved_timeline(
    timeline: &mut ResolvedTimeline,
    resolving_objects: &mut HashMap<String, ResolvingTimelineObject>,
    obj: ResolvingTimelineObject,
    raw_obj: Option<&dyn IsTimelineObject>,
) {
    let obj_id = obj.info.id.to_string();

    if let Some(raw_obj) = raw_obj {
        if let Some(classes) = raw_obj.classes() {
            for class in classes {
                if let Some(existing) = timeline.classes.get_mut(class) {
                    existing.push(obj_id.clone());
                } else {
                    timeline
                        .classes
                        .insert(class.to_string(), vec![obj_id.clone()]);
                }
            }
        }

        let obj_layer = raw_obj.layer();
        if !obj_layer.is_empty() {
            if let Some(existing) = timeline.layers.get_mut(obj_layer) {
                existing.push(obj_id.clone());
            } else {
                timeline
                    .layers
                    .insert(obj_layer.to_string(), vec![obj_id.clone()]);
            }
        }
    }

    // finally move the object
    resolving_objects.insert(obj_id, obj);
}

fn add_object_to_timeline(
    timeline: &mut ResolvedTimeline,
    resolving_objects: &mut HashMap<String, ResolvingTimelineObject>,
    obj: &dyn IsTimelineObject,
    depth: usize,
    parent_id: Option<&String>,
) {
    // TODO - duplicate id check
    // if (resolvedTimeline.objects[obj.id]) throw Error(`All timelineObjects must be unique! (duplicate: "${obj.id}")`)

    let resolved_obj = ResolvingTimelineObject {
        resolved: RwLock::new(TimelineObjectResolvingStatus::Pending),
        info: TimelineObjectInfo {
            id: obj.id().to_string(),
            enable: obj.enable().clone(),
            priority: obj.priority(),
            disabled: obj.disabled(),
            layer: obj.layer().to_string(),

            depth,
            parent_id: parent_id.cloned(),
            is_keyframe: false,
        },
    };

    // track child objects
    if let Some(children) = obj.children() {
        for child in children {
            add_object_to_timeline(
                timeline,
                resolving_objects,
                child.as_ref(),
                depth + 1,
                Some(&resolved_obj.info.id),
            );
        }
    }

    // track keyframes
    if let Some(keyframes) = obj.keyframes() {
        for keyframe in keyframes {
            let resolved_obj = ResolvingTimelineObject {
                resolved: RwLock::new(TimelineObjectResolvingStatus::Pending),
                info: TimelineObjectInfo {
                    id: keyframe.id().to_string(),
                    enable: keyframe.enable().clone(),
                    priority: 0,           // not supported
                    layer: "".to_string(), // not supported
                    disabled: keyframe.disabled(),

                    depth: depth + 1,
                    parent_id: Some(resolved_obj.info.id.clone()),
                    is_keyframe: true,
                },
            };
            add_object_to_resolved_timeline(timeline, resolving_objects, resolved_obj, None)
        }
    }

    add_object_to_resolved_timeline(timeline, resolving_objects, resolved_obj, Some(obj));
}

pub struct ResolvedTimeline {
    pub options: ResolveOptions,
    /** Map of all objects on timeline */
    pub objects: HashMap<String, ResolvedTimelineObject>,
    /** Map of all classes on timeline, maps className to object ids */
    pub classes: HashMap<String, Vec<String>>,
    /** Map of the object ids, per layer */
    pub layers: HashMap<String, Vec<String>>,
}

pub fn resolve_timeline(
    timeline: &[Box<dyn IsTimelineObject>],
    options: ResolveOptions,
) -> Result<Box<ResolvedTimeline>, ResolveError> {
    let mut resolved_timeline = Box::new(ResolvedTimeline {
        objects: HashMap::new(),
        classes: HashMap::new(),
        layers: HashMap::new(),
        options,
    });

    // Step 1: pre-populate resolvedTimeline with objects
    let mut resolving_objects = HashMap::new();
    for obj in timeline {
        add_object_to_timeline(
            &mut resolved_timeline,
            &mut resolving_objects,
            obj.as_ref(),
            0,
            None,
        );
    }

    let resolver_context = ResolverContext::create(&resolved_timeline, resolving_objects);

    // Step 2: go though and resolve the objects
    // TODO - support cache
    for obj in resolver_context.objects.iter() {
        // TODO - the immutability here will cause me nightmares
        resolver_context.resolve_object(obj.1)?;
    }

    let mut unresolved_ids = Vec::new();

    // TODO - convert the objects/instances
    for (id, obj) in resolver_context.objects.into_iter() {
        let inner = obj.resolved.into_inner().unwrap(); // TODO - handle error
        match inner {
            TimelineObjectResolvingStatus::Pending => {
                unresolved_ids.push(id);
            }
            TimelineObjectResolvingStatus::InProgress(_) => {
                unresolved_ids.push(id);
            }
            TimelineObjectResolvingStatus::Complete(res) => {
                resolved_timeline.objects.insert(
                    id,
                    ResolvedTimelineObject {
                        info: Rc::new(obj.info),
                        resolved: res,
                    },
                );
            }
        }
    }

    if !unresolved_ids.is_empty() {
        Err(ResolveError::UnresolvedObjects(unresolved_ids))
    } else {
        Ok(resolved_timeline)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::Expression;
    use crate::get_state;
    use crate::instance::TimelineObjectInstance;
    use crate::state::resolve_all_states;
    use crate::state::TimelineState;
    use crate::state::{EventType, NextEvent};
    use std::cmp::Ordering;
    use std::collections::HashSet;
    use std::rc::Rc;
    use std::sync::Mutex;

    #[derive(Default)]
    struct SimpleTimelineObj {
        pub id: String,
        pub enable: Vec<TimelineEnable>,
        pub layer: String,
        pub keyframes: Option<Vec<Box<dyn IsTimelineKeyframe>>>,
        pub classes: Option<Vec<String>>,
        pub disabled: bool,
        pub children: Option<Vec<Box<dyn IsTimelineObject>>>,
        pub priority: u64,
    }
    impl IsTimelineObject for SimpleTimelineObj {
        fn id(&self) -> &str {
            &self.id
        }
        fn enable(&self) -> &Vec<TimelineEnable> {
            &self.enable
        }
        fn layer(&self) -> &str {
            &self.layer
        }
        fn keyframes(&self) -> Option<&Vec<Box<dyn IsTimelineKeyframe>>> {
            self.keyframes.as_ref()
        }
        fn classes(&self) -> Option<&Vec<String>> {
            self.classes.as_ref()
        }
        fn disabled(&self) -> bool {
            self.disabled
        }
        fn children(&self) -> Option<&Vec<Box<dyn IsTimelineObject>>> {
            self.children.as_ref()
        }
        fn priority(&self) -> u64 {
            self.priority
        }
    }

    fn assert_instances(
        result: &HashMap<String, Rc<Mutex<TimelineObjectInstance>>>,
        expected: &Vec<Rc<TimelineObjectInstance>>,
    ) {
        assert_eq!(result.len(), expected.len());

        let mut result_vec: Vec<TimelineObjectInstance> = result
            .values()
            .map(|v| {
                let v2 = v.try_lock().unwrap();
                v2.clone()
            })
            .collect();
        result_vec.sort_by(|a, b| {
            if a.start < b.start {
                Ordering::Less
            } else if a.start > b.start {
                Ordering::Greater
            } else {
                b.id.cmp(&a.id)
            }
        });

        for (val, exp) in result_vec.iter().zip(expected) {
            assert_eq!(val.start, exp.start);
            assert_eq!(val.end, exp.end);

            // TODO - more props
        }
    }

    fn assert_obj_on_layer(state: &TimelineState, layer: &str, id: &str) {
        let obj = state
            .layers
            .get(layer)
            .expect(&format!("Expected '{}' on layer '{}'", id, layer));

        assert_eq!(obj.info.id, id.to_string());
    }

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
            // assert!(state0.layers.get("1").is_none()); // TODO - urgent
        }

        {
            let state0 = get_state(&states, 31, None);
            assert_obj_on_layer(&state0, "0", "video");
            assert_obj_on_layer(&state0, "1", "graphic1");
        }

        {
            let state0 = get_state(&states, 46, None);
            assert_obj_on_layer(&state0, "0", "video");
            // assert!(state0.layers.get("1").is_none()); // TODO - urgent
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
        // TODO - graphic0 is not repeating
        assert_instances(
            // TODO - urgent
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
            // assert!(state0.layers.get("1").is_none()); // TODO - urgent
        }

        {
            let state0 = get_state(&states, 51, None);
            assert_obj_on_layer(&state0, "0", "video");
            // assert_obj_on_layer(&state0, "1", "graphic0");
        }

        {
            let state0 = get_state(&states, 72, None);
            assert_obj_on_layer(&state0, "0", "video");
            assert_obj_on_layer(&state0, "1", "graphic0");
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
}
