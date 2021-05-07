use crate::api::ResolvedTimeline;
use crate::instance::ResolvedTimelineObjectInner;
use crate::instance::ResolvedTimelineObjectInstances;
use crate::instance::TimelineObjectInstance;
use crate::instance::TimelineObjectResolveInfo;
use crate::instance::TimelineObjectResolveStatus;
use crate::instance::{
    ResolvedTimelineObjectEntry, ResolvedTimelineObjectInstance,
    ResolvedTimelineObjectInstanceKeyframe, TimelineEnable,
};
use crate::util::set_instance_end_time;
use crate::util::Time;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::RwLock;

#[derive(PartialEq, Debug, Clone)]
pub enum EventType {
    Start = 0,
    End = 1,
    KeyFrame = 2,
}

#[derive(Debug, Clone)]
pub struct NextEvent {
    pub event_type: EventType,
    pub time: Time,
    pub object_id: String,
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
    /** If set to true, the resolver will go through the instances of the objects and fix collisions, so that the instances more closely resembles the end state. */
    pub resolve_instance_collisions: bool, // /** A cache thet is to persist data between resolves. If provided, will increase performance of resolving when only making small changes to the timeline. */
                                           // cache?: ResolverCache
}

pub struct ResolvedTimelineObject {
    pub object_id: String,
    pub object_enable: Vec<TimelineEnable>,
    pub object_priority: u64,
    pub object_disabled: bool,
    pub object_layer: String,

    // pub object: Box<dyn IsTimelineObject>,
    pub resolved: RwLock<TimelineObjectResolveStatus>,
    pub info: TimelineObjectResolveInfo,
}
impl ResolvedTimelineObject {
    pub fn is_self_referencing(&self) -> bool {
        let locked = self.resolved.read().unwrap(); // TODO - handle error
        locked.is_self_referencing()
    }
    pub fn is_resolved(&self) -> bool {
        let locked = self.resolved.read().unwrap(); // TODO - handle error
        match &*locked {
            TimelineObjectResolveStatus::Complete(_) => true,
            TimelineObjectResolveStatus::InProgress(_) => false,
            TimelineObjectResolveStatus::Pending => false,
        }
    }
}

pub type AllStates = HashMap<String, HashMap<Time, Vec<ResolvedTimelineObjectEntry>>>;
pub type StateInTime = HashMap<String, Rc<ResolvedTimelineObjectInstance>>;

pub struct ResolvedStates {
    // pub timeline: ResolvedTimeline, // TODO - is this necessary?
    pub state: AllStates,
    pub next_events: Vec<NextEvent>,

    // TODO - some of these below are excessive and
    /** Map of all objects on timeline */
    pub objects: HashMap<String, ResolvedTimelineObjectInstances>,
    /** Map of all classes on timeline, maps className to object ids */
    pub classes: HashMap<String, Vec<String>>,
    /** Map of the object ids, per layer */
    pub layers: HashMap<String, Vec<String>>,
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

