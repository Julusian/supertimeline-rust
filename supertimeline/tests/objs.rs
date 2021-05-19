use supertimeline::TimelineObjectInstance;
use supertimeline::{IsTimelineKeyframe, IsTimelineObject, Time, TimelineEnable};

#[derive(Default)]
pub struct SimpleTimelineObj {
    pub id: String,
    pub enable: Vec<TimelineEnable>,
    pub layer: String,
    pub keyframes: Vec<SimpleKeyframe>,
    pub classes: Vec<String>,
    pub disabled: bool,
    pub children: Option<Vec<SimpleTimelineObj>>,
    pub priority: i64,
}
impl IsTimelineObject<SimpleTimelineObj, SimpleKeyframe> for SimpleTimelineObj {
    fn id(&self) -> &str {
        &self.id
    }
    fn enable(&self) -> &Vec<TimelineEnable> {
        &self.enable
    }
    fn layer(&self) -> &str {
        &self.layer
    }
    fn keyframes(&self) -> Option<&Vec<SimpleKeyframe>> {
        Some(self.keyframes.as_ref())
    }
    fn classes(&self) -> Option<&Vec<String>> {
        Some(self.classes.as_ref())
    }
    fn disabled(&self) -> bool {
        self.disabled
    }
    fn children(&self) -> Option<&Vec<SimpleTimelineObj>> {
        self.children.as_ref()
    }
    fn priority(&self) -> i64 {
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

/**
 * This is a copy of TimelineObjectInstance, with many properties removed, to simplify comparison
 */
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineObjectInstanceLight {
    // pub id: String,
    pub is_first: bool,
    pub start: Time,
    pub end: Option<Time>,

    pub original_start: Option<Time>,
    pub original_end: Option<Time>,

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
