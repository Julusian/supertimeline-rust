use crate::instance::{TimelineEnable, TimelineObjectResolved};
use crate::resolver::resolve_timeline_obj;
use crate::state::{ResolvedTimeline, ResolvedTimelineObject};
use std::collections::HashMap;

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

fn add_object_to_resolved_timeline(timeline: &mut ResolvedTimeline, obj: ResolvedTimelineObject) {
    let obj_id = obj.object.id().to_string();

    if let Some(classes) = obj.object.classes() {
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

    let obj_layer = obj.object.layer();
    if obj_layer.len() > 0 {
        if let Some(existing) = timeline.layers.get_mut(obj_layer) {
            existing.push(obj_id.clone());
        } else {
            timeline
                .layers
                .insert(obj_layer.to_string(), vec![obj_id.clone()]);
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
    let resolved_obj = ResolvedTimelineObject {
        object: obj.clone(), // TODO - I think we can omit the children and keyframes here and save up some potentially costly cloning
        resolved: TimelineObjectResolved {
            resolved: false,
            resolving: false,
            levelDeep: Some(depth),
            directReferences: if let Some(id) = parent_id {
                vec![id.clone()]
            } else {
                vec![]
            },
            parentId: parent_id.cloned(),
            isKeyframe: is_keyframe,
            isSelfReferencing: None,
        },
    };

    add_object_to_resolved_timeline(timeline, resolved_obj);

    let obj_id = obj.id().to_string();

    // track child objects
    // if obj.is_group() {
    if let Some(children) = obj.children() {
        for child in children {
            add_object_to_timeline(timeline, child, depth + 1, Some(&obj_id), false);
        }
    }
    // }

    // track keyframes
    if let Some(keyframes) = obj.keyframes() {
        for keyframe in keyframes {
            let keyframeExt = {
                // TODO
            };
            add_object_to_timeline(timeline, keyframeExt, depth + 1, Some(&obj_id), true);
        }
    }
}

pub fn resolve_timeline(timeline: Vec<Box<dyn IsTimelineObject>>) -> ResolvedTimeline {
    let mut resolved_timeline = ResolvedTimeline {
        objects: HashMap::new(),
        classes: HashMap::new(),
        layers: HashMap::new(),
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
