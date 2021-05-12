use crate::api::ResolveOptions;
use crate::api::DEFAULT_LIMIT_COUNT;
use crate::caps::{Cap, CapsBuilder};
use crate::events::{EventForInstance, EventForInstanceExt};
use crate::instance::TimelineObjectInstance;
use crate::lookup_expression::LookupExpressionResultType;
use crate::references::ReferencesBuilder;
use crate::resolver::ResolverContext;
use crate::resolver::TimeWithReference;
use std::cmp::{max, min};
use std::collections::HashSet;

pub type Time = u64;

pub fn invert_instances(
    ctx: &ResolverContext,
    instances: &[TimelineObjectInstance],
) -> Vec<TimelineObjectInstance> {
    if instances.is_empty() {
        vec![TimelineObjectInstance {
            id: ctx.generate_id(),
            is_first: true,
            start: 0,
            end: None,
            references: HashSet::new(),

            original_start: None,
            original_end: None,

            caps: Vec::new(),
            from_instance_id: None,
        }]
    } else {
        let cleaned_instances = clean_instances(ctx, instances, true, true);

        let mut inverted_instances = Vec::new();

        let first_instance = &cleaned_instances[0];

        // Fill the time between the first and zero
        if first_instance.start != 0 {
            inverted_instances.push(TimelineObjectInstance {
                id: ctx.generate_id(),
                is_first: true,
                start: 0,
                end: None,
                references: ReferencesBuilder::new()
                    .add(&first_instance.references)
                    .add_id(&first_instance.id)
                    .done(),
                caps: Vec::new(),

                // TODO - what are these for if they arent set here?
                original_start: None,
                original_end: None,
                from_instance_id: None,
            });
        }

        // Fill in between the instances
        for instance in cleaned_instances {
            if let Some(prev_instance) = inverted_instances.last_mut() {
                prev_instance.end = Some(instance.start);
            }

            if let Some(end) = instance.end {
                inverted_instances.push(TimelineObjectInstance {
                    id: ctx.generate_id(),
                    is_first: false,
                    start: end,
                    end: None,
                    references: ReferencesBuilder::new()
                        .add(&instance.references)
                        .add_id(&instance.id)
                        .done(),
                    caps: instance.caps,

                    // TODO - what are these for if they arent set here?
                    original_start: None,
                    original_end: None,
                    from_instance_id: None,
                });
            }
        }

        inverted_instances
    }
}

// Cleanup instances. Join overlaps or touching etc
pub fn clean_instances(
    ctx: &ResolverContext,
    instances: &[TimelineObjectInstance],
    allow_merge: bool,
    allow_zero_gaps: bool,
) -> Vec<TimelineObjectInstance> {
    match instances.len() {
        0 => Vec::new(),
        1 => {
            let mut instance = instances[0].clone();
            instance.original_start = Some(instance.start);
            instance.original_end = instance.end;

            vec![instance]
        }
        _ => {
            let mut events = Vec::new();

            for instance in instances {
                events.push(EventForInstance {
                    time: instance.start,
                    is_start: true,
                    references: instance.references.clone(),
                    caps: instance.caps.clone(),
                    id: instance.id.clone(),
                });

                if let Some(end) = instance.end {
                    events.push(EventForInstance {
                        time: end,
                        is_start: false,
                        references: instance.references.clone(),
                        caps: instance.caps.clone(),
                        id: instance.id.clone(),
                    });
                }
            }

            events.into_instances(ctx, allow_merge, allow_zero_gaps)
        }
    }
}

pub fn add_caps_to_resuming(instance: &mut TimelineObjectInstance, caps: &[Cap]) {
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

    instance.caps = CapsBuilder::new()
        .add(instance.caps.iter().cloned())
        .add(new_caps.into_iter())
        .done();
}

fn get_converted_array_to_operate(
    a: &LookupExpressionResultType,
) -> Option<Vec<TimelineObjectInstance>> {
    match a {
        LookupExpressionResultType::Null => None,
        LookupExpressionResultType::TimeRef(time_ref) => Some(vec![TimelineObjectInstance {
            id: "".to_string(),
            start: time_ref.value,
            end: Some(time_ref.value),
            references: time_ref.references.clone(),

            is_first: false,
            original_start: None,
            original_end: None,
            caps: vec![],
            from_instance_id: None,
        }]),
        LookupExpressionResultType::Instances(_) => None,
    }
}
fn get_existing_array_to_operate(
    a: &LookupExpressionResultType,
) -> Option<&Vec<TimelineObjectInstance>> {
    match a {
        LookupExpressionResultType::Null => None,
        LookupExpressionResultType::TimeRef(_) => None,
        LookupExpressionResultType::Instances(instances) => Some(instances),
    }
}

