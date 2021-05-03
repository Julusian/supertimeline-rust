use crate::api::{ResolvedTimeline, ResolverContext};
use crate::events::{EventForInstance, EventForInstanceExt};
use crate::expression::{
    interpret_expression, is_constant, simplify_expression, Expression, ExpressionError,
};
use crate::instance::TimelineObjectInstance;
use crate::lookup_expression::{lookup_expression, LookupExpressionResultType};
use crate::references::ReferencesBuilder;
use crate::state;
use crate::util::{apply_parent_instances, apply_repeating_instances, cap_instance, Time};
use std::cmp::min;
use std::collections::HashSet;

#[derive(PartialEq, Debug, Clone)]
pub enum ObjectRefType {
    Start,
    End,
    Duration,
}

pub struct TimeWithReference {
    pub value: Time,
    pub references: HashSet<String>,
}

pub enum ResolveError {
    CircularDependency(String),
    BadExpression((String, &'static str, ExpressionError)),
    InstancesArrayNotSupported((String, &'static str)),
}

pub fn resolve_timeline_obj(
    resolved_timeline: &mut ResolvedTimeline,
    obj: &mut state::ResolvedTimelineObject,
) -> Result<(), ResolveError> {
    if obj.resolved.resolved {
        Ok(())
    } else if obj.resolved.resolving {
        Err(ResolveError::CircularDependency(obj.object_id.to_string()))
    } else {
        obj.resolved.resolving = true;

        let mut direct_references = HashSet::new();

        let mut instances = Vec::new();

        let obj_id = &obj.object_id;
        for enable in &obj.object_enable {
            let repeating_expr = if let Some(expr) = &enable.repeating {
                match interpret_expression(expr) {
                    Ok(val) => val,
                    Err(err) => {
                        return Err(ResolveError::BadExpression((
                            obj_id.to_string(),
                            "repeating",
                            err,
                        )))
                    }
                }
            } else {
                Expression::Null
            };

            let looked_up_repeating = lookup_expression(
                resolved_timeline,
                obj,
                &repeating_expr,
                &ObjectRefType::Duration,
            );
            direct_references.extend(looked_up_repeating.all_references);

            let looked_up_repeating2 = match looked_up_repeating.result {
                LookupExpressionResultType::Instances(_) => {
                    return Err(ResolveError::InstancesArrayNotSupported((
                        obj_id.to_string(),
                        "repeating",
                    )))
                }
                LookupExpressionResultType::TimeRef(r) => Some(r),
                LookupExpressionResultType::Null => None,
            };

            let start = simplify_expression(
                &enable
                    .enable_while
                    .unwrap_or(enable.enable_start.unwrap_or(Expression::Null)),
            )
            .or_else(|e| {
                Err(ResolveError::BadExpression((
                    obj_id.to_string(),
                    "simplify",
                    e,
                )))
            })?;

            let mut parent_instances = None;
            let mut has_parent = false;
            let mut refer_to_parent = false;
            if let Some(parent_id) = &obj.resolved.parentId {
                has_parent = true;

                let expr = Expression::String(format!(r"#{}", parent_id));
                let lookup =
                    lookup_expression(resolved_timeline, obj, &expr, &ObjectRefType::Start);
                match lookup.result {
                    LookupExpressionResultType::TimeRef(_) => {}
                    LookupExpressionResultType::Instances(instances) => {
                        parent_instances = Some(instances);
                    }
                    LookupExpressionResultType::Null => {}
                }

                direct_references.extend(lookup.all_references);

                if is_constant(&start) {
                    // Only use parent if the expression resolves to a number (ie doesn't contain any references)
                    refer_to_parent = true;
                }
            }

            let lookup_start =
                lookup_expression(resolved_timeline, obj, &start, &ObjectRefType::Start);
            direct_references.extend(lookup_start.all_references);

            let looked_up_starts = if refer_to_parent {
                apply_parent_instances(resolved_timeline, &parent_instances, &lookup_start.result)
            } else {
                lookup_start.result
            };

            let mut new_instances = Vec::new();

            if let Some(enable_while) = &enable.enable_while {
                match looked_up_starts {
                    LookupExpressionResultType::Instances(instances) => new_instances = instances,
                    LookupExpressionResultType::TimeRef(time_ref) => {
                        new_instances.push(TimelineObjectInstance {
                            id: resolved_timeline.get_id(),
                            start: time_ref.value,
                            end: None,
                            references: time_ref.references,

                            isFirst: false,
                            originalStart: None,
                            originalEnd: None,
                            caps: vec![],
                            fromInstanceId: None,
                        })
                    }
                    LookupExpressionResultType::Null => {}
                }
            } else {
                let mut events = Vec::new();
                let mut i_start = 0;
                let mut i_end = 0;

                match &looked_up_starts {
                    LookupExpressionResultType::Instances(instances) => {
                        for instance in instances {
                            let index = i_start;
                            i_start = i_start + 1;

                            events.push(EventForInstance {
                                time: instance.start,
                                is_start: true,
                                instance,
                                id: Some(format!("{}_{}", obj_id, index)),
                                references: &instance.references,
                            })
                        }
                    }
                    LookupExpressionResultType::TimeRef(time_ref) => {
                        let index = i_start;
                        i_start = i_start + 1;

                        events.push(EventForInstance {
                            time: time_ref.value,
                            is_start: true,
                            instance: &TimelineObjectInstance {
                                id: resolved_timeline.get_id(),
                                start: time_ref.value,
                                end: None,
                                references: time_ref.references.clone(),

                                isFirst: false,
                                originalStart: None,
                                originalEnd: None,
                                caps: vec![],
                                fromInstanceId: None,
                            },
                            id: Some(format!("{}_{}", obj_id, index)),
                            references: &time_ref.references,
                        })
                    }
                    LookupExpressionResultType::Null => {}
                }

                if let Some(enable_end) = &enable.enable_end {
                    let end_expr = match interpret_expression(enable_end) {
                        Ok(val) => val,
                        Err(err) => {
                            return Err(ResolveError::BadExpression((
                                obj_id.to_string(),
                                "end",
                                err,
                            )))
                        }
                    };
                    // lookedupEnds will contain an inverted list of instances. Therefore .start means an end
                    let lookup_end =
                        lookup_expression(resolved_timeline, obj, &end_expr, &ObjectRefType::End);
                    let looked_up_ends = if refer_to_parent && is_constant(&end_expr) {
                        apply_parent_instances(
                            resolved_timeline,
                            &parent_instances,
                            &lookup_end.result,
                        )
                    } else {
                        lookup_end.result
                    };

                    direct_references.extend(lookup_end.all_references);
                    match &looked_up_ends {
                        LookupExpressionResultType::Instances(instances) => {
                            for instance in instances {
                                let index = i_end;
                                i_start = i_end + 1;

                                events.push(EventForInstance {
                                    time: instance.start,
                                    is_start: false,
                                    instance,
                                    id: Some(format!("{}_{}", obj_id, index)),
                                    references: &instance.references,
                                })
                            }
                        }
                        LookupExpressionResultType::TimeRef(time_ref) => {
                            let index = i_end;
                            i_start = i_end + 1;

                            events.push(EventForInstance {
                                time: time_ref.value,
                                is_start: false,
                                instance: &TimelineObjectInstance {
                                    id: resolved_timeline.get_id(),
                                    start: time_ref.value,
                                    end: None,
                                    references: time_ref.references.clone(),

                                    isFirst: false,
                                    originalStart: None,
                                    originalEnd: None,
                                    caps: vec![],
                                    fromInstanceId: None,
                                },
                                id: Some(format!("{}_{}", obj_id, index)),
                                references: &time_ref.references,
                            })
                        }
                        LookupExpressionResultType::Null => {}
                    }
                } else if let Some(enable_duration) = &enable.duration {
                    let duration_expr = match interpret_expression(enable_duration) {
                        Ok(val) => val,
                        Err(err) => {
                            return Err(ResolveError::BadExpression((
                                obj_id.to_string(),
                                "duration",
                                err,
                            )))
                        }
                    };
                    let lookup_duration = lookup_expression(
                        resolved_timeline,
                        obj,
                        &duration_expr,
                        &ObjectRefType::Duration,
                    );

                    direct_references.extend(lookup_duration.all_references);

                    let looked_up_duration = match lookup_duration.result {
                        LookupExpressionResultType::Instances(instances) => {
                            if instances.len() > 1 {
                                return Err(ResolveError::InstancesArrayNotSupported((
                                    obj_id.to_string(),
                                    "duration",
                                )));
                            } else if let Some(instance) = instances.get(0) {
                                Some(TimeWithReference {
                                    value: instance.start,
                                    references: instance.references.clone(),
                                })
                            } else {
                                None
                            }
                        }
                        LookupExpressionResultType::TimeRef(time_ref) => Some(time_ref),
                        LookupExpressionResultType::Null => None,
                    };

                    if let Some(duration2) = looked_up_duration {
                        let duration_val = if let Some(repeating) = &looked_up_repeating2 {
                            min(repeating.value, duration2.value)
                        } else {
                            duration2.value
                        };

                        let mut new_events = Vec::new();
                        for event in events {
                            if event.is_start {
                                let endTime = event.time + duration_val;
                                let references = ReferencesBuilder::new()
                                    .add(&event.instance.references)
                                    .add(&duration2.references)
                                    .done();

                                let index = i_end;
                                i_start = i_end + 1;

                                new_events.push(EventForInstance {
                                    time: endTime,
                                    is_start: false,
                                    instance: &TimelineObjectInstance {
                                        id: event.instance.id.clone(),
                                        start: endTime,
                                        end: None,
                                        references,

                                        isFirst: false,
                                        originalStart: None,
                                        originalEnd: None,
                                        caps: vec![],
                                        fromInstanceId: None,
                                    },
                                    id: event.id.clone(),
                                    references: &references,
                                })
                            }
                        }
                        events.extend(new_events);
                    }
                }

                new_instances.extend(events.to_instances(resolved_timeline, false, false));
            }

            if has_parent {
                // figure out what parent-instance the instances are tied to, and cap them
                let mut capped_instances = Vec::new();

                if let Some(parent_instances) = parent_instances {
                    for (i, instance) in new_instances.iter().enumerate() {
                        let referred_parent_instance =
                            parent_instances.iter().find(|parent_instance| {
                                instance.references.contains(&parent_instance.id)
                            });

                        if let Some(referred_parent_instance) = referred_parent_instance {
                            // If the child refers to its parent, there should be one specific instance to cap into
                            let capped_instance =
                                cap_instance(instance, &vec![referred_parent_instance]);

                            // TODO
                            // if (cappedInstance) {
                            //
                            //     if (!cappedInstance.caps) cappedInstance.caps = []
                            //     cappedInstance.caps.push({
                            //         id: referredParentInstance.id,
                            //         start: referredParentInstance.start,
                            //         end: referredParentInstance.end
                            //     })
                            //     cappedInstances.push(cappedInstance)
                            // }
                        } else {
                            // TODO
                            // If the child doesn't refer to its parent, it should be capped within all of its parent instances
                            // for (let i = 0; i < parentInstances.length; i++) {
                            //     const parentInstance = parentInstances[i]
                            //
                            //     const cappedInstance = capInstances([instance], [parentInstance])[0]
                            //
                            //     if (cappedInstance) {
                            //         if (parentInstance) {
                            //             if (!cappedInstance.caps) cappedInstance.caps = []
                            //             cappedInstance.caps.push({
                            //                 id: parentInstance.id,
                            //                 start: parentInstance.start,
                            //                 end: parentInstance.end
                            //             })
                            //         }
                            //         cappedInstances.push(cappedInstance)
                            //     }
                            // }
                        }
                    }
                }

                new_instances = capped_instances;
            }

            instances.extend(apply_repeating_instances(
                resolved_timeline,
                &new_instances,
                looked_up_repeating2,
                &resolved_timeline.options,
            ));
        }

        // filter out zero-length instances:
        let filtered_instances = instances
            .into_iter()
            .filter(|instance| instance.end.unwrap_or(Time::MAX) > instance.start)
            .collect();

        obj.resolved.resolved = true;
        obj.resolved.resolving = false;
        obj.resolved.instances = Some(filtered_instances);
        obj.resolved.directReferences = direct_references;

        Ok(())
    }
}
//
// fn generate_end_events() {
//
// }
