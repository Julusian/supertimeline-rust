use crate::instance::{TimelineEnable, TimelineObjectResolved};
use crate::resolver::resolve_timeline_obj;
use crate::state::{ResolveOptions, ResolvedTimeline, ResolvedTimelineObject};
use std::collections::{HashMap, HashSet};

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
    is_keyframe: bool,
) {
    // TODO - duplicate id check
    // if (resolvedTimeline.objects[obj.id]) throw Error(`All timelineObjects must be unique! (duplicate: "${obj.id}")`)

    let resolved_obj = ResolvedTimelineObject {
        object_id: obj.id().to_string(),
        object_enable: obj.enable().clone(),
        // object: obj.clone(), // TODO - I think we can omit the children and keyframes here and save up some potentially costly cloning
        resolved: TimelineObjectResolved {
            resolved: false,
            resolving: false,
            levelDeep: Some(depth),
            instances: None,
            directReferences: if let Some(id) = parent_id {
                set![id.clone()]
            } else {
                set![]
            },
            parentId: parent_id.cloned(),
            isKeyframe: is_keyframe,
            is_self_referencing: false,
        },
    };

    add_object_to_resolved_timeline(timeline, resolved_obj, Some(obj));

    // track child objects
    // if obj.is_group() {
    if let Some(children) = obj.children() {
        for child in children {
            add_object_to_timeline(
                timeline,
                child,
                depth + 1,
                Some(&resolved_obj.object_id),
                false,
            );
        }
    }
    // }

    // track keyframes
    if let Some(keyframes) = obj.keyframes() {
        for keyframe in keyframes {
            let resolved_obj = ResolvedTimelineObject {
                object_id: keyframe.id().to_string(),
                object_enable: keyframe.enable().clone(),
                resolved: TimelineObjectResolved {
                    resolved: false,
                    resolving: false,
                    levelDeep: Some(depth + 1),
                    instances: None,
                    directReferences: set![resolved_obj.object_id.clone()],
                    parentId: Some(resolved_obj.object_id.clone()),
                    isKeyframe: true,
                    is_self_referencing: false,
                },
            };
            add_object_to_resolved_timeline(timeline, resolved_obj, None)
        }
    }
}

pub fn resolve_timeline(
    timeline: Vec<Box<dyn IsTimelineObject>>,
    options: ResolveOptions,
) -> ResolvedTimeline {
    let mut resolved_timeline = ResolvedTimeline {
        objects: HashMap::new(),
        classes: HashMap::new(),
        layers: HashMap::new(),
        options,
    };

    // Step 1: pre-populate resolvedTimeline with objects
    for obj in timeline {
        add_object_to_timeline(&mut resolved_timeline, &obj, 0, None, false);
    }

    // Step 2: go though and resolve the objects
    // TODO - support cache
    for obj in resolved_timeline.objects.values() {
        // TODO - the immutability here will cause me nightmares
        resolve_timeline_obj(&resolved_timeline, obj);
    }

    resolved_timeline
}
