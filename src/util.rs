use crate::instance::{Cap, TimelineObjectInstance};
use crate::lookup_expression::LookupExpressionResultType;
use crate::resolver::TimeWithReference;
use crate::state::ResolveOptions;
use std::cmp::max;
use std::collections::{HashMap, HashSet};

pub type Time = u64;

#[derive(Debug, Clone)]
pub struct TimelineObject {
    // TODO
    pub id: String,
}

pub fn getId() -> String {
    // TODO
    "".to_string()
}

pub fn invert_instances(instances: &Vec<TimelineObjectInstance>) -> Vec<TimelineObjectInstance> {
    if instances.len() == 0 {
        vec![TimelineObjectInstance {
            id: getId(),
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
        let cleaned_instances = clean_instances(instances, true, true);

        let mut inverted_instances = Vec::new();

        let first_instance = &cleaned_instances[0];

        // Fill the time between the first and zero
        if first_instance.start != 0 {
            inverted_instances.push(TimelineObjectInstance {
                id: getId(),
                isFirst: true,
                start: 0,
                end: None,
                references: join_references(
                    &first_instance.references,
                    None,
                    Some(&first_instance.id),
                ),
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
                    id: getId(),
                    isFirst: false,
                    start: end,
                    end: None,
                    references: join_references(&instance.references, None, Some(&instance.id)),
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
                });

                if let Some(end) = instance.end {
                    events.push(EventForInstance {
                        time: end,
                        is_start: false,
                        references: &instance.references,
                        instance,
                    });
                }
            }

            convert_events_to_instances(events, allow_merge, allow_zero_gaps)
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
        cap_map.insert(&cap.id, cap);
    }
    for cap in b {
        cap_map.insert(&cap.id, cap);
    }

    cap_map.values().cloned()
}

pub fn join_references(
    a: &HashSet<String>,
    b: Option<&HashSet<String>>,
    c: Option<&String>,
) -> HashSet<String> {
    let mut new_refs = HashSet::new();
    new_refs.extend(a);

    if let Some(b) = b {
        new_refs.extend(b);
    }

    if let Some(c) = c {
        new_refs.insert(c);
    }

    new_refs
}

pub fn join_references3(
    a: &Option<TimeWithReference>,
    b: &Option<TimeWithReference>,
) -> HashSet<String> {
    let mut new_refs = HashSet::new();
    if let Some(a) = a {
        new_refs.extend(&a.references);
    }
    if let Some(b) = b {
        new_refs.extend(&b.references);
    }
    new_refs.cloned()
}

pub fn join_references4(
    a: Option<&HashSet<String>>,
    b: Option<&HashSet<String>>,
) -> HashSet<String> {
    let mut new_refs = HashSet::new();
    if let Some(a) = a {
        new_refs.extend(&a);
    }
    if let Some(b) = b {
        new_refs.extend(&b);
    }
    new_refs.cloned()
}

pub fn join_references2(a: &HashSet<String>, b: &HashSet<String>) -> HashSet<String> {
    let mut new_refs = HashSet::new();
    new_refs.extend(a);
    new_refs.extend(b);
    new_refs.cloned()
}

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

pub fn operate_on_arrays(
    lookup0: &LookupExpressionResultType,
    lookup1: &LookupExpressionResultType,
    operate: fn(
        a: Option<&TimeWithReference>,
        b: Option<&TimeWithReference>,
    ) -> Option<TimeWithReference>,
) -> LookupExpressionResultType {
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
                            references: join_references(&a.references, None, Some(&a.id)),
                        }),
                        Some(&TimeWithReference {
                            value: b.start,
                            references: join_references(&b.references, None, Some(&b.id)),
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
                                    references: join_references(&a.references, None, Some(&a.id)),
                                })
                            }),
                            b.end.and_then(|end| {
                                Some(&TimeWithReference {
                                    value: end,
                                    references: join_references(&b.references, None, Some(&b.id)),
                                })
                            }),
                        )
                    };

                    result.push(TimelineObjectInstance {
                        id: getId(),
                        start: start.value,
                        end: end.and_then(|e| Some(e.value)),
                        references: join_references(
                            &start.references,
                            end.and_then(|e| Some(&e.references)),
                            None,
                        ),
                        caps: join_caps(&a.caps, &b.caps),

                        isFirst: false,
                        originalStart: None,
                        originalEnd: None,
                        fromInstanceId: None,
                    })
                }
            }

            LookupExpressionResultType::Instances(clean_instances(&result, false, false))
        } else {
            LookupExpressionResultType::Null
        }
    } else {
        LookupExpressionResultType::Null
    }
}

