use crate::api::{ResolverContext, DEFAULT_LIMIT_COUNT};
use crate::events::{EventForInstance, EventForInstanceExt};
use crate::instance::{Cap, TimelineObjectInstance};
use crate::lookup_expression::LookupExpressionResultType;
use crate::references::ReferencesBuilder;
use crate::resolver::TimeWithReference;
use crate::state::ResolveOptions;
use std::cmp::{max, min};
use std::collections::{HashMap, HashSet};

pub type Time = u64;

pub fn invert_instances(
    ctx: &dyn ResolverContext,
    instances: &Vec<TimelineObjectInstance>,
) -> Vec<TimelineObjectInstance> {
    if instances.len() == 0 {
        vec![TimelineObjectInstance {
            id: ctx.get_id(),
            isFirst: true,
            start: 0,
            end: None,
            references: HashSet::new(),

            originalStart: None,
            originalEnd: None,

            caps: Vec::new(),
            fromInstanceId: None,
        }]
    } else {
        let cleaned_instances = clean_instances(ctx, instances, true, true);

        let mut inverted_instances = Vec::new();

        let first_instance = &cleaned_instances[0];

        // Fill the time between the first and zero
        if first_instance.start != 0 {
            inverted_instances.push(TimelineObjectInstance {
                id: ctx.get_id(),
                isFirst: true,
                start: 0,
                end: None,
                references: ReferencesBuilder::new()
                    .add(&first_instance.references)
                    .add_id(&first_instance.id)
                    .done(),
                caps: Vec::new(),

                // TODO - what are these for if they arent set here?
                originalStart: None,
                originalEnd: None,
                fromInstanceId: None,
            });
        }

        // Fill in between the instances
        for instance in cleaned_instances {
            if let Some(prev_instance) = inverted_instances.last_mut() {
                prev_instance.end = Some(instance.start);
            }

            if let Some(end) = instance.end {
                inverted_instances.push(TimelineObjectInstance {
                    id: ctx.get_id(),
                    isFirst: false,
                    start: end,
                    end: None,
                    references: ReferencesBuilder::new()
                        .add(&instance.references)
                        .add_id(&instance.id)
                        .done(),
                    caps: instance.caps,

                    // TODO - what are these for if they arent set here?
                    originalStart: None,
                    originalEnd: None,
                    fromInstanceId: None,
                });
            }
        }

        inverted_instances
    }
}

// Cleanup instances. Join overlaps or touching etc
pub fn clean_instances(
    ctx: &dyn ResolverContext,
    instances: &Vec<TimelineObjectInstance>,
    allow_merge: bool,
    allow_zero_gaps: bool,
) -> Vec<TimelineObjectInstance> {
    match instances.len() {
        0 => Vec::new(),
        1 => {
            let mut instance = instances[0].clone();
            instance.originalStart = Some(instance.start);
            instance.originalEnd = instance.end;

            vec![instance]
        }
        _ => {
            let mut events = Vec::new();

            for instance in instances {
                events.push(EventForInstance {
                    time: instance.start,
                    is_start: true,
                    references: &instance.references,
                    instance,
                    id: None,
                });

                if let Some(end) = instance.end {
                    events.push(EventForInstance {
                        time: end,
                        is_start: false,
                        references: &instance.references,
                        instance,
                        id: None,
                    });
                }
            }

            events.to_instances(ctx, allow_merge, allow_zero_gaps)
        }
    }
}

pub fn add_caps_to_resuming(instance: &mut TimelineObjectInstance, caps: &Vec<Cap>) {
    let mut new_caps = Vec::new();

    for cap in caps {
        if let Some(cap_end) = cap.end {
            if let Some(instance_end) = instance.end {
                if cap_end > instance_end {
                    new_caps.push(Cap {
                        id: cap.id.clone(),
                        start: 0,
                        end: Some(cap_end),
                    })
                }
            }
        }
    }

    instance.caps = join_caps(&instance.caps, &new_caps)
}

pub fn join_caps(a: &Vec<Cap>, b: &Vec<Cap>) -> Vec<Cap> {
    let mut cap_map = HashMap::new();

    for cap in a {
        cap_map.insert(&cap.id, cap.clone());
    }
    for cap in b {
        cap_map.insert(&cap.id, cap.clone());
    }

    cap_map.into_iter().map(|e| e.1).collect()
}

// pub fn clone_hashset_with_value<T: Clone + Eq + Hash>(a: &HashSet<T>, c: &T) -> HashSet<T> {
//     let mut res = HashSet::new();
//     res.extend(a.iter().cloned());
//     res.insert(c.clone());
//     res
// }
//
// pub fn join_maybe_hashset<T: Clone + Eq + Hash>(
//     a: Option<&HashSet<T>>,
//     b: Option<&HashSet<T>>,
// ) -> HashSet<T> {
//     let mut res = HashSet::new();
//     if let Some(a) = a {
//         res.extend(a.iter().cloned());
//     }
//     if let Some(b) = b {
//         res.extend(b.iter().cloned());
//     }
//     res
// }
//
// pub fn join_hashset<T: Clone + Eq + Hash>(a: &HashSet<T>, b: &HashSet<T>) -> HashSet<T> {
//     let mut res = HashSet::new();
//     res.extend(a.iter().cloned());
//     res.extend(b.iter().cloned());
//     res
// }

