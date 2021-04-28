use crate::util::TimelineObject;
use crate::state::{ResolvedTimeline, ResolvedTimelineObject};
use std::collections::HashMap;
use crate::instance::TimelineObjectResolved;
use crate::resolver::resolve_timeline_obj;

fn add_object_to_resolved_timeline(timeline: &mut ResolvedTimeline, obj: ResolvedTimelineObject) {
    // TODO
}

fn add_object_to_timeline(timeline: &mut ResolvedTimeline, obj: &TimelineObject, depth: usize, parent_id: Option<&String>, is_keyframe: bool) {
    let resolved_obj = ResolvedTimelineObject {
        object: obj.clone(), // TODO - I think we can omit the children and keyframes here and save up some potentially costly cloning
        resolved: TimelineObjectResolved {
            resolved: false,
            resolving: false,
            levelDeep: Some(depth),
            directReferences: if let Some(id) = parent_id { vec![id.clone()]} else { vec![] },
            parentId: parent_id.cloned(),
            isKeyframe: is_keyframe,
            isSelfReferencing: None,
        }
    };

    add_object_to_resolved_timeline(timeline, resolved_obj);

    // track child objects
    if obj.is_group {
        if let Some(children) = obj.children {
            for child in children {
                add_object_to_timeline(timeline, child, depth + 1, Some(&obj.id), false);
            }
        }
    }

    // track keyframes
    if let Some(keyframes) = obj.keyframes {
        for keyframe in keyframes {
            let keyframeExt = {
                // TODO
            };
            add_object_to_timeline(timeline, keyframeExt, depth + 1, Some(&obj.id), true);
        }
    }


}

pub fn resolve_timeline(timeline: Vec<TimelineObject>) -> ResolvedTimeline {
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
        // TODO - immutability here will cause nightmares
        resolve_timeline_obj(&resolved_timeline, obj);
    }

    resolved_timeline
}