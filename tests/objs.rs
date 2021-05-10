use supertimeline::{IsTimelineKeyframe, IsTimelineObject, TimelineEnable};

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
