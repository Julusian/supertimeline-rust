use crate::api::ResolverContext;
use crate::instance::TimelineObjectInstance;
use crate::util::{add_caps_to_resuming, join_hashset, Time};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

pub trait IsEvent {
    fn time(&self) -> Time;
    fn is_start(&self) -> bool;
    fn id(&self) -> &str;
}

pub trait VecIsEventExt {
    fn sort(&mut self);
}
impl<T: IsEvent> VecIsEventExt for Vec<T> {
    fn sort(&mut self) {
        self.sort_by(|a, b| {
            let a_time = a.time();
            let b_time = b.time();

            if a_time > b_time {
                Ordering::Greater
            } else if a_time < b_time {
                Ordering::Less
            } else {
                let a_start = a.is_start();
                let b_start = b.is_start();

                let a_id = a.id();
                let b_id = b.id();

                if a_id == b_id {
                    // If the event refer to the same ID, let the ending event be first:
                    if a_start && !b_start {
                        return Ordering::Less;
                    } else if !a_start && b_start {
                        return Ordering::Greater;
                    }
                } else {
                    if a_start && !b_start {
                        return Ordering::Greater;
                    } else if !a_start && b_start {
                        return Ordering::Less;
                    }
                }

                Ordering::Equal
            }
        });
    }
}

pub struct EventForInstance<'a> {
    pub time: Time,
    pub is_start: bool,
    pub references: &'a HashSet<String>,
    pub instance: &'a TimelineObjectInstance,
    pub id: Option<String>,
}
impl<'a> IsEvent for EventForInstance<'a> {
    fn time(&self) -> u64 {
        self.time
    }

    fn is_start(&self) -> bool {
        self.is_start
    }

    fn id(&self) -> &str {
        if let Some(id) = &self.id {
            id
        } else {
            &self.instance.id
        }
    }
}

pub trait EventForInstanceExt {
    fn to_instances(
        &mut self,
        ctx: &dyn ResolverContext,
        allow_merge: bool,
        allow_zero_gaps: bool,
    ) -> Vec<TimelineObjectInstance>;
}
impl<'a> EventForInstanceExt for Vec<EventForInstance<'a>> {
    fn to_instances(
        &mut self,
        ctx: &dyn ResolverContext,
        allow_merge: bool,
        allow_zero_gaps: bool,
    ) -> Vec<TimelineObjectInstance> {
        self.sort();

        let mut return_instances: Vec<TimelineObjectInstance> = Vec::new();

        let mut active_instances = HashMap::new();
        let mut active_instance_id: Option<&String> = None;
        let mut previous_active = false;

        for event in self {
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
                    if !allow_merge
                        && event.is_start
                        && last_instance.end.is_none()
                        && !active_instance_id
                            .and_then(|aiid| Some(aiid.eq(event_id)))
                            .unwrap_or(false)
                    {
                        // Start a new instance:
                        last_instance.end = Some(event.time);
                        return_instances.push(TimelineObjectInstance {
                            id: ctx.get_id(),
                            start: event.time,
                            end: None,
                            references: event.references.clone(),

                            isFirst: false,
                            caps: Vec::new(),
                            originalStart: None,
                            originalEnd: None,
                            fromInstanceId: None,
                        });
                        active_instance_id = Some(event_id);
                    } else if !allow_merge
                        && !event.is_start
                        && active_instance_id
                            .and_then(|aiid| Some(aiid.eq(event_id)))
                            .unwrap_or(false)
                    {
                        // The active instance stopped playing, but another is still playing
                        let latest_instance =
                            active_instances
                                .iter()
                                .reduce(|a, b| if a.1.time < b.1.time { b } else { a });

                        if let Some(latest_instance) = latest_instance {
                            // Restart that instance now:
                            last_instance.end = Some(event.time);
                            return_instances.push(TimelineObjectInstance {
                                id: format!("{}_{}", event_id, ctx.get_id()),
                                start: event.time,
                                end: None,
                                references: latest_instance.1.references.clone(),

                                isFirst: false,
                                caps: Vec::new(),
                                originalStart: None,
                                originalEnd: None,
                                fromInstanceId: None,
                            });
                            active_instance_id = Some(latest_instance.0);
                        }
                    } else if allow_merge
                        && !allow_zero_gaps
                        && last_instance.end.unwrap_or(Time::MAX) == event.time
                    {
                        // The previously running ended just now
                        // resume previous instance:
                        last_instance.end = None;
                        last_instance.references =
                            join_hashset(&last_instance.references, event.references);
                        add_caps_to_resuming(last_instance, &event.instance.caps);
                    } else if let Some(end) = last_instance.end {
                        // There is no previously running instance
                        // Start a new instance:
                        return_instances.push(TimelineObjectInstance {
                            id: event_id.to_string(),
                            start: event.time,
                            end: None,
                            references: event.references.clone(),
                            caps: event.instance.caps.clone(),

                            isFirst: false,
                            originalStart: None,
                            originalEnd: None,
                            fromInstanceId: None,
                        });
                        active_instance_id = Some(event_id);
                    } else {
                        // There is already a running instance
                        last_instance.references =
                            join_hashset(&last_instance.references, event.references);
                        add_caps_to_resuming(last_instance, &event.instance.caps);
                    }
                } else {
                    // There is no previously running instance
                    // Start a new instance:
                    return_instances.push(TimelineObjectInstance {
                        id: event_id.to_string(),
                        start: event.time,
                        end: None,
                        references: event.references.clone(),
                        caps: event.instance.caps.clone(),

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
}
