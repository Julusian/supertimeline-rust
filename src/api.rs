use crate::caps::Cap;
use crate::events::{EventForInstance, EventForInstanceExt};
use crate::expression::{interpret_expression, is_constant, simplify_expression, Expression};
use crate::instance::TimelineObjectResolveInfo;
use crate::instance::TimelineObjectResolvedWip;
use crate::instance::{TimelineEnable, TimelineObjectResolved};
use crate::instance::{TimelineObjectInstance, TimelineObjectResolveStatus};
use crate::lookup_expression::lookup_expression;
use crate::lookup_expression::LookupExpressionResultType;
use crate::references::ReferencesBuilder;
use crate::resolver::ObjectRefType;
use crate::resolver::ResolveError;
use crate::resolver::TimeWithReference;
use crate::state::{ResolveOptions, ResolvedTimelineObject};
use crate::util::apply_parent_instances;
use crate::util::cap_instance;
use crate::util::{apply_repeating_instances, Time};
use std::cmp::min;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
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

fn add_object_to_resolved_timeline(
    timeline: &mut ResolvedTimeline,
    obj: ResolvedTimelineObject,
    raw_obj: Option<&Box<dyn IsTimelineObject>>,
) {
    let obj_id = obj.object_id.to_string();

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
        if obj_layer.len() > 0 {
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
    timeline.objects.insert(obj_id, obj);
}

fn add_object_to_timeline(
    timeline: &mut ResolvedTimeline,
    obj: &Box<dyn IsTimelineObject>,
    depth: usize,
    parent_id: Option<&String>,
) {
    // TODO - duplicate id check
    // if (resolvedTimeline.objects[obj.id]) throw Error(`All timelineObjects must be unique! (duplicate: "${obj.id}")`)

    let resolved_obj = ResolvedTimelineObject {
        object_id: obj.id().to_string(),
        object_enable: obj.enable().clone(),
        resolved: RwLock::new(TimelineObjectResolveStatus::Pending),
        info: TimelineObjectResolveInfo {
            depth: depth,
            parent_id: parent_id.cloned(),
            is_keyframe: false,
        },
    };

    // track child objects
    if let Some(children) = obj.children() {
        for child in children {
            add_object_to_timeline(timeline, child, depth + 1, Some(&resolved_obj.object_id));
        }
    }

    // track keyframes
    if let Some(keyframes) = obj.keyframes() {
        for keyframe in keyframes {
            let resolved_obj = ResolvedTimelineObject {
                object_id: keyframe.id().to_string(),
                object_enable: keyframe.enable().clone(),
                resolved: RwLock::new(TimelineObjectResolveStatus::Pending),
                info: TimelineObjectResolveInfo {
                    depth: depth + 1,
                    parent_id: Some(resolved_obj.object_id.clone()),
                    is_keyframe: true,
                },
            };
            add_object_to_resolved_timeline(timeline, resolved_obj, None)
        }
    }

    add_object_to_resolved_timeline(timeline, resolved_obj, Some(obj));
}

pub trait ResolverContext {
    fn generate_id(&self) -> String;

    fn resolve_object(&self, obj: &ResolvedTimelineObject) -> Result<(), ResolveError>;

    fn get_object(&self, id: &str) -> Option<&ResolvedTimelineObject>;

    fn get_object_ids_for_class(&self, class: &str) -> Option<&Vec<String>>;
    fn get_object_ids_for_layer(&self, layer: &str) -> Option<&Vec<String>>;
}

// TODO - this should be split into a result and a context
pub struct ResolvedTimeline {
    pub options: ResolveOptions,
    /** Map of all objects on timeline */
    objects: HashMap<String, ResolvedTimelineObject>,
    /** Map of all classes on timeline, maps className to object ids */
    classes: HashMap<String, Vec<String>>,
    /** Map of the object ids, per layer */
    pub layers: HashMap<String, Vec<String>>,
    // pub statistics: ResolveStatistics,
    next_id: AtomicUsize,
}

pub fn resolve_timeline(
    timeline: Vec<Box<dyn IsTimelineObject>>,
    options: ResolveOptions,
) -> Result<Box<ResolvedTimeline>, ResolveError> {
    let mut resolved_timeline = ResolvedTimeline {
        objects: HashMap::new(),
        classes: HashMap::new(),
        layers: HashMap::new(),
        options,

        next_id: AtomicUsize::new(0),
    };

    // Step 1: pre-populate resolvedTimeline with objects
    for obj in timeline {
        add_object_to_timeline(&mut resolved_timeline, &obj, 0, None);
    }

    let resolved_timeline2 = Box::new(resolved_timeline);

    // Step 2: go though and resolve the objects
    // TODO - support cache
    for obj in resolved_timeline2.objects.iter() {
        // TODO - the immutability here will cause me nightmares
        resolved_timeline2.resolve_object(obj.1)?;
    }

    Ok(resolved_timeline2)
}

impl ResolverContext for ResolvedTimeline {
    fn generate_id(&self) -> String {
        let index = self.next_id.fetch_add(1, Ordering::Relaxed);
        format!("{}", index)
    }

    fn get_object(&self, id: &str) -> Option<&ResolvedTimelineObject> {
        self.objects.get(id)
    }

    fn get_object_ids_for_class(&self, class: &str) -> Option<&Vec<String>> {
        self.classes.get(class)
    }
    fn get_object_ids_for_layer(&self, layer: &str) -> Option<&Vec<String>> {
        self.layers.get(layer)
    }

    fn resolve_object(&self, obj: &ResolvedTimelineObject) -> Result<(), ResolveError> {
        {
            let mut current_status = obj.resolved.write().unwrap(); // TODO - handle error
            match &mut *current_status {
                TimelineObjectResolveStatus::Complete(_) => {
                    // Already resolved
                    return Ok(());
                }
                TimelineObjectResolveStatus::InProgress(progress) => {
                    // In progress means we hit a circular route
                    // TODO - this will need to track callers/something when threading
                    progress.is_self_referencing = true;

                    return Err(ResolveError::CircularDependency(obj.object_id.to_string()));
                }
                TimelineObjectResolveStatus::Pending => {
                    // Mark it as in progress, and release the lock
                    *current_status =
                        TimelineObjectResolveStatus::InProgress(TimelineObjectResolvedWip {
                            is_self_referencing: false,
                        });
                }
            };
        }

        // Start resolving
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

            let looked_up_repeating =
                lookup_expression(self, &obj, &repeating_expr, &ObjectRefType::Duration);
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
                enable
                    .enable_while
                    .as_ref()
                    .or(enable.enable_start.as_ref())
                    .unwrap_or(&Expression::Null),
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
            if let Some(parent_id) = &obj.info.parent_id {
                has_parent = true;

                let expr = Expression::String(format!(r"#{}", parent_id));
                let lookup = lookup_expression(self, &obj, &expr, &ObjectRefType::Start);
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

            let lookup_start = lookup_expression(self, &obj, &start, &ObjectRefType::Start);
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
                            i_start = i_start + 1;

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
                    let lookup_end = lookup_expression(self, &obj, &end_expr, &ObjectRefType::End);
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
                                i_end = i_end + 1;

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
                        lookup_expression(self, &obj, &duration_expr, &ObjectRefType::Duration);

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
                                    references: references,
                                    caps: vec![],
                                })
                            }
                        }
                        events.extend(new_events);
                    }
                }

                new_instances.extend(events.to_instances(self, false, false));
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
                                cap_instance(instance, &vec![referred_parent_instance]);

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
                                let capped_instance =
                                    cap_instance(instance, &vec![&parent_instance]);

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
                &new_instances,
                looked_up_repeating2,
                &self.options,
            ));
        }

        // filter out zero-length instances:
        let filtered_instances = instances
            .into_iter()
            .filter(|instance| instance.end.unwrap_or(Time::MAX) > instance.start)
            .collect();

        let mut locked_result = obj.resolved.write().unwrap(); // TODO - handle error
        match &*locked_result {
            TimelineObjectResolveStatus::Pending => {
                // Resolving hasn't been started, so something has messed up
                Err(ResolveError::ResolvedWhilePending(obj.object_id.clone()))
            }
            TimelineObjectResolveStatus::Complete(_) => {
                // Resolving has already been completed, so something has messed up
                Err(ResolveError::ResolvedWhileResolvec(obj.object_id.clone()))
            }
            TimelineObjectResolveStatus::InProgress(progress) => {
                *locked_result = TimelineObjectResolveStatus::Complete(TimelineObjectResolved {
                    is_self_referencing: progress.is_self_referencing,

                    instances: filtered_instances,
                    direct_references: direct_references,
                });

                Ok(())
            }
        }
    }
}
