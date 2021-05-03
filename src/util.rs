use crate::instance::{Cap, TimelineObjectInstance};
use crate::lookup_expression::{LookupExpressionResult, LookupExpressionResultType};
use crate::resolver::TimeWithReference;
use crate::state::ResolveOptions;
use std::cmp::{max, min, Ordering};
use std::collections::{HashMap, HashSet};
use std::ops::Deref;

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
    ) -> Option<&TimeWithReference>,
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
    // TODO
    // const operate = (a: ValueWithReference | null, b: ValueWithReference | null): ValueWithReference | null => {
    //     if (a === null || b === null) return null
    //     return {
    //         value: a.value + b.value,
    //         references: joinReferences(a.references, b.references)
    //     }
    // }
    // return operateOnArrays(parentInstances, value, operate)
}
