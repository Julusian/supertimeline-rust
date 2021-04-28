use crate::instance::{TimelineObjectInstance, Cap};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use crate::resolver::TimeWithReference;

pub type Time = u64;


#[derive(Debug, Clone)]
pub struct TimelineObject {
    // TODO
    pub id: String,
}

pub fn getId() -> String {
// TODO
 "".to_string()
}

pub fn invert_instances(instances: &Vec<TimelineObjectInstance>) -> Vec<TimelineObjectInstance> {
    if instances.len() == 0 {
        vec![TimelineObjectInstance {
            id: getId(),
            isFirst: true,
            start: 0,
            end: None,
            references: Vec::new(),

            originalStart: None,
            originalEnd: None,

            caps: Vec::new(),
            fromInstanceId: None
        }]
    } else {
        let cleaned_instances = clean_instances(instances, true, true);

        let mut inverted_instances = Vec::new();

        let first_instance = &cleaned_instances[0];

        // Fill the time between the first and zero
        if first_instance.start != 0 {
            inverted_instances.push(TimelineObjectInstance {
                id: getId(),
                isFirst: true,
                start: 0,
                end: None,
                references: join_references(&first_instance.references, None, Some(&first_instance.id)),
                caps: Vec::new(),

                // TODO - what are these for if they arent set here?
                originalStart: None,
                originalEnd: None,
                fromInstanceId: None,
            });
        }

        // Fill in between the instances
        for instance in cleaned_instances {
            if let Some(prev_instance) = inverted_instances.last_mut() {
                prev_instance.end = Some(instance.start);
            }

            if let Some(end) = instance.end {
                inverted_instances.push(TimelineObjectInstance {
                    id: getId(),
                    isFirst: false,
                    start: end,
                    end: None,
                    references: join_references(&instance.references, None, Some(&instance.id)),
                    caps: instance.caps,

                    // TODO - what are these for if they arent set here?
                    originalStart: None,
                    originalEnd: None,
                    fromInstanceId: None,
                });
            }
        }


        inverted_instances
    }
}

// Cleanup instances. Join overlaps or touching etc
pub fn clean_instances (instances: &Vec<TimelineObjectInstance>, allow_merge: bool, allow_zero_gaps: bool) -> Vec<TimelineObjectInstance> {
    match instances.len() {
        0 => Vec::new(),
        1 => {
            let mut instance = instances[0].clone();
            instance.originalStart = Some(instance.start);
            instance.originalEnd = instance.end;

            vec![instance]
        },
        _ => {
            let mut events = Vec::new();

            for instance in instances {
                events.push(EventForInstance{
                    time: instance.start,
                    is_start: true,
                    references: &instance.references,
                    instance,
                });

                if let Some(end) = instance.end {
                    events.push(EventForInstance{
                        time: end,
                        is_start: false,
                        references: &instance.references,
                        instance,
                    });
                }
            }

            convert_events_to_instances(events, allow_merge, allow_zero_gaps)
        }
    }
}

struct EventForInstance<'a> {
    time: Time,
    is_start: bool,
    references: &'a Vec<String>,
    instance: &'a TimelineObjectInstance,
}

fn sort_events(mut events: Vec<EventForInstance>) {
    events.sort_by(|a, b| {
        if a.time > b.time {
            Ordering::Greater
        } else if a.time < b.time {
            Ordering::Less
        } else {
            // const aId = a.data && (a.data.id || (a.data.instance && a.data.instance.id))
            // const bId = b.data && (b.data.id || (b.data.instance && b.data.instance.id))

            if a.instance.id == b.instance.id {
                // If the event refer to the same ID, let the ending event be first:
                if a.is_start && !b.is_start {
                    return Ordering::Less
                } else if !a.is_start && b.is_start {
                    return Ordering::Greater
                }
            }

            if a.is_start && !b.is_start {
                return Ordering::Greater
            } else if !a.is_start && b.is_start {
                return Ordering::Less
            }

            Ordering::Equal
        }
    });
}