    for layer_id in resolved.layers.keys() {
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
                                    if let Some(parent_id) = &keyframe.instance.inner.info.parent_id
                                    {
                                        if parent_id.eq(&state.instance.id) {
                                            if keyframe.keyframe_end_time.unwrap_or(Time::MAX)
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
    _instance: &mut ResolvedTimelineObjectInstance,
    _keyframe: &ResolvedTimelineObjectInstanceKeyframe,
) {
    // TODO
    // Something maybe here, ideally this shouldnt deal with content, and should simply report the presence of the keyframe for the consumer to do the content mangling
}

// -------

pub enum ResolvedStatesError {
    //
}

struct PointInTime {
    // object_id: String,
    // instance_id: String,
    /** if the instance turns on or off at this point */
    enable: bool,

    obj: Rc<ResolvedTimelineObjectInstance>,
}

pub fn resolve_states(
    resolved: &ResolvedTimeline,
    only_for_time: Option<Time>,
) -> Result<ResolvedStates, ResolvedStatesError> {
    // if (
    // 	cache &&
    // 	!onlyForTime &&
    // 	resolved.statistics.resolvingCount === 0 &&
    // 	cache.resolvedStates
    // ) {
    // 	// Nothing has changed since last time, just return the states right away:
    // 	return cache.resolvedStates
    // }

    // TODO - we should do some work on the 'input' data, as having the objects/instances wrapped in Rc<> and inside HashMaps will help with performance

    let resolved_objects = {
        let mut vals: Vec<&ResolvedTimelineObject> = resolved.objects.values().collect();
        // Sort to make sure parent groups are evaluated before their children:
        vals.sort_by(|a, b| {
            if a.info.depth > b.info.depth {
                Ordering::Greater
            } else if a.info.depth < b.info.depth {
                Ordering::Less
            } else {
                // id is enough
                b.object_id.cmp(&a.object_id)
            }
        });
        vals
    };

    // Step 1: Collect all points-of-interest (which points in time we want to evaluate)
    // and which instances that are interesting
    let mut points_in_time: HashMap<Time, Vec<PointInTime>> = HashMap::new();

    let mut add_point_in_time =
        |time: Time,
         enable: bool,
         inner_obj: &Rc<ResolvedTimelineObjectInstance>|
        //  obj: &ResolvedTimelineObject,
        //  instance: &TimelineObjectInstance| 
         {
            let new_point = PointInTime {
                enable,
                obj: inner_obj.clone(),
            };

            if let Some(current) = points_in_time.get_mut(&time) {
                current.push(new_point);
            } else {
                points_in_time.insert(time, vec![new_point]);
            }
        };

    for obj in resolved_objects {
        if !obj.object_disabled && obj.object_layer.len() > 0 && !obj.info.is_keyframe {
            let locked = obj.resolved.read().unwrap(); // TODO - handle error
            match &*locked {
                TimelineObjectResolveStatus::Pending => {} // do nothing
                TimelineObjectResolveStatus::InProgress(_) => {} // do nothing
                TimelineObjectResolveStatus::Complete(res) => {
                    for instance in &res.instances {
                        let use_instance = {
                            if let Some(only_for_time) = only_for_time {
                                instance.start <= only_for_time
                                    && instance.end.unwrap_or(Time::MAX) > only_for_time
                            } else {
                                true
                            }
                        };

                        if use_instance {
                            let mut time_events = Vec::new();

                            time_events.push(TimeEvent {
                                time: instance.start,
                                enable: true,
                            });
                            if let Some(end) = instance.end {
                                time_events.push(TimeEvent {
                                    time: end,
                                    enable: false,
                                });
                            }

                            // Also include times from parents, as they could affect the state of this instance:
                            let parent_times = get_times_from_parent(resolved, obj);
                            for parent_time in parent_times {
                                if parent_time.time > instance.start
                                    && parent_time.time < instance.end.unwrap_or(Time::MAX)
                                {
                                    time_events.push(parent_time)
                                }
                            }

                            let inner_obj = make_resolved_obj(obj, instance);

                            // Save a reference to this instance on all points in time that could affect it:
                            for time_event in time_events {
                                add_point_in_time(time_event.time, time_event.enable, &inner_obj);
                            }
                        }
                    }
                }
            };
        }
    }

    // Also add keyframes to pointsInTime:
    for obj in resolved_objects {
        if !obj.object_disabled
            && obj.object_layer.len() > 0
            && obj.info.is_keyframe
            && obj.info.parent_id.is_some()
        {
            // TODO - should this check the layer for being empty?
            let locked = obj.resolved.read().unwrap(); // TODO - handle error
            match &*locked {
                TimelineObjectResolveStatus::Pending => {} // do nothing
                TimelineObjectResolveStatus::InProgress(_) => {} // do nothing
                TimelineObjectResolveStatus::Complete(res) => {
                    for instance in &res.instances {
                        let inner_obj = make_resolved_obj(obj, instance);

                        // Keyframe start time
                        add_point_in_time(instance.start, true, &inner_obj);

                        // Keyframe end time
                        if let Some(end) = instance.end {
                            add_point_in_time(end, false, &inner_obj);
                        }
                    }
                }
            }
        }
    }

    // Step 2: Resolve the state for the points-of-interest
    // This is done by sweeping the points-of-interest chronologically,
    // determining the state for every point in time by adding & removing objects from aspiringInstances
    // Then sorting it to determine who takes precedence

    let mut current_state: StateInTime = HashMap::new();
    let mut active_object_ids = HashMap::new();
    let mut active_keyframes = HashMap::new();
    let mut active_keyframes_checked = HashSet::new();

    let event_object_times = HashMap::new();

    let mut resolved_states = ResolvedStates {
        // timeline: (),
        state: HashMap::new(),
        next_events: Vec::new(),

        objects: HashMap::new(),
        layers: HashMap::new(),
        classes: HashMap::new(),
    };

    // /** The objects in aspiringInstances  */
    let mut aspiring_instances: HashMap<String, Vec<Rc<ResolvedTimelineObjectInstance>>> =
        HashMap::new();

    let mut keyframe_events: Vec<NextEvent> = Vec::new();

    let times = {
        let mut times: Vec<u64> = points_in_time.keys().cloned().collect();
        times.sort();
        times
    };
    for time in times {
        if let Some(instances_to_check) = points_in_time.get(&time) {
            let mut checked_objects_this_time = HashSet::new();

            let instances_to_check2 = {
                let mut res: Vec<&PointInTime> = instances_to_check.iter().collect();
                res.sort_by(|a, b| {
                    // Keyframes comes first:
                    if a.obj.inner.info.is_keyframe && !b.obj.inner.info.is_keyframe {
                        return Ordering::Less;
                    }
                    if !a.obj.inner.info.is_keyframe && b.obj.inner.info.is_keyframe {
                        return Ordering::Greater;
                    }

                    // Ending events come before starting events:
                    if a.enable && !b.enable {
                        return Ordering::Greater;
                    }
                    if !a.enable && b.enable {
                        return Ordering::Less;
                    }

                    // Deeper objects (children in groups) comes later, we want to check the parent groups first:
                    if a.obj.inner.info.depth > b.obj.inner.info.depth {
                        return Ordering::Greater;
                    }
                    if a.obj.inner.info.depth < b.obj.inner.info.depth {
                        return Ordering::Less;
                    }

                    return Ordering::Equal;
                });
                res
            };

            for o in instances_to_check2 {
                let obj = &o.obj.inner;
                let instance = &o.obj.instance;

                let to_be_enabled =
                    instance.start <= time && instance.end.unwrap_or(Time::MAX) > time;

                let identifier = format!("{}_{}_{}", obj.object_id, instance.id, o.enable);
                if checked_objects_this_time.insert(identifier) {
                    // Only check each object and event-type once for every point in time
                    if !obj.info.is_keyframe {
                        // If object has a parent, only set if parent is on a layer (if layer is set for parent)
                        let to_be_enabled2 = if to_be_enabled {
                            if let Some(parent_obj) = obj
                                .info
                                .parent_id
                                .as_ref()
                                .and_then(|parent_id| resolved.objects.get(parent_id))
                            {
                                parent_obj.object_layer.len() == 0
                                    || active_object_ids.contains_key(&parent_obj.object_id)
                            } else {
                                to_be_enabled
                            }
                        } else {
                            to_be_enabled
                        };

                        let mut layer_aspiring_instances = aspiring_instances
                            .entry(obj.object_layer.clone())
                            .or_insert(Vec::new());

                        if to_be_enabled2 {
                            // The instance wants to be enabled (is starting)

                            // Add to aspiringInstances:
                            layer_aspiring_instances.push(o.obj.clone());

                            layer_aspiring_instances.sort_by(|a, b| {
                                // Determine who takes precedence:

                                // First, sort using priority
                                if a.inner.object_priority < b.inner.object_priority {
                                    return Ordering::Greater;
                                }
                                if a.inner.object_priority > b.inner.object_priority {
                                    return Ordering::Less;
                                }

                                // Then, sort using the start time
                                if a.instance.start < b.instance.start {
                                    return Ordering::Greater;
                                }
                                if a.instance.start > b.instance.start {
                                    return Ordering::Less;
                                }

                                // Last resort: sort using id:
                                return b.inner.object_id.cmp(&a.inner.object_id);
                            });
                        } else {
                            // The instance doesn't want to be enabled (is ending)

                            // Remove from aspiringInstances:
                            layer_aspiring_instances
                                .retain(|i| !i.inner.object_id.eq(&obj.object_id));
                        }

                        // Now, the one on top has the throne
                        // Update current state:
                        let current_on_top_of_layer = layer_aspiring_instances.first();
                        let prev_obj = current_state.get(&obj.object_layer);

                        let replace_old_obj =
                            if let Some(current_on_top_of_layer) = current_on_top_of_layer {
                                if let Some(prev_obj) = prev_obj {
                                    !prev_obj
                                        .inner
                                        .object_id
                                        .eq(&current_on_top_of_layer.inner.object_id)
                                        || !prev_obj
                                            .instance
                                            .id
                                            .eq(&current_on_top_of_layer.instance.id)
                                } else {
                                    true
                                }
                            } else {
                                false
                            };
                        let remove_old_obj =
                            prev_obj.is_some() && current_on_top_of_layer.is_none();

                        if replace_old_obj || remove_old_obj {
                            if let Some(prev_obj) = prev_obj {
                                // Cap the old instance, so it'll end at this point in time:
                                set_instance_end_time(&mut prev_obj.instance, time);

                                // Update activeObjIds:
                                active_object_ids.remove(&prev_obj.inner.object_id);

                                // Add to nextEvents:
                                let add_event = only_for_time
                                    .as_ref()
                                    .and_then(|t| Some(time > *t))
                                    .unwrap_or(true);
                                if add_event {
                                    resolved_states.next_events.push(NextEvent {
                                        event_type: EventType::End,
                                        time: time,
                                        object_id: prev_obj.inner.object_id.clone(),
                                    });
                                    event_object_times
                                        .insert(instance.end.unwrap_or(Time::MAX), EventType::End);
                                }
                            }
                        }
                        if replace_old_obj {
                            // Set the new object to State

                            let current_on_top_of_layer = current_on_top_of_layer.unwrap(); // TODO - eww

                            // Construct a new object clone:
                            let new_obj = resolved_states
                                .objects
                                .get(&current_on_top_of_layer.inner.object_id)
                                .unwrap_or_else(|| {
                                    // TODO - how does the object properties line up with the one we are operating on?
                                    let new_obj = ResolvedTimelineObjectInstances {
                                        inner: current_on_top_of_layer.inner.clone(),
                                        instances: Vec::new(),
                                    };

                                    resolved_states
                                        .objects
                                        .entry(new_obj.inner.object_id.clone())
                                        .or_insert(new_obj)
                                });

                            let new_instance = {
                                let mut new_instance = current_on_top_of_layer.instance.clone();
                                // We're setting new start & end times so they match up with the state:
                                new_instance.start = time;
                                new_instance.end = None;
                                new_instance.from_instance_id =
                                    Some(current_on_top_of_layer.instance.id.clone());

                                if new_instance.original_end.is_none() {
                                    new_instance.original_end =
                                        current_on_top_of_layer.instance.end;
                                }
                                if new_instance.original_start.is_none() {
                                    new_instance.original_start =
                                        Some(current_on_top_of_layer.instance.start);
                                }

                                // Make the instance id unique:
                                for instance in &new_obj.instances {
                                    if instance.id.eq(&new_instance.id) {
                                        new_instance.id = format!(
                                            "{}_${}",
                                            new_instance.id,
                                            new_obj.instances.len()
                                        );
                                    }
                                }

                                new_instance
                            };
                            new_obj.instances.push(new_instance.clone());

                            let new_obj_instance = Rc::new(ResolvedTimelineObjectInstance {
                                inner: new_obj.inner.clone(),
                                instance: new_instance.clone(),
                            });

                            // Save to current state:
                            current_state.insert(
                                new_obj.inner.object_layer.clone(),
                                new_obj_instance.clone(),
                            );

                            // Update activeObjIds:
                            active_object_ids.insert(
                                new_obj_instance.inner.object_id.clone(),
                                new_obj_instance.clone(),
                            );

                            // Update the tracking state as well:
                            set_state_at_time(
                                &resolved_states.state,
                                &new_obj.inner.object_layer,
                                time,
                                Some(&new_obj_instance),
                            );

                            // Add to nextEvents:
                            if new_instance.start > only_for_time.unwrap_or(0) {
                                resolved_states.next_events.push(NextEvent {
                                    event_type: EventType::Start,
                                    time: new_instance.start,
                                    object_id: obj.object_id.clone(),
                                });
                                event_object_times.insert(new_instance.start, EventType::Start);
                            }
                        } else if remove_old_obj {
                            // Remove from current state:
                            current_state.remove(&obj.object_layer);

                            // Update the tracking state as well:
                            set_state_at_time(
                                &resolved_states.state,
                                &obj.object_layer,
                                time,
                                None,
                            );
                        }
                    } else {
                        // Is a keyframe

                        // Add keyframe to resolvedStates.objects:
                        resolved_states
                            .objects
                            .insert(obj.object_id.clone(), obj.clone());

                        if to_be_enabled {
                            active_keyframes.insert(obj.object_id.clone(), o.obj.clone());
                        } else {
                            active_keyframes.remove(&obj.object_id);
                            active_keyframes_checked.remove(&obj.object_id);
                        }
                    }
                }
            }

            for (obj_id, obj_instance) in &active_keyframes {
                let keyframe = &obj_instance;
                let instance = &keyframe.instance;

                let mut unhandled = true;

                let parent_obj = keyframe
                    .inner
                    .info
                    .parent_id
                    .as_ref()
                    .and_then(|parent_id| active_object_ids.get(parent_id));
                if let Some(parent_obj) = parent_obj {
                    if parent_obj.inner.object_layer.len() > 0 {
                        // keyframe is on an active object
                        if let Some(parent_obj_instance) =
                            current_state.get(&parent_obj.inner.object_layer)
                        {
                            if active_keyframes_checked.insert(obj_id.clone()) {
                                // hasn't started before
                                let keyframe_instance =
                                    Rc::new(ResolvedTimelineObjectInstanceKeyframe {
                                        instance: ResolvedTimelineObjectInstance {
                                            inner: keyframe.inner.clone(),
                                            instance: instance.clone(),
                                        },
                                        keyframe_end_time: instance.end,
                                    });

                                // Add keyframe to the tracking state:
                                add_keyframe_at_time(
                                    &resolved_states.state,
                                    &parent_obj.inner.object_layer,
                                    time,
                                    keyframe_instance,
                                );

                                // Add keyframe to nextEvents:
                                keyframe_events.push(NextEvent {
                                    event_type: EventType::KeyFrame,
                                    time: instance.start,
                                    object_id: keyframe.inner.object_id.clone(),
                                });

                                if let Some(end) = instance.end {
                                    if parent_obj_instance
                                        .instance
                                        .end
                                        .as_ref()
                                        .and_then(|p_end| Some(end < *p_end))
                                        .unwrap_or(true)
                                    // Only add the keyframe if it ends before its parent
                                    {
                                        keyframe_events.push(NextEvent {
                                            event_type: EventType::KeyFrame,
                                            time: end,
                                            object_id: keyframe.inner.object_id.clone(),
                                        })
                                    }
                                }
                            } else {
                                unhandled = false;
                            }
                        }
                    }
                }

                if unhandled {
                    active_keyframes_checked.remove(&obj_id);
                }
            }
        }
    }

    // Go through the keyframe events and add them to nextEvents:
    for event in keyframe_events {
        if !event_object_times.contains_key(&event.time) {
            // no need to put a keyframe event if there's already another event there
            event_object_times.insert(event.time, EventType::KeyFrame);
            resolved_states.next_events.push(event);
        }
    }

    if let Some(only_for_time) = only_for_time {
        resolved_states
            .next_events
            .retain(|e| e.time > only_for_time);
    }
    resolved_states.next_events.sort_by(|a, b| {
        if a.time > b.time {
            return Ordering::Greater;
        }
        if a.time < b.time {
            return Ordering::Less;
        }

        if a.event_type > b.event_type {
            return Ordering::Less;
        }
        if a.event_type < b.event_type {
            return Ordering::Greater;
        }

        return b.object_id.cmp(&a.object_id);
    });

    // if (cache && !onlyForTime) {
    // 	cache.resolvedStates = resolvedStates
    // }

    Ok(resolved_states)
}

struct TimeEvent {
    time: u64,
    enable: bool,
}

fn get_times_from_parent(
    resolved: &ResolvedTimeline,
    obj: &ResolvedTimelineObject,
) -> Vec<TimeEvent> {
    let mut times = Vec::new();

    if let Some(parent_id) = &obj.info.parent_id {
        if let Some(parent_obj) = resolved.objects.get(parent_id) {
            let locked = parent_obj.resolved.read().unwrap(); // TODO - handle error
            match &*locked {
                TimelineObjectResolveStatus::Complete(res) => {
                    for instance in res.instances {
                        times.push(TimeEvent {
                            time: instance.start,
                            enable: true,
                        });
                        if let Some(end) = instance.end {
                            times.push(TimeEvent {
                                time: end,
                                enable: false,
                            });
                        }
                    }

                    times.extend(get_times_from_parent(resolved, parent_obj));
                }
                TimelineObjectResolveStatus::InProgress(_) => {}
                TimelineObjectResolveStatus::Pending => {}
            };
        }
    }

    times
}

fn make_resolved_obj(
    obj: &ResolvedTimelineObject,
    instance: &TimelineObjectInstance,
) -> Rc<ResolvedTimelineObjectInstance> {
    Rc::new(ResolvedTimelineObjectInstance {
        inner: ResolvedTimelineObjectInner {
            object_id: obj.object_id.clone(),
            // object_enable: Vec<TimelineEnable>,
            object_disabled: obj.object_disabled,
            object_priority: obj.object_priority,
            object_layer: obj.object_layer.clone(),

            info: obj.info.clone(),
        },
        instance: instance.clone(),
    })
}

fn set_state_at_time(
    states: &AllStates,
    layer: &String,
    time: Time,
    instance: Option<&Rc<ResolvedTimelineObjectInstance>>,
) {
    let mut layer_states = states.entry(layer.clone()).or_default();
    layer_states.insert(
        time,
        instance
            .and_then(|i| Some(vec![i.clone()]))
            .unwrap_or_default(),
    );
}
