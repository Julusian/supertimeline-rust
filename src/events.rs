use crate::caps::Cap;
use crate::instance::TimelineObjectInstance;
use crate::references::ReferencesBuilder;
use crate::resolver::ResolverContext;
use crate::util::{add_caps_to_resuming, Time};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

pub trait IsEvent {
    fn time(&self) -> Time;
    fn is_start(&self) -> bool;
    fn id(&self) -> &String;
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

#[derive(Debug)]
pub struct EventForInstance {
    pub time: Time,
    pub is_start: bool,
    pub references: HashSet<String>,
    pub caps: Vec<Cap>,
    pub id: String,
}
impl IsEvent for EventForInstance {
    fn time(&self) -> u64 {
        self.time
    }

    fn is_start(&self) -> bool {
        self.is_start
    }

    fn id(&self) -> &String {
        &self.id
    }
}

pub trait EventForInstanceExt {
    fn into_instances(
        self,
        ctx: &ResolverContext,
        allow_merge: bool,
        allow_zero_gaps: bool,
    ) -> Vec<TimelineObjectInstance>;
}
impl EventForInstanceExt for Vec<EventForInstance> {
    fn into_instances(
        mut self,
        ctx: &ResolverContext,
        allow_merge: bool,
        allow_zero_gaps: bool,
    ) -> Vec<TimelineObjectInstance> {
        self.sort();

        // let mut return_instances_map = HashMap::new();
        let mut return_instances: Vec<TimelineObjectInstance> = Vec::new();

        let mut active_instances = HashMap::new();
        let mut active_instance_id: Option<String> = None;
        let mut previous_active = false;

        for event in self.iter() {
            let event_id = event.id().clone();

            let last_instance = return_instances.last_mut();

            // Track the event's change
            if event.is_start {
                active_instances.insert(event_id.clone(), event);
            } else {
                active_instances.remove(&event_id);
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
                            .as_ref()
                            .map(|aiid| aiid.eq(&event_id))
                            .unwrap_or(false)
                    {
                        // Start a new instance:
                        last_instance.end = Some(event.time);
                        return_instances.push(TimelineObjectInstance {
                            id: ctx.generate_id(),
                            start: event.time,
                            end: None,
                            references: event.references.clone(),

                            is_first: false,
                            caps: Vec::new(),
                            original_start: None,
                            original_end: None,
                            from_instance_id: None,
                        });
                        active_instance_id = Some(event_id);
                    } else if !allow_merge
                        && !event.is_start
                        && active_instance_id
                            .as_ref()
                            .map(|aiid| aiid.eq(&event_id))
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
                                id: format!("{}_{}", event_id, ctx.generate_id()),
                                start: event.time,
                                end: None,
                                references: latest_instance.1.references.clone(),

                                is_first: false,
                                caps: Vec::new(),
                                original_start: None,
                                original_end: None,
                                from_instance_id: None,
                            });
                            active_instance_id = Some(latest_instance.0.clone());
                        }
                    } else if allow_merge
                        && !allow_zero_gaps
                        && last_instance.end.unwrap_or(Time::MAX) == event.time
                    {
                        // The previously running ended just now
                        // resume previous instance:
                        last_instance.end = None;
                        add_caps_to_resuming(last_instance, &event.caps);
                        last_instance.references = ReferencesBuilder::new()
                            .add(&last_instance.references)
                            .add(&event.references)
                            .done();
                    } else if let Some(_end) = last_instance.end {
                        // There is no previously running instance
                        // Start a new instance:
                        return_instances.push(TimelineObjectInstance {
                            id: event_id.clone(),
                            start: event.time,
                            end: None,
                            references: event.references.clone(),
                            caps: event.caps.clone(),

                            is_first: false,
                            original_start: None,
                            original_end: None,
                            from_instance_id: None,
                        });
                        active_instance_id = Some(event_id);
                    } else {
                        // There is already a running instance
                        last_instance.references = ReferencesBuilder::new()
                            .add(&last_instance.references)
                            .add(&event.references)
                            .done();
                        add_caps_to_resuming(last_instance, &event.caps);
                    }
                } else {
                    // There is no previously running instance
                    // Start a new instance:
                    return_instances.push(TimelineObjectInstance {
                        id: event_id.clone(),
                        start: event.time,
                        end: None,
                        references: event.references.clone(),
                        caps: event.caps.clone(),

                        is_first: false,
                        original_start: None,
                        original_end: None,
                        from_instance_id: None,
                    });
                    active_instance_id = Some(event_id);
                }
            }
        }

        return_instances
    }
}
