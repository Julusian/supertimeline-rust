use supertimeline::TimelineObjectInstance;
use supertimeline::{IsTimelineKeyframe, IsTimelineObject, Time, TimelineEnable};

#[derive(Default)]
pub struct SimpleTimelineObj {
    pub id: String,
    pub enable: Vec<TimelineEnable>,
    pub layer: String,
    pub keyframes: Vec<Box<dyn IsTimelineKeyframe>>,
    pub classes: Vec<String>,
    pub disabled: bool,
    pub children: Option<Vec<Box<dyn IsTimelineObject>>>,
    pub priority: u64,
}
impl IsTimelineObject for SimpleTimelineObj {
    fn id(&self) -> &str {
        &self.id
    }
    fn enable(&self) -> &Vec<TimelineEnable> {
        &self.enable
    }
    fn layer(&self) -> &str {
        &self.layer
    }
    fn keyframes(&self) -> Option<&Vec<Box<dyn IsTimelineKeyframe>>> {
        Some(self.keyframes.as_ref())
    }
    fn classes(&self) -> Option<&Vec<String>> {
        Some(self.classes.as_ref())
    }
    fn disabled(&self) -> bool {
        self.disabled
    }
    fn children(&self) -> Option<&Vec<Box<dyn IsTimelineObject>>> {
        self.children.as_ref()
    }
    fn priority(&self) -> u64 {
        self.priority
    }
}

#[derive(Default)]
pub struct SimpleKeyframe {
    pub id: String,
    pub enable: Vec<TimelineEnable>,
    pub classes: Vec<String>,
    pub disabled: bool,
}
impl IsTimelineKeyframe for SimpleKeyframe {
    fn id(&self) -> &str {
        &self.id
    }
    fn enable(&self) -> &Vec<TimelineEnable> {
        &self.enable
    }
    fn classes(&self) -> Option<&Vec<String>> {
        Some(self.classes.as_ref())
    }
    fn disabled(&self) -> bool {
        self.disabled
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimelineObjectInstanceLight {
    // /** id of the instance (unique)  */
    // pub id: String,
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

    /** If the instance was generated from another instance, reference to the original */
    pub from_instance_id: Option<String>,
}
impl TimelineObjectInstanceLight {
    pub fn from(instance: &TimelineObjectInstance) -> TimelineObjectInstanceLight {
        TimelineObjectInstanceLight {
            // id: instance.id.clone(),
            is_first: instance.is_first,
            start: instance.start,
            end: instance.end,

            original_start: instance.original_start,
            original_end: instance.original_end,

            from_instance_id: instance.from_instance_id.clone(),
        }
    }
}