fn convert_events_to_instances(mut events: Vec<EventForInstance>, allow_merge: bool, allow_zero_gaps: bool) -> Vec<TimelineObjectInstance> {
    sort_events(events);

    let mut return_instances = Vec::new();

    let mut active_instances = HashMap::new();
    let mut active_instance_id = None;
    let mut previous_active = false;

    for event in events {
        let event_id = &event.instance.id;

        let last_instance = return_instances.last_mut();

        // Track the event's change
        if event.is_start {
            active_instances.insert(event_id, &event);
        } else {
            active_instances.remove(event_id);
        }

        if active_instances.is_empty() {
            // No instances are active
            if previous_active {
                if let Some(last_instance) = last_instance {
                    last_instance.end = Some(event.time);
                }
            }
            previous_active = false
        } else {
            // There is an active instance
            previous_active = true;

            if let Some(last_instance) = last_instance {
                if !allow_merge && event.is_start && last_instance.end.is_none() && !active_instance_id.eq(event_id) {
                    // Start a new instance:
                    last_instance.end = Some(event.time);
                    return_instances.push(TimelineObjectInstance {
                        id: getId(),
                        start: event.time,
                        end: None,
                        references: event.references.cloned(),

                        isFirst: false,
                        caps: Vec::new(),
                        originalStart: None,
                        originalEnd: None,
                        fromInstanceId: None,
                    });
                    active_instance_id = Some(event_id);
                } else if !allow_merge && !event.is_start && active_instance_id == event_id {
                    // The active instance stopped playing, but another is still playing
                    let latest_instance = active_instances.iter().reduce(|a,b| if a.1.time < b.1.time { b } else { a });

                    if let Some(latest_instance) = latest_instance {
                        // Restart that instance now:
                        last_instance.end = Some(event.time);
                        return_instances.push(TimelineObjectInstance {
                            id: event_id + '_' + getId(),
                            start: event.time,
                            end: None,
                            references: latest_instance.references.cloned(),

                            isFirst: false,
                            caps: Vec::new(),
                            originalStart: None,
                            originalEnd: None,
                            fromInstanceId: None,
                        });
                        active_instance_id = Some(latest_instance.0);
                    }
                 } else if allow_merge && !allow_zero_gaps && last_instance.end == event.time {
                        // The previously running ended just now
                        // resume previous instance:
                    last_instance.end = None;
                    last_instance.references = join_references(&last_instance.references, Some(event.references), None);
                    add_caps_to_resuming(last_instance, &event.instance.caps);
                } else if let Some(end) = last_instance.end {
                    // There is no previously running instance
                    // Start a new instance:
                    return_instances.push( TimelineObjectInstance{
                        id: event_id.to_string(),
                        start: event.time,
                        end: None,
                        references: event.references.clone(),
                        caps: event.instance.caps.cloned(),

                        isFirst: false,
                        originalStart: None,
                        originalEnd: None,
                        fromInstanceId: None,
                    });
                    active_instance_id = Some(event_id);
                } else {
                    // There is already a running instance
                    last_instance.references = join_references(&last_instance.references, Some(event.references), None);
                    add_caps_to_resuming(last_instance, &event.instance.caps);
                }
            } else {
                // There is no previously running instance
                // Start a new instance:
                return_instances.push( TimelineObjectInstance{
                    id: event_id.to_string(),
                    start: event.time,
                    end: None,
                    references: event.references.clone(),
                    caps: event.instance.caps.cloned(),

                    isFirst: false,
                    originalStart: None,
                    originalEnd: None,
                    fromInstanceId: None,
                });
                active_instance_id = Some(event_id);
            }
        }
    }

    return_instances
}

fn add_caps_to_resuming(instance: &mut TimelineObjectInstance, caps: &Vec<Cap>) {
    let mut new_caps = Vec::new();

    for cap in caps {
        if let Some(cap_end) = cap.end {
            if let Some(instance_end) = instance.end {
                if cap_end > instance_end {
                    new_caps.push(Cap {
                        id: cap.id.clone(),
                        start: 0,
                        end: Some(cap_end),
                    })
                }
            }
        }
    }

    instance.caps = join_caps(&instance.caps, &new_caps)
}

pub fn join_caps(a: &Vec<Cap>, b: &Vec<Cap>) -> Vec<Cap> {
    let mut cap_map = HashMap::new();

    for cap in a {
        cap_map.insert(&cap.id, cap);
    }
    for cap in b {
        cap_map.insert(&cap.id, cap);
    }

   cap_map.values().cloned()
}

pub fn join_references(a: &Vec<String>, b: Option<&Vec<String>>, c: Option<&String>)-> Vec<String> {
    let mut new_refs = HashSet::new();
    new_refs.extend(a);

    if let Some(b) = b {
        new_refs.extend(b);
    }

    if let Some(c) = c {
        new_refs.insert(c);
    }

    new_refs.into_iter().collect()
}


pub fn join_references3(a: &Option<TimeWithReference>, b: &Option<TimeWithReference>)-> HashSet<String> {
    let mut new_refs = HashSet::new();
    if let Some(a) = a {
        new_refs.extend(&a.references);
    }
    if let Some(b) = b {
        new_refs.extend(&b.references);
    }
    new_refs.cloned()
}

pub fn join_references4(a: Option<&HashSet<String>>, b: Option<&HashSet<String>>)-> HashSet<String> {
    let mut new_refs = HashSet::new();
    if let Some(a) = a {
        new_refs.extend(&a);
    }
    if let Some(b) = b {
        new_refs.extend(&b);
    }
    new_refs.cloned()
}

pub fn join_references2(a: &HashSet<String>, b: &HashSet<String>)-> HashSet<String> {
    let mut new_refs = HashSet::new();
    new_refs.extend(a);
    new_refs.extend(b);
    new_refs.cloned()
}


pub fn operate_on_arrays(a: &Vec<TimelineObjectInstance>, b: &Vec<TimelineObjectInstance>) -> Vec<TimelineObjectInstance> {
    // TODO
    Vec::new()
}