fn get_as_array_to_operate(a: &LookupExpressionResultType) -> Option<&Vec<TimelineObjectInstance>> {
    match a {
        LookupExpressionResultType::Null => None,
        LookupExpressionResultType::TimeRef(time_ref) => Some(&vec![TimelineObjectInstance {
            id: "".to_string(),
            start: time_ref.value,
            end: Some(time_ref.value),
            references: time_ref.references.clone(),

            isFirst: false,
            originalStart: None,
            originalEnd: None,
            caps: vec![],
            fromInstanceId: None,
        }]),
        LookupExpressionResultType::Instances(instances) => Some(instances),
    }
}

pub fn operate_on_arrays<T>(
    ctx: &dyn ResolverContext,
    lookup0: &LookupExpressionResultType,
    lookup1: &LookupExpressionResultType,
    operate: &T,
) -> LookupExpressionResultType
where
    T: Fn(Option<&TimeWithReference>, Option<&TimeWithReference>) -> Option<TimeWithReference>,
{
    if let Some(lookup0) = get_as_array_to_operate(lookup0) {
        if let Some(lookup1) = get_as_array_to_operate(lookup1) {
            // TODO - both refs shortcut
            // if (
            //     isReference(array0) &&
            //         isReference(array1)
            // ) {
            //     return operate(array0, array1)
            // }

            let mut result = Vec::new();

            // let min_length = min(lookup0.len(), lookup1.len());
            // Iterate through both until we run out of one
            for (a, b) in lookup0.iter().zip(lookup1.iter()) {
                let start = if a.isFirst {
                    Some(TimeWithReference {
                        value: a.start,
                        references: a.references.clone(),
                    })
                } else if b.isFirst {
                    Some(TimeWithReference {
                        value: b.start,
                        references: b.references.clone(),
                    })
                } else {
                    operate(
                        Some(&TimeWithReference {
                            value: a.start,
                            references: ReferencesBuilder::new()
                                .add(&a.references)
                                .add_id(&a.id)
                                .done(),
                        }),
                        Some(&TimeWithReference {
                            value: b.start,
                            references: ReferencesBuilder::new()
                                .add(&b.references)
                                .add_id(&b.id)
                                .done(),
                        }),
                    )
                };

                if let Some(start) = start {
                    let end = if a.isFirst {
                        a.end.and_then(|end| {
                            Some(TimeWithReference {
                                value: end,
                                references: a.references.clone(),
                            })
                        })
                    } else if b.isFirst {
                        b.end.and_then(|end| {
                            Some(TimeWithReference {
                                value: end,
                                references: b.references.clone(),
                            })
                        })
                    } else {
                        operate(
                            a.end.and_then(|end| {
                                Some(&TimeWithReference {
                                    value: end,
                                    references: ReferencesBuilder::new()
                                        .add(&a.references)
                                        .add_id(&a.id)
                                        .done(),
                                })
                            }),
                            b.end.and_then(|end| {
                                Some(&TimeWithReference {
                                    value: end,
                                    references: ReferencesBuilder::new()
                                        .add(&b.references)
                                        .add_id(&b.id)
                                        .done(),
                                })
                            }),
                        )
                    };

                    result.push(TimelineObjectInstance {
                        id: ctx.get_id(),
                        start: start.value,
                        end: end.and_then(|e| Some(e.value)),
                        references: ReferencesBuilder::new()
                            .add(&start.references)
                            .add_some(end.and_then(|e| Some(&e.references)))
                            .done(),
                        caps: join_caps(&a.caps, &b.caps),

                        isFirst: false,
                        originalStart: None,
                        originalEnd: None,
                        fromInstanceId: None,
                    })
                }
            }

            LookupExpressionResultType::Instances(clean_instances(ctx, &result, false, false))
        } else {
            LookupExpressionResultType::Null
        }
    } else {
        LookupExpressionResultType::Null
    }
}

