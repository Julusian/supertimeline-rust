use crate::caps::Cap;
use crate::expression::Expression;
use crate::util::Time;
#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct TimelineObjectInfo {
    pub id: String,
    pub enable: Vec<TimelineEnable>,
    pub priority: u64,
    pub disabled: bool,
    pub layer: String,

    /** Increases the more levels inside of a group the objects is */
    pub depth: usize,
    /** Id of the parent object */
    pub parent_id: Option<String>,
    /** True if object is a keyframe */
    pub is_keyframe: bool,
}

#[derive(Debug, Clone)]
pub struct TimelineObjectResolvedWip {
    /** True if object is referencing itself (only directly, not indirectly via another object) */
    pub is_self_referencing: bool,
}

#[derive(Debug, Clone)]
pub struct TimelineObjectResolved {
    // pub status: Rc<Atomic<TimelineObjectResolvingStatus>>,
    /** True if object is referencing itself (only directly, not indirectly via another object) */
    pub is_self_referencing: bool,

    /** Instances of the object on the timeline */
    pub instances: Vec<TimelineObjectInstance>,
    /** Ids of all other objects that directly affects this object (ie through direct reference, classes, etc) */
    pub direct_references: HashSet<String>,
}

#[derive(Debug, Clone, Default)]
pub struct TimelineObjectInstance {
    /** id of the instance (unique)  */
    pub id: String,
    /** if true, the instance starts from the beginning of time */
    pub is_first: bool,
    /** The start time of the instance */
    pub start: Time,
    /** The end time of the instance (null = infinite) */
    pub end: Option<Time>,

    /** The original start time of the instance (if an instance is split or capped, the original start time is retained in here).
     * If undefined, fallback to .start
     */
    pub original_start: Option<Time>,
    /** The original end time of the instance (if an instance is split or capped, the original end time is retained in here)
     * If undefined, fallback to .end
     */
    pub original_end: Option<Time>,

    /** array of the id of the referenced objects */
    pub references: HashSet<String>,

    /** If set, tells the cap of the parent. The instance will always be capped inside this. */
    pub caps: Vec<Cap>,
    /** If the instance was generated from another instance, reference to the original */
    pub from_instance_id: Option<String>,
}

#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TimelineEnable {
    /** (Optional) The start time of the object. (Cannot be combined with .while) */
    #[cfg_attr(
        feature = "serde_support",
        serde(rename = "start", skip_serializing_if = "Option::is_none")
    )]
    pub enable_start: Option<Expression>,

    /** (Optional) The end time of the object (Cannot be combined with .while or .duration) */
    #[cfg_attr(
        feature = "serde_support",
        serde(rename = "end", skip_serializing_if = "Option::is_none")
    )]
    pub enable_end: Option<Expression>,

    /** (Optional) Enables the object WHILE expression is true (ie sets both the start and end). (Cannot be combined with .start, .end or .duration ) */
    #[cfg_attr(
        feature = "serde_support",
        serde(rename = "while", skip_serializing_if = "Option::is_none")
    )]
    pub enable_while: Option<Expression>,

    /** (Optional) The duration of an object */
    #[cfg_attr(
        feature = "serde_support",
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub duration: Option<Expression>,

    /** (Optional) Makes the object repeat with given interval */
    #[cfg_attr(
        feature = "serde_support",
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub repeating: Option<Expression>,
}
