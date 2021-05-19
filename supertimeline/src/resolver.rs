use crate::api::ResolveOptions;
use crate::api::ResolvedTimeline;
use crate::caps::Cap;
use crate::events::{EventForInstance, EventForInstanceExt};
use crate::expression::interpret_expression;
use crate::expression::is_constant;
use crate::expression::ExpressionError;
use crate::expression::{hack_boolean_expression, simplify_expression, Expression};
use crate::instance::TimelineObjectInfo;
use crate::instance::TimelineObjectInstance;
use crate::instance::TimelineObjectResolved;
use crate::instance::TimelineObjectResolvedWip;
use crate::lookup_expression::{lookup_expression, LookupExpressionResultType};
use crate::references::ReferencesBuilder;
use crate::util::apply_parent_instances;
use crate::util::apply_repeating_instances;
use crate::util::cap_instance;
use crate::util::Time;
use core::cmp::min;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::RwLock;

#[derive(PartialEq, Debug, Clone)]
pub enum ObjectRefType {
    Start,
    End,
    Duration,
}

#[derive(Debug)]
pub struct TimeWithReference {
    pub value: Time,
    pub references: HashSet<String>,
}

#[derive(Debug, Clone)]
pub enum ResolveError {
    CircularDependency(String),
    BadExpression((String, &'static str, ExpressionError)),
    InstancesArrayNotSupported((String, &'static str)),
    ResolvedWhilePending(String),
    ResolvedWhileResolvec(String),
    UnresolvedObjects(Vec<String>),
}

pub struct ResolvingTimelineObject {
    // pub object: Box<dyn IsTimelineObject>,
    pub resolved: RwLock<TimelineObjectResolvingStatus>,
    pub info: TimelineObjectInfo,
}
impl ResolvingTimelineObject {
    pub fn is_self_referencing(&self) -> bool {
        let locked = self.resolved.read().unwrap(); // TODO - handle error
        locked.is_self_referencing()
    }
}

#[derive(Debug, Clone)]
pub enum TimelineObjectResolvingStatus {
    Pending,
    InProgress(TimelineObjectResolvedWip),
    Complete(TimelineObjectResolved),
}
impl TimelineObjectResolvingStatus {
    pub fn is_self_referencing(&self) -> bool {
        match self {
            TimelineObjectResolvingStatus::Pending => {
                // Clearly not
                false
            }
            TimelineObjectResolvingStatus::InProgress(progress) => progress.is_self_referencing,
            TimelineObjectResolvingStatus::Complete(res) => res.is_self_referencing,
        }
    }
}

// TODO - this should be split into a result and a context
pub struct ResolverContext<'a> {
    pub options: &'a ResolveOptions,
    /** Map of all objects on timeline */
    pub objects: HashMap<String, ResolvingTimelineObject>,
    /** Map of all classes on timeline, maps className to object ids */
    classes: &'a HashMap<String, Vec<String>>,
    /** Map of the object ids, per layer */
    layers: &'a HashMap<String, Vec<String>>,
    // pub statistics: ResolveStatistics,
    next_id: AtomicUsize,
}

impl<'a> ResolverContext<'a> {
    pub fn create(
        resolved_timeline: &'a ResolvedTimeline,
        objects: HashMap<String, ResolvingTimelineObject>,
    ) -> ResolverContext<'a> {
        ResolverContext {
            options: &resolved_timeline.options,
            objects,
            classes: &resolved_timeline.classes,
            layers: &resolved_timeline.layers,
            next_id: AtomicUsize::new(0),
        }
    }

    pub fn generate_id(&self) -> String {
        let index = self.next_id.fetch_add(1, Ordering::Relaxed);
        format!("@{}", index)
    }

    pub fn get_object(&self, id: &str) -> Option<&ResolvingTimelineObject> {
        self.objects.get(id)
    }

    pub fn get_object_ids_for_class(&self, class: &str) -> Option<&Vec<String>> {
        self.classes.get(class)
    }
    pub fn get_object_ids_for_layer(&self, layer: &str) -> Option<&Vec<String>> {
        self.layers.get(layer)
    }

    pub fn resolve_object(&self, obj: &ResolvingTimelineObject) -> Result<(), ResolveError> {
        {
            let mut current_status = obj.resolved.write().unwrap(); // TODO - handle error
            match &mut *current_status {
                TimelineObjectResolvingStatus::Complete(_) => {
                    // Already resolved
                    return Ok(());
                }
                TimelineObjectResolvingStatus::InProgress(progress) => {
                    // In progress means we hit a circular route
                    // TODO - this will need to track callers/something when threading
                    progress.is_self_referencing = true;

                    return Err(ResolveError::CircularDependency(obj.info.id.to_string()));
                }
                TimelineObjectResolvingStatus::Pending => {
                    // Mark it as in progress, and release the lock
                    *current_status =
                        TimelineObjectResolvingStatus::InProgress(TimelineObjectResolvedWip {
                            is_self_referencing: false,
                        });
                }
            };
        }

        // Start resolving
        let mut direct_references = HashSet::new();

        let mut instances = Vec::new();

        let obj_id = &obj.info.id;
        for enable in &obj.info.enable {
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

            let looked_up_repeating =
                lookup_expression(self, &obj, &repeating_expr, &ObjectRefType::Duration)?;
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

            let hacked_while = hack_boolean_expression(enable.enable_while.as_ref());

            let start = simplify_expression(
                hacked_while
                    .as_ref()
                    .or_else(|| enable.enable_while.as_ref())
                    .or_else(|| enable.enable_start.as_ref())
                    .unwrap_or(&Expression::Null),
            )
            .map_err(|e| ResolveError::BadExpression((obj_id.to_string(), "simplify", e)))?;

            let mut parent_instances = None;
            let mut has_parent = false;
            let mut refer_to_parent = false;
            if let Some(parent_id) = &obj.info.parent_id {
                has_parent = true;

                let expr = Expression::String(format!(r"#{}", parent_id));
                let lookup = lookup_expression(self, &obj, &expr, &ObjectRefType::Start)?;
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

            let lookup_start = lookup_expression(self, &obj, &start, &ObjectRefType::Start)?;
            direct_references.extend(lookup_start.all_references);

            let looked_up_starts = if refer_to_parent {
                apply_parent_instances(self, &parent_instances, &lookup_start.result)
            } else {
                lookup_start.result
            };

            let mut new_instances = Vec::new();

            if let Some(_enable_while) = &enable.enable_while {
                match looked_up_starts {
                    LookupExpressionResultType::Instances(instances) => new_instances = instances,
                    LookupExpressionResultType::TimeRef(time_ref) => {
                        new_instances.push(TimelineObjectInstance {
                            id: self.generate_id(),
                            start: time_ref.value,
                            end: None,
                            references: time_ref.references,

                            is_first: false,
                            original_start: None,
                            original_end: None,
                            caps: vec![],
                            from_instance_id: None,
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
                            i_start += 1;

                            events.push(EventForInstance {
                                time: instance.start,
                                is_start: true,
                                id: format!("{}_{}", obj_id, index),
                                references: instance.references.clone(),
                                caps: instance.caps.clone(),
                            })
                        }
                    }
                    LookupExpressionResultType::TimeRef(time_ref) => {
                        events.push(EventForInstance {
                            time: time_ref.value,
                            is_start: true,
                            id: format!("{}_0", obj_id),
                            references: time_ref.references.clone(),
                            caps: vec![],
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
                    let lookup_end = lookup_expression(self, &obj, &end_expr, &ObjectRefType::End)?;
                    let looked_up_ends = if refer_to_parent && is_constant(&end_expr) {
                        apply_parent_instances(self, &parent_instances, &lookup_end.result)
                    } else {
                        lookup_end.result
                    };

                    direct_references.extend(lookup_end.all_references);
                    match &looked_up_ends {
                        LookupExpressionResultType::Instances(instances) => {
                            for instance in instances {
                                let index = i_end;
                                i_end += 1;

                                events.push(EventForInstance {
                                    time: instance.start,
                                    is_start: false,
                                    id: format!("{}_{}", obj_id, index),
                                    references: instance.references.clone(),
                                    caps: instance.caps.clone(),
                                })
                            }
                        }
                        LookupExpressionResultType::TimeRef(time_ref) => {
                            events.push(EventForInstance {
                                time: time_ref.value,
                                is_start: false,
                                id: format!("{}_0", obj_id),
                                references: time_ref.references.clone(),
                                caps: vec![],
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
                    let lookup_duration =
                        lookup_expression(self, &obj, &duration_expr, &ObjectRefType::Duration)?;

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
                        for event in &events {
                            if event.is_start {
                                let references = ReferencesBuilder::new()
                                    .add(&event.references)
                                    .add(&duration2.references)
                                    .done();

                                new_events.push(EventForInstance {
                                    time: event.time + duration_val,
                                    is_start: false,
                                    id: event.id.clone(),
                                    references,
                                    caps: vec![],
                                })
                            }
                        }
                        events.extend(new_events);
                    }
                }

                new_instances.extend(events.into_instances(self, false, false));
            }

            if has_parent {
                // figure out what parent-instance the instances are tied to, and cap them
                let mut capped_instances = Vec::new();

                if let Some(parent_instances) = &parent_instances {
                    for instance in &new_instances {
                        let referred_parent_instance =
                            parent_instances.iter().find(|parent_instance| {
                                instance.references.contains(&parent_instance.id)
                            });

                        if let Some(referred_parent_instance) = referred_parent_instance {
                            // If the child refers to its parent, there should be one specific instance to cap into
                            let capped_instance =
                                cap_instance(instance, &[referred_parent_instance]);

                            if let Some(mut capped_instance) = capped_instance {
                                capped_instance.caps.push(Cap {
                                    id: referred_parent_instance.id.clone(),
                                    start: referred_parent_instance.start,
                                    end: referred_parent_instance.end,
                                });
                                capped_instances.push(capped_instance)
                            }
                        } else {
                            // If the child doesn't refer to its parent, it should be capped within all of its parent instances
                            for parent_instance in parent_instances {
                                let capped_instance = cap_instance(instance, &[&parent_instance]);

                                if let Some(mut capped_instance) = capped_instance {
                                    capped_instance.caps.push(Cap {
                                        id: parent_instance.id.clone(),
                                        start: parent_instance.start,
                                        end: parent_instance.end,
                                    });
                                    capped_instances.push(capped_instance)
                                }
                            }
                        }
                    }
                }

                new_instances = capped_instances;
            }

            instances.extend(apply_repeating_instances(
                self,
                new_instances,
                looked_up_repeating2,
                &self.options,
            ));
        }

        // filter out zero-length instances:
        instances.retain(|instance| instance.end.unwrap_or(Time::MAX) > instance.start);

        let mut locked_result = obj.resolved.write().unwrap(); // TODO - handle error
        match &*locked_result {
            TimelineObjectResolvingStatus::Pending => {
                // Resolving hasn't been started, so something has messed up
                Err(ResolveError::ResolvedWhilePending(obj.info.id.clone()))
            }
            TimelineObjectResolvingStatus::Complete(_) => {
                // Resolving has already been completed, so something has messed up
                Err(ResolveError::ResolvedWhileResolvec(obj.info.id.clone()))
            }
            TimelineObjectResolvingStatus::InProgress(progress) => {
                *locked_result = TimelineObjectResolvingStatus::Complete(TimelineObjectResolved {
                    is_self_referencing: progress.is_self_referencing,

                    instances,
                    direct_references,
                });

                Ok(())
            }
        }
    }
}
