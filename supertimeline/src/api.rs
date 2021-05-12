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

pub trait IsTimelineObject<
    TChild: IsTimelineObject<TChild, TKeyframe>,
    TKeyframe: IsTimelineKeyframe,
> /*: IsTimelineObjectChildren */
{
    fn id(&self) -> &str;
    fn enable(&self) -> &Vec<TimelineEnable>;
    fn layer(&self) -> &str;
    fn keyframes(&self) -> Option<&Vec<TKeyframe>>;
    fn classes(&self) -> Option<&Vec<String>>;
    fn disabled(&self) -> bool;
    //fn is_group (&self) -> bool;
    fn children(&self) -> Option<&Vec<TChild>>;
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

fn add_object_to_resolved_timeline<
    TChild: IsTimelineObject<TChild, TKeyframe>,
    TKeyframe: IsTimelineKeyframe,
>(
    timeline: &mut ResolvedTimeline,
    resolving_objects: &mut HashMap<String, ResolvingTimelineObject>,
    obj: ResolvingTimelineObject,
    raw_obj: Option<&TChild>,
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

fn add_object_to_timeline<
    TChild: IsTimelineObject<TChild, TKeyframe>,
    TKeyframe: IsTimelineKeyframe,
>(
    timeline: &mut ResolvedTimeline,
    resolving_objects: &mut HashMap<String, ResolvingTimelineObject>,
    obj: &TChild,
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
                child,
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
            add_object_to_resolved_timeline::<TChild, TKeyframe>(
                timeline,
                resolving_objects,
                resolved_obj,
                None,
            )
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

pub fn resolve_timeline<
    TChild: IsTimelineObject<TChild, TKeyframe>,
    TKeyframe: IsTimelineKeyframe,
>(
    timeline: &[TChild],
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
        add_object_to_timeline(&mut resolved_timeline, &mut resolving_objects, obj, 0, None);
    }

    let resolver_context = ResolverContext::create(&resolved_timeline, resolving_objects);

    // Step 2: go though and resolve the objects
    // TODO - support cache
    for obj in resolver_context.objects.iter() {
        // TODO - the immutability here will cause me nightmares
        resolver_context.resolve_object(obj.1)?;
    }

    let mut unresolved_ids = Vec::new();

    // convert the objects/instances, and verify everything resolved
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
mod test {}
