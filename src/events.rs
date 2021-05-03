use std::collections::{HashSet, HashMap};
use crate::instance::TimelineObjectInstance;
use crate::util::{Time, getId, join_references, add_caps_to_resuming};
use std::cmp::Ordering;

pub struct EventForInstance<'a> {
    pub time: Time,
    pub is_start: bool,
    pub references: &'a HashSet<String>,
    pub instance: &'a TimelineObjectInstance,
    pub id: Option<String>,
}

pub fn sort_events(mut events: Vec<EventForInstance>) {
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

pub fn convert_events_to_instances(mut events: Vec<EventForInstance>, allow_merge: bool, allow_zero_gaps: bool) -> Vec<TimelineObjectInstance> {
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