pub fn operate_on_arrays<T>(
    ctx: &ResolverContext,
    lookup0: &LookupExpressionResultType,
    lookup1: &LookupExpressionResultType,
    operate: &T,
) -> LookupExpressionResultType
where
    T: Fn(Option<&TimeWithReference>, Option<&TimeWithReference>) -> Option<TimeWithReference>,
{
    let lookup0_converted = get_converted_array_to_operate(lookup0);
    let lookup0_orig = get_existing_array_to_operate(lookup0);
    if let Some(lookup0) = lookup0_orig.or_else(|| lookup0_converted.as_ref()) {
        let lookup1_converted = get_converted_array_to_operate(lookup1);
        let lookup1_orig = get_existing_array_to_operate(lookup1);

        if let Some(lookup1) = lookup1_orig.or_else(|| lookup1_converted.as_ref()) {
            // TODO - both refs shortcut
            // if (
            //     isReference(array0) &&
            //         isReference(array1)
            // ) {
            //     return operate(array0, array1)
            // }

            let mut result = Vec::new();

            let lookup0_len = if let Some(l) = lookup0_orig {
                l.len()
            } else {
                usize::MAX
            };
            let lookup1_len = if let Some(l) = lookup1_orig {
                l.len()
            } else {
                usize::MAX
            };

            let min_length = if lookup0_len == usize::MAX && lookup1_len == usize::MAX {
                1
            } else {
                min(lookup0_len, lookup1_len)
            };

            // let min_length = min(lookup0.len(), lookup1.len());
            // Iterate through both until we run out of one
            for i in 0..min_length {
                let a = lookup0.get(i).or_else(|| lookup0.get(0));
                let b = lookup1.get(i).or_else(|| lookup1.get(0));
                if let Some(a) = a {
                    if let Some(b) = b {
                        let start = if a.is_first {
                            Some(TimeWithReference {
                                value: a.start,
                                references: a.references.clone(),
                            })
                        } else if b.is_first {
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
                            let end = if a.is_first {
                                a.end.map(|end| TimeWithReference {
                                    value: end,
                                    references: a.references.clone(),
                                })
                            } else if b.is_first {
                                b.end.map(|end| TimeWithReference {
                                    value: end,
                                    references: b.references.clone(),
                                })
                            } else {
                                let a_end = a.end.map(|end| TimeWithReference {
                                    value: end,
                                    references: ReferencesBuilder::new()
                                        .add(&a.references)
                                        .add_id(&a.id)
                                        .done(),
                                });
                                let b_end = b.end.map(|end| TimeWithReference {
                                    value: end,
                                    references: ReferencesBuilder::new()
                                        .add(&b.references)
                                        .add_id(&b.id)
                                        .done(),
                                });

                                operate(a_end.as_ref(), b_end.as_ref())
                            };

                            result.push(TimelineObjectInstance {
                                id: ctx.generate_id(),
                                start: start.value,
                                end: end.as_ref().map(|e| e.value),
                                references: ReferencesBuilder::new()
                                    .add(&start.references)
                                    .add_some2(end.map(|e| e.references))
                                    .done(),
                                caps: CapsBuilder::new()
                                    .add(a.caps.iter().cloned())
                                    .add(b.caps.iter().cloned())
                                    .done(),

                                is_first: false,
                                original_start: None,
                                original_end: None,
                                from_instance_id: None,
                            })
                        }
                    }
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
    ctx: &ResolverContext,
    instances: Vec<TimelineObjectInstance>,
    repeat_time: Option<TimeWithReference>,
    options: &ResolveOptions,
) -> Vec<TimelineObjectInstance> {
    if let Some(repeat_time) = &repeat_time {
        if repeat_time.value != 0 {
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

        for instance in &instances {
            // TODO - fix this maths hack
            let mut start_time = max(
                (options.time as i128
                    - (((options.time as i128) - instance.start as i128)
                        % repeat_time.value as i128)) as u64,
                instance.start,
            );
            let mut end_time = instance.end.map(|end| end + (start_time - instance.start));

            let cap = instance
                .caps
                .iter()
                .find(|cap| instance.references.contains(&cap.id));

            let limit = options.limit_count.unwrap_or(DEFAULT_LIMIT_COUNT);
            for _i in 0..limit {
                if let Some(limit_time) = options.limit_time {
                    if start_time >= limit_time {
                        break;
                    }
                }

                let capped_start_time = cap
                    .map(|cap| max(cap.start, start_time))
                    .unwrap_or(start_time);
                let capped_end_time = if let Some(end_time) = end_time {
                    Some(
                        cap.and_then(|cap| cap.end)
                            .map(|cap_end| min(cap_end, end_time))
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
                        id: ctx.generate_id(),
                        start: capped_start_time,
                        end: capped_end_time,
                        references,

                        is_first: false,
                        original_start: None,
                        original_end: None,
                        caps: Vec::new(),
                        from_instance_id: None,
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
        instances
    }
} else {
    instances
}
}

pub fn apply_parent_instances(
    ctx: &ResolverContext,
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

pub fn cap_instance(
    instance: &TimelineObjectInstance,
    parent_instances: &[&TimelineObjectInstance],
) -> Option<TimelineObjectInstance> {
    let mut parent: Option<&TimelineObjectInstance> = None;

    let instance_end = instance.end.unwrap_or(Time::MAX);

    for p in parent_instances {
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
    if instance.original_end.is_none() {
        instance.original_end = instance.end;
    }

    instance.end = Some(end);
}
pub fn set_instance_start_time(instance: &mut TimelineObjectInstance, start: Time) {
    if instance.original_start.is_none() {
        instance.original_start = Some(instance.start);
    }

    instance.start = start;
}