pub fn apply_repeating_instances(
    ctx: &dyn ResolverContext,
    instances: &Vec<TimelineObjectInstance>,
    repeat_time: Option<TimeWithReference>,
    options: &ResolveOptions,
) -> Vec<TimelineObjectInstance> {
    if let Some(repeat_time) = &repeat_time {
        let mut repeated_instances = Vec::new();

        // TODO - why was this necessary?
        // if (isReference(instances)) {
        //     instances = [{
        //         id: '',
        //         start: instances.value,
        //         end: null,
        //         references: instances.references
        //     }]
        // }

        for instance in instances {
            let mut start_time = max(
                options.time - ((options.time - instance.start) % repeat_time.value),
                instance.start,
            );
            let mut end_time = instance
                .end
                .and_then(|end| Some(end + (start_time - instance.start)));

            let cap = instance
                .caps
                .iter()
                .find(|cap| instance.references.contains(&cap.id));

            let limit = options.limitCount.unwrap_or(DEFAULT_LIMIT_COUNT);
            for i in 0..limit {
                if let Some(limit_time) = options.limitTime {
                    if start_time >= limit_time {
                        break;
                    }
                }

                let capped_start_time = cap
                    .and_then(|cap| Some(max(cap.start, start_time)))
                    .unwrap_or(start_time);
                let capped_end_time = if let Some(end_time) = end_time {
                    Some(
                        cap.and_then(|cap| cap.end)
                            .and_then(|cap_end| Some(min(cap_end, end_time)))
                            .unwrap_or(end_time),
                    )
                } else {
                    None
                };

                if capped_end_time.unwrap_or(Time::MAX) > capped_start_time {
                    let references = ReferencesBuilder::new()
                        .add_id(&instance.id)
                        .add(&instance.references)
                        .add(&repeat_time.references)
                        .done();
                    repeated_instances.push(TimelineObjectInstance {
                        id: ctx.get_id(),
                        start: capped_start_time,
                        end: capped_end_time,
                        references,

                        isFirst: false,
                        originalStart: None,
                        originalEnd: None,
                        caps: Vec::new(),
                        fromInstanceId: None,
                    })
                }

                start_time += repeat_time.value;
                if let Some(end_time0) = &end_time {
                    end_time = Some(end_time0 + repeat_time.value);
                }
            }
        }

        clean_instances(ctx, &repeated_instances, false, false)
    } else {
        instances.clone()
    }
}

pub fn apply_parent_instances(
    ctx: &dyn ResolverContext,
    parent_instances: &Option<Vec<TimelineObjectInstance>>,
    value: &LookupExpressionResultType,
) -> LookupExpressionResultType {
    if let Some(parent_instances) = parent_instances {
        let operate = |a: Option<&TimeWithReference>, b: Option<&TimeWithReference>| {
            if let Some(a) = a {
                if let Some(b) = b {
                    Some(TimeWithReference {
                        value: a.value + b.value,
                        references: ReferencesBuilder::new()
                            .add(&a.references)
                            .add(&b.references)
                            .done(),
                    })
                } else {
                    None
                }
            } else {
                None
            }
        };
        operate_on_arrays(
            ctx,
            &LookupExpressionResultType::Instances(parent_instances.clone()),
            value,
            &operate,
        )
    } else {
        LookupExpressionResultType::Null
    }
}

pub fn cap_instances(
    instances: &Vec<TimelineObjectInstance>,
    parent_instances: &LookupExpressionResultType,
) -> Vec<TimelineObjectInstance> {
    match parent_instances {
        LookupExpressionResultType::Null => instances.clone(),
        LookupExpressionResultType::TimeRef(_) => instances.clone(),
        LookupExpressionResultType::Instances(parent_instances) => {
            let mut return_instances = Vec::new();

            for instance in instances {
                if let Some(new_instance) =
                    cap_instance(instance, &parent_instances.iter().collect())
                {
                    return_instances.push(new_instance);
                }
            }

            return_instances
        }
    }
}

pub fn cap_instance(
    instance: &TimelineObjectInstance,
    parent_instances: &Vec<&TimelineObjectInstance>,
) -> Option<TimelineObjectInstance> {
    let mut parent: Option<&TimelineObjectInstance> = None;

    let instance_end = instance.end.unwrap_or(Time::MAX);

    for p in *parent_instances {
        let p_end = p.end.unwrap_or(Time::MAX);
        // TODO - could this not be achieved by instance.start <= p.end && instance.end >= p.start ?
        if (instance.start >= p.start && instance.start < p_end)
            || (instance.start < p.start && instance_end > p_end)
        {
            if let Some(old_parent) = parent {
                if p_end > old_parent.end.unwrap_or(Time::MAX) {
                    parent = Some(p);
                }
            } else {
                parent = Some(p);
            }
        }
    }

    if parent.is_none() {
        for p in parent_instances {
            if instance_end > p.start && instance_end <= p.end.unwrap_or(Time::MAX) {
                parent = Some(p);
            }
        }
    }

    if let Some(parent) = parent {
        let mut instance2 = instance.clone();

        if let Some(p_end) = parent.end {
            if instance.end.unwrap_or(Time::MAX) > p_end {
                set_instance_end_time(&mut instance2, p_end)
            }
        }

        if instance.start < parent.start {
            set_instance_start_time(&mut instance2, parent.start)
        }

        Some(instance2)
    } else {
        None
    }
}

pub fn set_instance_end_time(instance: &mut TimelineObjectInstance, end: Time) {
    if instance.originalEnd.is_none() {
        instance.originalEnd = instance.end;
    }

    instance.end = Some(end);
}
pub fn set_instance_start_time(instance: &mut TimelineObjectInstance, start: Time) {
    if instance.originalStart.is_none() {
        instance.originalStart = Some(instance.start);
    }

    instance.start = start;
}
