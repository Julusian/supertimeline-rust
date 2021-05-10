use crate::api::ResolvedTimeline;
use crate::instance::TimelineObjectInfo;
use crate::instance::TimelineObjectInstance;
use crate::instance::TimelineObjectResolved;
use crate::util::Time;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::HashSet;
use std::ops::Deref;
use std::rc::Rc;

#[derive(PartialEq, Debug, Clone, PartialOrd)]
pub enum EventType {
    Start = 0,
    End = 1,
    KeyFrame = 2,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NextEvent {
    pub event_type: EventType,
    pub time: Time,
    pub object_id: String,
}

pub struct ResolvedTimelineObject {
    pub resolved: TimelineObjectResolved,
    pub info: Rc<TimelineObjectInfo>,
}

pub type AllStates = HashMap<String, HashMap<Time, TimelineLayerState>>;

#[derive(Clone)]
pub struct ResolvedStatesForObject {
    pub info: Rc<TimelineObjectInfo>,
    pub instances: Vec<Rc<TimelineObjectInstance>>,
}

pub struct ResolvedStates {
    pub state: AllStates,
    pub next_events: Vec<NextEvent>,

    // TODO - some of these below are excessive and need clarifying what they now are
    /** Map of all objects on timeline */
    pub objects: HashMap<String, ResolvedStatesForObject>,
    /** Map of all classes on timeline, maps className to object ids */
    pub classes: HashMap<String, Vec<String>>,
    /** Map of the object ids, per layer */
    pub layers: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct ResolvedTimelineObjectInstanceKeyframe {
    // TODO - remove info from here, and surely the keyframe id will be needed instead?
    pub info: Rc<TimelineObjectInfo>,
    // pub instance: Rc<TimelineObjectInstance>,
    pub keyframe_end_time: Option<Time>,
}

#[derive(Debug, Clone)]
pub struct ResolvedTimelineObjectInstance {
    pub info: Rc<TimelineObjectInfo>,
    pub instance: Rc<TimelineObjectInstance>,
}

#[derive(Debug, Clone)]
pub struct TimelineLayerState {
    pub info: Rc<TimelineObjectInfo>,
    pub instance: Rc<TimelineObjectInstance>, // TODO - this is a bit heavy now?
    pub keyframes: Vec<Rc<ResolvedTimelineObjectInstanceKeyframe>>,
}

pub struct TimelineState {
    pub time: Time,
    pub layers: HashMap<String, TimelineLayerState>,
    pub next_events: Vec<NextEvent>,
}

pub fn get_state(
    resolved: &ResolvedStates,
    time: Time,
    event_limit: Option<usize>,
) -> TimelineState {
    let event_limit2 = if let Some(event_limit) = event_limit {
        if event_limit == 0 {
            usize::MAX
        } else {
            event_limit
        }
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

    TimelineState {
        time,
        layers,
        next_events,
    }
}

fn get_state_at_time_for_layer(
    states: &AllStates,
    layer_id: &str,
    request_time: Time,
) -> Option<TimelineLayerState> {
    if let Some(layer_states) = states.get(layer_id) {
        let layer_contents = {
            let mut tmp = layer_states.iter().collect::<Vec<_>>();
            tmp.sort_by_key(|&a| a.0);
            tmp
        };

        let mut best_state: Option<TimelineLayerState> = None;

        for (time, current_state_instances) in layer_contents {
            if *time <= request_time {
                let mut keyframes = current_state_instances.keyframes.clone();
                keyframes.retain(|keyframe| {
                    if let Some(parent_id) = &keyframe.info.parent_id {
                        if parent_id.eq(&current_state_instances.instance.id)
                            && keyframe.keyframe_end_time.unwrap_or(Time::MAX) > request_time
                        {
                            // Apply the keyframe on the state:
                            return true;
                        }
                    }
                    false
                });
                best_state = Some(TimelineLayerState {
                    info: current_state_instances.info.clone(),
                    instance: current_state_instances.instance.clone(),
                    keyframes,
                });
            } else {
                break;
            }
        }

        best_state
    } else {
        None
    }
}

// -------

#[derive(Debug, Clone)]
pub enum ResolvedStatesError {
    //
}

struct PointInTime {
    /** if the instance turns on or off at this point */
    enable: bool,

    obj: Rc<ResolvedTimelineObjectInstance>,
}

pub fn resolve_all_states(
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
                b.info.id.cmp(&a.info.id)
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

    for obj in &resolved_objects {
        if !obj.info.disabled && !obj.info.layer.is_empty() && !obj.info.is_keyframe {
            for instance in &obj.resolved.instances {
                let use_instance = {
                    if let Some(only_for_time) = only_for_time {
                        instance.start <= only_for_time
                            && instance.end.unwrap_or(Time::MAX) > only_for_time
                    } else {
                        true
                    }
                };

                if use_instance {
                    let mut time_events = vec![TimeEvent {
                        time: instance.start,
                        enable: true,
                    }];

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
    }

    // Also add keyframes to pointsInTime:
    for obj in &resolved_objects {
        if !obj.info.disabled
            && !obj.info.layer.is_empty()
            && obj.info.is_keyframe
            && obj.info.parent_id.is_some()
        {
            // TODO - should this check the layer for being empty?
            for instance in &obj.resolved.instances {
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

    // Step 2: Resolve the state for the points-of-interest
    // This is done by sweeping the points-of-interest chronologically,
    // determining the state for every point in time by adding & removing objects from aspiringInstances
    // Then sorting it to determine who takes precedence

    let mut current_state: HashMap<String, Rc<ResolvedTimelineObjectInstance>> = HashMap::new();
    let mut active_object_ids = HashMap::new();
    let mut active_keyframes = HashMap::new();
    let mut active_keyframes_checked = HashSet::new();

    let mut event_object_times = HashSet::new();

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

    let sorted_points_in_time = {
        let mut sorted_points_in_time: Vec<(u64, Vec<PointInTime>)> =
            points_in_time.into_iter().collect();
        sorted_points_in_time.sort_by_key(|e| e.0);
        sorted_points_in_time
    };

    for (time, instances_to_check) in sorted_points_in_time {
        let mut checked_objects_this_time = HashSet::new();

        let instances_to_check2 = {
            let mut res: Vec<&PointInTime> = instances_to_check.iter().collect();
            res.sort_by(|a, b| {
                // Keyframes comes first:
                if a.obj.info.is_keyframe && !b.obj.info.is_keyframe {
                    return Ordering::Less;
                }
                if !a.obj.info.is_keyframe && b.obj.info.is_keyframe {
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
                if a.obj.info.depth > b.obj.info.depth {
                    return Ordering::Greater;
                }
                if a.obj.info.depth < b.obj.info.depth {
                    return Ordering::Less;
                }

                Ordering::Equal
            });
            res
        };

        for o in instances_to_check2 {
            let obj = &o.obj;
            let instance = &o.obj.instance;

            let to_be_enabled = instance.start <= time && instance.end.unwrap_or(Time::MAX) > time;

            let identifier = format!("{}_{}_{}", obj.info.id, instance.id, o.enable);
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
                            parent_obj.info.layer.is_empty()
                                || active_object_ids.contains_key(&parent_obj.info.id)
                        } else {
                            to_be_enabled
                        }
                    } else {
                        to_be_enabled
                    };

                    let layer_aspiring_instances = aspiring_instances
                        .entry(obj.info.layer.clone())
                        .or_insert_with(Vec::new);

                    if to_be_enabled2 {
                        // The instance wants to be enabled (is starting)

                        // Add to aspiringInstances:
                        layer_aspiring_instances.push(o.obj.clone());

                        layer_aspiring_instances.sort_by(|a, b| {
                            // Determine who takes precedence:

                            // First, sort using priority
                            if a.info.priority < b.info.priority {
                                return Ordering::Greater;
                            }
                            if a.info.priority > b.info.priority {
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
                            b.info.id.cmp(&a.info.id)
                        });
                    } else {
                        // The instance doesn't want to be enabled (is ending)

                        // Remove from aspiringInstances:
                        layer_aspiring_instances.retain(|i| !i.info.id.eq(&obj.info.id));
                    }

                    // Now, the one on top has the throne
                    // Update current state:
                    let current_on_top_of_layer = layer_aspiring_instances.first();
                    let prev_obj = current_state.get(&obj.info.layer);

                    let replace_old_obj =
                        if let Some(current_on_top_of_layer) = current_on_top_of_layer {
                            if let Some(prev_obj) = prev_obj {
                                !prev_obj.info.id.eq(&current_on_top_of_layer.info.id)
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
                    let remove_old_obj = prev_obj.is_some() && current_on_top_of_layer.is_none();

                    if replace_old_obj || remove_old_obj {
                        if let Some(prev_obj) = prev_obj {
                            // Cap the old instance, so it'll end at this point in time:
                            // TODO - this needs enabling, but its going to be such a pain! Lets get some tests passing first
                            // set_instance_end_time(&mut prev_obj.instance, time);

                            // Update activeObjIds:
                            active_object_ids.remove(&prev_obj.info.id);

                            // Add to nextEvents:
                            let add_event =
                                only_for_time.as_ref().map(|t| time > *t).unwrap_or(true);
                            if add_event {
                                resolved_states.next_events.push(NextEvent {
                                    event_type: EventType::End,
                                    time,
                                    object_id: prev_obj.info.id.clone(),
                                });
                                if let Some(end) = instance.end {
                                    event_object_times.insert(end);
                                }
                            }
                        }
                    }
                    if replace_old_obj {
                        // Set the new object to State

                        let current_on_top_of_layer = current_on_top_of_layer.unwrap(); // TODO - eww

                        // Construct a new object clone:
                        let new_obj = {
                            let existing_obj = resolved_states
                                .objects
                                .get_mut(&current_on_top_of_layer.info.id);

                            if let Some(obj) = existing_obj {
                                obj
                            } else {
                                // TODO - how does the object properties line up with the one we are operating on?
                                let new_obj = ResolvedStatesForObject {
                                    info: current_on_top_of_layer.info.clone(),
                                    instances: Vec::new(),
                                };

                                // TODO - this?
                                // if let Some(classes) = new_obj.info.classes {
                                //     for class in classes {
                                //         if let Some(existing) = timeline.classes.get_mut(class) {
                                //             existing.push(obj_id.clone());
                                //         } else {
                                //             timeline
                                //                 .classes
                                //                 .insert(class.to_string(), vec![obj_id.clone()]);
                                //         }
                                //     }
                                // }

                                let obj_layer = &new_obj.info.layer;
                                if !obj_layer.is_empty() {
                                    if let Some(existing) =
                                        resolved_states.layers.get_mut(obj_layer)
                                    {
                                        existing.push(new_obj.info.id.clone());
                                    } else {
                                        resolved_states.layers.insert(
                                            obj_layer.to_string(),
                                            vec![new_obj.info.id.clone()],
                                        );
                                    }
                                }

                                resolved_states
                                    .objects
                                    .entry(new_obj.info.id.clone())
                                    .or_insert(new_obj)
                            }
                        };

                        let new_instance = {
                            let mut new_instance = current_on_top_of_layer.instance.deref().clone();
                            // We're setting new start & end times so they match up with the state:
                            new_instance.start = time;
                            new_instance.end = None;
                            new_instance.from_instance_id =
                                Some(current_on_top_of_layer.instance.id.clone());

                            if new_instance.original_end.is_none() {
                                new_instance.original_end = current_on_top_of_layer.instance.end;
                            }
                            if new_instance.original_start.is_none() {
                                new_instance.original_start =
                                    Some(current_on_top_of_layer.instance.start);
                            }

                            // Make the instance id unique:
                            for instance in &new_obj.instances {
                                if instance.id.eq(&new_instance.id) {
                                    new_instance.id =
                                        format!("{}_${}", new_instance.id, new_obj.instances.len());
                                }
                            }

                            Rc::new(new_instance)
                        };
                        new_obj.instances.push(new_instance.clone());

                        let new_obj_instance = Rc::new(ResolvedTimelineObjectInstance {
                            info: new_obj.info.clone(),
                            instance: new_instance.clone(),
                        });

                        // Save to current state:
                        current_state.insert(new_obj.info.layer.clone(), new_obj_instance.clone());

                        // Update activeObjIds:
                        active_object_ids
                            .insert(new_obj_instance.info.id.clone(), new_obj_instance.clone());

                        // Update the tracking state as well:
                        set_state_at_time(
                            &mut resolved_states.state,
                            &new_obj.info.layer,
                            time,
                            Some(&new_obj_instance),
                        );

                        // Add to nextEvents:
                        if new_instance.start > only_for_time.unwrap_or(0) {
                            resolved_states.next_events.push(NextEvent {
                                event_type: EventType::Start,
                                time: new_instance.start,
                                object_id: obj.info.id.clone(),
                            });
                            event_object_times.insert(new_instance.start);
                        }
                    } else if remove_old_obj {
                        // Remove from current state:
                        current_state.remove(&obj.info.layer);

                        // Update the tracking state as well:
                        set_state_at_time(&mut resolved_states.state, &obj.info.layer, time, None);
                    }
                } else {
                    // Is a keyframe

                    // Add keyframe to resolvedStates.objects:
                    resolved_states.objects.insert(
                        obj.info.id.clone(),
                        ResolvedStatesForObject {
                            info: obj.info.clone(),
                            instances: vec![obj.instance.clone()],
                        },
                    );

                    if to_be_enabled {
                        active_keyframes.insert(obj.info.id.clone(), o.obj.clone());
                    } else {
                        active_keyframes.remove(&obj.info.id);
                        active_keyframes_checked.remove(&obj.info.id);
                    }
                }
            }
        }

        for (obj_id, obj_instance) in &active_keyframes {
            let keyframe = &obj_instance;
            let instance = &keyframe.instance;

            let mut unhandled = true;

            let parent_obj = keyframe
                .info
                .parent_id
                .as_ref()
                .and_then(|parent_id| active_object_ids.get(parent_id));
            if let Some(parent_obj) = parent_obj {
                if !parent_obj.info.layer.is_empty() {
                    // keyframe is on an active object
                    if let Some(parent_obj_instance) = current_state.get(&parent_obj.info.layer) {
                        if active_keyframes_checked.insert(obj_id.clone()) {
                            // hasn't started before
                            let keyframe_instance =
                                Rc::new(ResolvedTimelineObjectInstanceKeyframe {
                                    info: keyframe.info.clone(),
                                    // instance: instance.clone(),
                                    keyframe_end_time: instance.end,
                                });

                            // Add keyframe to the tracking state:
                            add_keyframe_at_time(
                                &mut resolved_states.state,
                                &parent_obj.info.layer,
                                time,
                                &keyframe_instance,
                            );

                            // Add keyframe to nextEvents:
                            keyframe_events.push(NextEvent {
                                event_type: EventType::KeyFrame,
                                time: instance.start,
                                object_id: keyframe.info.id.clone(),
                            });

                            if let Some(end) = instance.end {
                                if parent_obj_instance
                                    .instance
                                    .end
                                    .as_ref()
                                    .map(|p_end| end < *p_end)
                                    .unwrap_or(true)
                                // Only add the keyframe if it ends before its parent
                                {
                                    keyframe_events.push(NextEvent {
                                        event_type: EventType::KeyFrame,
                                        time: end,
                                        object_id: keyframe.info.id.clone(),
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
                active_keyframes_checked.remove(obj_id);
            }
        }
    }

    // Go through the keyframe events and add them to nextEvents:
    for event in keyframe_events {
        if event_object_times.insert(event.time) {
            // no need to put a keyframe event if there's already another event there
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

        b.object_id.cmp(&a.object_id)
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
            for instance in &parent_obj.resolved.instances {
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
    }

    times
}

fn make_resolved_obj(
    obj: &ResolvedTimelineObject,
    instance: &TimelineObjectInstance,
) -> Rc<ResolvedTimelineObjectInstance> {
    Rc::new(ResolvedTimelineObjectInstance {
        info: obj.info.clone(),
        instance: Rc::new(instance.clone()),
    })
}

fn set_state_at_time(
    states: &mut AllStates,
    layer: &str,
    time: Time,
    instance: Option<&Rc<ResolvedTimelineObjectInstance>>,
) {
    let layer_states = states.entry(layer.to_string()).or_default();
    if let Some(instance) = instance {
        layer_states.insert(
            time,
            TimelineLayerState {
                info: instance.info.clone(),
                instance: instance.instance.clone(),
                keyframes: Vec::new(),
            },
        );
    } else {
        layer_states.remove(&time);
    }
}

fn add_keyframe_at_time(
    states: &mut AllStates,
    layer: &str,
    time: Time,
    instance: &Rc<ResolvedTimelineObjectInstanceKeyframe>,
) {
    let layer_states = states.entry(layer.to_string()).or_default();

    // TODO - this isnt as perfect as before, as it would create the entry with just the kf
    if let Some(time_state) = layer_states.get_mut(&time) {
        time_state.keyframes.push(instance.clone());
    }
}
