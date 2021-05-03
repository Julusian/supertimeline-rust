use crate::instance::{
    ResolvedTimelineObjectEntry, ResolvedTimelineObjectInstance,
    ResolvedTimelineObjectInstanceKeyframe, TimelineEnable, TimelineObjectResolved,
};
use crate::util::Time;
use std::collections::HashMap;

#[derive(PartialEq, Debug, Clone)]
pub enum EventType {
    Start = 0,
    End = 1,
    KeyFrame = 2,
}

#[derive(Debug, Clone)]
pub struct NextEvent {
    pub eventType: EventType,
    pub time: Time,
    pub objectId: String,
}

#[derive(Debug, Clone)]
pub struct ResolveOptions {
    /** The base time to use when resolving. Usually you want to input the current time (Date.now()) here. */
    pub time: Time,
    /** Limits the number of repeating objects in the future.
     * Defaults to 2, which means that the current one and the next will be resolved.
     */
    pub limitCount: Option<usize>,
    /** Limits the repeating objects to a time in the future */
    pub limitTime: Option<Time>,
    /** If set to true, the resolver will go through the instances of the objects and fix collisions, so that the instances more closely resembles the end state. */
    pub resolveInstanceCollisions: bool, // /** A cache thet is to persist data between resolves. If provided, will increase performance of resolving when only making small changes to the timeline. */
                                         // cache?: ResolverCache
}

/*
pub struct ResolveStatistics {
    // TODO
}
*/

pub struct ResolvedTimeline {
    // TODO
    pub options: ResolveOptions,
    /** Map of all objects on timeline */
    pub objects: HashMap<String, ResolvedTimelineObject>,
    /** Map of all classes on timeline, maps className to object ids */
    pub classes: HashMap<String, Vec<String>>,
    /** Map of the object ids, per layer */
    pub layers: HashMap<String, Vec<String>>,
    // pub statistics: ResolveStatistics,
}

pub struct ResolvedTimelineObject {
    pub object_id: String,
    pub object_enable: Vec<TimelineEnable>,
    // pub object: Box<dyn IsTimelineObject>,
    pub resolved: TimelineObjectResolved,
}

pub type AllStates = HashMap<String, HashMap<Time, Vec<ResolvedTimelineObjectEntry>>>;
pub type StateInTime = HashMap<String, ResolvedTimelineObjectInstance>;

pub struct ResolvedStates {
    pub timeline: ResolvedTimeline,
    pub state: AllStates,
    pub next_events: Vec<NextEvent>,
}

pub struct TimelineState {
    pub time: Time,
    pub layers: StateInTime,
    pub next_events: Vec<NextEvent>,
}

pub fn get_state(resolved: ResolvedStates, time: Time, event_limit: usize) -> TimelineState {
    let event_limit2 = if event_limit == 0 {
        event_limit
    } else {
        usize::MAX
    };
    let next_events = resolved
        .next_events
        .iter()
        .filter(|&e| e.time > time)
        .take(event_limit2)
        .cloned()
        .collect::<Vec<_>>();

    let mut layers = HashMap::new();

    for (layer_id, _) in &resolved.timeline.layers {
        if let Some(state) = get_state_at_time_for_layer(&resolved.state, layer_id, time) {
            layers.insert(layer_id.clone(), state);
        }
    }

    return TimelineState {
        time,
        layers,
        next_events,
    };
}

fn get_state_at_time_for_layer(
    states: &AllStates,
    layer_id: &str,
    request_time: Time,
) -> Option<ResolvedTimelineObjectInstance> {
    if let Some(layer_states) = states.get(layer_id) {
        let layer_contents = {
            let mut tmp = layer_states.iter().collect::<Vec<_>>();
            tmp.sort_by_key(|&a| a.0);
            tmp
        };

        let mut best_state: Option<ResolvedTimelineObjectInstance> = None;

        for (time, current_state_instances) in layer_contents {
            if *time <= request_time {
                if current_state_instances.len() > 0 {
                    for current_state in current_state_instances {
                        match current_state {
                            ResolvedTimelineObjectEntry::Instance(instance) => {
                                best_state = Some(instance.clone());
                            }
                            ResolvedTimelineObjectEntry::Keyframe(keyframe) => {
                                if let Some(ref mut state) = &mut best_state {
                                    if let Some(parent_id) = &keyframe.instance.resolved.parentId {
                                        if parent_id.eq(&state.instance.id) {
                                            if keyframe.keyframeEndTime.unwrap_or(Time::MAX)
                                                > request_time
                                            {
                                                // Apply the keyframe on the state:
                                                apply_keyframe_content(state, keyframe)
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    best_state = None;
                }
            } else {
                break;
            }
        }

        best_state
    } else {
        None
    }
}

fn apply_keyframe_content(
    instance: &mut ResolvedTimelineObjectInstance,
    keyframe: &ResolvedTimelineObjectInstanceKeyframe,
) {
    // TODO
}
