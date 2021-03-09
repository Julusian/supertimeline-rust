use std::iter::Map;
use std::collections::HashMap;
use std::thread::current;

pub type Time = u64;

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

pub struct ResolveOptions{
    // TODO
}

pub struct ResolveStatistics{
    // TODO
}

pub struct ResolvedTimeline {
    // TODO
    pub options: ResolveOptions,
    /** Map of all objects on timeline */
    pub objects: HashMap<String, ResolvedTimelineObject>,
    /** Map of all classes on timeline, maps className to object ids */
    pub classes: HashMap<String, Vec<String>>,
    /** Map of the object ids, per layer */
    pub layers: HashMap<String, Vec<String>>,
    pub statistics: ResolveStatistics,
}

pub struct ResolvedTimelineObjectInstanceKeyframe {
    // TODO
}

pub struct ResolvedTimelineObjectInstance {
    // TODO
}

pub struct Cap {
    pub id: String, // id of the parent
    pub start: Time,
    pub end: Option<Time>,
}
pub struct TimelineObjectInstance {
    /** id of the instance (unique)  */
    pub id: String,
    /** if true, the instance starts from the beginning of time */
    pub isFirst: bool,
    /** The start time of the instance */
    pub start: Time,
    /** The end time of the instance (null = infinite) */
    pub end: Option<Time>,

    /** The original start time of the instance (if an instance is split or capped, the original start time is retained in here).
     * If undefined, fallback to .start
     */
    pub originalStart: Option<Time>,
    /** The original end time of the instance (if an instance is split or capped, the original end time is retained in here)
     * If undefined, fallback to .end
     */
    pub originalEnd: Option<Time>,

    /** array of the id of the referenced objects */
    pub references: Vec<String>,

    /** If set, tells the cap of the parent. The instance will always be capped inside this. */
    pub caps: Vec<Cap>,
    /** If the instance was generated from another instance, reference to the original */
    pub fromInstanceId: Option<String>,

}

pub struct ResolvedTimelineObject {
    pub id: String,
    // TODO

    pub isSelfReferencing: bool,
    pub resolving: bool,
    pub resolved: bool,
    pub resolved_instances: Vec<TimelineObjectInstance>, // TODO
}

pub type AllStates = HashMap<String, HashMap<Time, Vec<ResolvedTimelineObjectInstanceKeyframe>>>;
pub type StateInTime = HashMap<String, ResolvedTimelineObjectInstance>;

pub struct ResolvedStates {
    pub timeline: ResolvedTimeline,
   pub state: AllStates,
    pub nextEvents: Vec<NextEvent>,
}

pub struct TimelineState {
    pub time: Time,
    pub layers: StateInTime,
    pub nextEvents: Vec<NextEvent>,
}

pub fn getState(resolved: ResolvedStates, time: Time, event_limit: usize) -> TimelineState {
    let nextEvents = resolved.nextEvents.iter().filter(|&e| e.time > time).take(event_limit).cloned().collect::<Vec<_>>();

    let mut layers = HashMap::new();

    for (layer_id, _) in &resolved.timeline.layers {
        if let Some(state) = getStateAtTime(&resolved.state, layer_id, &time) {
            layers.insert(layer_id.clone(), state);
        }
    }

    return TimelineState {
        time,
        layers,
        nextEvents,
    }
}

fn getStateAtTime(states: &AllStates, layer_id: &str, request_time: &Time) -> Option<ResolvedTimelineObjectInstance> {
    if let Some(layer_states) = states.get(layer_id) {
        let layer_contents = {
            let mut tmp = layer_states.iter().collect::<Vec<_>>();
            tmp.sort_by_key(|&a| a.0);
            tmp
        };

        let mut best_state = None;
        let mut is_cloned = false;

        for (time, current_state_instances) in layer_contents {
            if time <= request_time {
                if current_state_instances.len() > 0 {
                    for current_state in current_state_instances {
                        // if current_state.is_keyframe {
                        //     // TODO
                        //     // const keyframe = currentState
                        //     // if (state && keyframe.resolved.parentId === state.id) {
                        //     //     if (
                        //     //         (keyframe.keyframeEndTime || Infinity) > requestTime
                        //     //     ) {
                        //     //         if (!isCloned) {
                        //     //             isCloned = true
                        //     //             state = {
                        //     //                 ...state,
                        //     //                 content: JSON.parse(JSON.stringify(state.content))
                        //     //             }
                        //     //         }
                        //     //         // Apply the keyframe on the state:
                        //     //         applyKeyframeContent(state.content, keyframe.content)
                        //     //     }
                        //     // }
                        // } else {
                        //     best_state = current_state;
                        //     is_cloned = false;
                        // }
                    }
                } else {
                    best_state = None;
                    is_cloned = false;
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