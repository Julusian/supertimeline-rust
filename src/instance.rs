use crate::util::Time;
use crate::util::TimelineObject;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub enum ResolvedTimelineObjectEntry {
    Instance(ResolvedTimelineObjectInstance),
    Keyframe(ResolvedTimelineObjectInstanceKeyframe),
}

#[derive(Debug, Clone)]
pub struct ResolvedTimelineObjectInstanceKeyframe {
    pub instance: ResolvedTimelineObjectInstance,
    //pub isKeyframe: bool,
    pub keyframeEndTime: Option<Time>,
}

#[derive(Debug, Clone)]
pub struct ResolvedTimelineObjectInstance {
    pub object: TimelineObject,
    pub resolved: TimelineObjectResolved,
    pub instance: TimelineObjectInstance,
}

#[derive(Debug, Clone)]
pub struct Cap {
    pub id: String, // id of the parent
    pub start: Time,
    pub end: Option<Time>,
}

#[derive(Debug, Clone)]
pub struct TimelineObjectResolved {
    /** Is set to true when object has been resolved */
    pub resolved: bool,
    /** Is set to true while object is resolved (to prevent circular references) */
    pub resolving: bool,
    /** Instances of the object on the timeline */
    // instances: Array<TimelineObjectInstance>
    /** Increases the more levels inside of a group the objects is */
    pub levelDeep: Option<usize>,
    /** Id of the parent object */
    pub parentId: Option<String>,
    /** True if object is a keyframe */
    pub isKeyframe: Option<bool>,
    /** True if object is referencing itself (only directly, not indirectly via another object) */
    pub isSelfReferencing: Option<bool>,
    /** Ids of all other objects that directly affects this object (ie through direct reference, classes, etc) */
    pub directReferences: Vec<String>,
}

#[derive(Debug, Clone)]
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
    pub references: HashSet<String>,

    /** If set, tells the cap of the parent. The instance will always be capped inside this. */
    pub caps: Vec<Cap>,
    /** If the instance was generated from another instance, reference to the original */
    pub fromInstanceId: Option<String>,
}
