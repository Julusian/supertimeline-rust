use napi::JsObject;
use supertimeline::IsTimelineKeyframe;
use supertimeline::IsTimelineObject;
use supertimeline::TimelineEnable;

pub struct NapiTimelineObjectKeyframe {
    pub id: String,
    pub enable: Vec<TimelineEnable>,
    pub classes: Option<Vec<String>>,
    pub disabled: bool,
    pub content: JsObject,
}
impl IsTimelineKeyframe for NapiTimelineObjectKeyframe {
    fn id(&self) -> &str {
        &self.id
    }
    fn enable(&self) -> &Vec<TimelineEnable> {
        &self.enable
    }
    fn classes(&self) -> Option<&Vec<String>> {
        self.classes.as_ref()
    }
    fn disabled(&self) -> bool {
        self.disabled
    }
}

pub struct NapiTimelineObject {
    pub id: String,
    pub enable: Vec<TimelineEnable>,
    pub layer: String,
    pub keyframes: Option<Vec<NapiTimelineObjectKeyframe>>,
    pub classes: Option<Vec<String>>,
    pub disabled: bool,
    pub content: JsObject,
    pub children: Option<Vec<NapiTimelineObject>>,
    pub priority: i64,
}
impl IsTimelineObject<NapiTimelineObject, NapiTimelineObjectKeyframe> for NapiTimelineObject {
    fn id(&self) -> &str {
        &self.id
    }
    fn enable(&self) -> &Vec<TimelineEnable> {
        &self.enable
    }
    fn layer(&self) -> &str {
        &self.layer
    }
    fn keyframes(&self) -> Option<&Vec<NapiTimelineObjectKeyframe>> {
        self.keyframes.as_ref()
    }
    fn classes(&self) -> Option<&Vec<String>> {
        self.classes.as_ref()
    }
    fn disabled(&self) -> bool {
        self.disabled
    }
    fn children(&self) -> Option<&Vec<NapiTimelineObject>> {
        self.children.as_ref()
    }
    fn priority(&self) -> i64 {
        self.priority
    }
}