pub fn apply_repeating_instances(
    instances: &Vec<TimelineObjectInstance>,
    repeat_time: Option<TimeWithReference>,
    options: &ResolveOptions,
) -> Vec<TimelineObjectInstance> {
    if let Some(repeat_time) = &repeat_time {
        let repeated_instances = Vec::new();

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
            let start_time = max(
                options.time - ((options.time - instance.start) % repeat_time.value),
                instance.start,
            );
            let end_time = instance
                .end
                .and_then(|end| Some(end + (start_time - instance.start)));

            // TODO
            // let cap = instance.caps
            // const cap: Cap | null = (
            //     instance.caps ?
            // _.find(instance.caps, (cap) => instance.references.indexOf(cap.id) !== -1)
            //     : null
            // ) || null
            //
            // const limit = options.limitCount || 2
            // for (let i = 0; i < limit; i++) {
            //     if (
            //         options.limitTime &&
            //             startTime >= options.limitTime
            //     ) break
            //
            //     const cappedStartTime: Time = (
            //         cap ?
            //     Math.max(cap.start, startTime) :
            //         startTime
            //     )
            //     const cappedEndTime: Time | null = (
            //         cap && cap.end !== null && endTime !== null ?
            //     Math.min(cap.end, endTime) :
            //         endTime
            //     )
            //     if ((cappedEndTime || Infinity) > cappedStartTime) {
            //         repeatedInstances.push({
            //             id: getId(),
            //             start: cappedStartTime,
            //             end: cappedEndTime,
            //             references: joinReferences(instance.id, instance.references, repeatTime0.references)
            //         })
            //     }
            //
            //     startTime += repeatTime
            //     if (endTime !== null) endTime += repeatTime
            // }
        }

        clean_instances(&repeated_instances, false, false)
    } else {
        instances.clone()
    }
}

pub fn apply_parent_instances(
    parent_instances: &Option<Vec<TimelineObjectInstance>>,
    value: &LookupExpressionResultType,
) -> LookupExpressionResultType {
    let operate = |a: Option<&TimeWithReference>, b: Option<&TimeWithReference>| {
        if let Some(a) = a {
            if let Some(b) = b {
                Some(TimeWithReference {
                    value: a.value + b.value,
                    references: join_references(&a.references, Some(&b.references), None),
                })
            } else {
                None
            }
        } else {
            None
        }
    };
    operate_on_arrays(parentInstances, value, operate)
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
                let mut parent = None;

                let instance_end = instance.end.unwrap_or(Time::MAX);

                for p in parent_instances {
                    let p_end = p.end.unwrap_or(Time::MAX);
                    // TODO - could this not be achieved by instance.start <= p.end && instance.end >= p.start ?
                    if (instance.start >= p.start && instance.start < p_end)
                        || (instance.start < p.start && instance_end > p_end)
                    {
                        if let Some(old_parent) = parent {
                            if p_end > old_parent.end.unwrap_or(Time::MAX) {
                                parent = some(p);
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
                        setInstanceStartTime(&mut instance2, parent.start)
                    }

                    return_instances.push(instance2);
                }
            }

            return_instances
        }
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