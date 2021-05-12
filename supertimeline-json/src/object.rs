use serde::{Deserialize, Serialize};
use supertimeline::IsTimelineKeyframe;
use supertimeline::IsTimelineObject;
use supertimeline::TimelineEnable;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct JsonTimelineObjectKeyframe {
    pub id: String,
    pub enable: Vec<TimelineEnable>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classes: Option<Vec<String>>,
    pub disabled: bool,
    pub content: serde_json::Value,
}
impl IsTimelineKeyframe for JsonTimelineObjectKeyframe {
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

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct JsonTimelineObject {
    pub id: String,
    pub enable: Vec<TimelineEnable>,
    pub layer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keyframes: Option<Vec<JsonTimelineObjectKeyframe>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classes: Option<Vec<String>>,
    pub disabled: bool,
    pub content: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<JsonTimelineObject>>,
    pub priority: u64,
}
impl IsTimelineObject<JsonTimelineObject, JsonTimelineObjectKeyframe> for JsonTimelineObject {
    fn id(&self) -> &str {
        &self.id
    }
    fn enable(&self) -> &Vec<TimelineEnable> {
        &self.enable
    }
    fn layer(&self) -> &str {
        &self.layer
    }
    fn keyframes(&self) -> Option<&Vec<JsonTimelineObjectKeyframe>> {
        self.keyframes.as_ref()
    }
    fn classes(&self) -> Option<&Vec<String>> {
        self.classes.as_ref()
    }
    fn disabled(&self) -> bool {
        self.disabled
    }
    fn children(&self) -> Option<&Vec<JsonTimelineObject>> {
        self.children.as_ref()
    }
    fn priority(&self) -> u64 {
        self.priority
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use supertimeline::Expression;
    use supertimeline::ExpressionObj;
    use supertimeline::ExpressionOperator;

    #[test]
    // #[ignore]
    fn demo() {
        let src = JsonTimelineObject {
            id: "test".to_string(),
            layer: "lay".to_string(),
            classes: None,
            disabled: false,
            priority: 0,
            content: serde_json::from_str("{}").unwrap(),
            enable: vec![TimelineEnable {
                enable_start: Some(Expression::Number(4)),
                enable_end: Some(ExpressionObj::create(
                    Expression::Number(10),
                    ExpressionOperator::Add,
                    Expression::String("abc".to_string()),
                )),
                // duration: Some(Expression::Null),
                ..Default::default()
            }],
            children: None,
            keyframes: None,
        };

        let j = serde_json::to_string(&src).unwrap();

        // Print, write to a file, or send to an HTTP server.
        println!("output {}", j);

        let parsed = serde_json::from_str::<JsonTimelineObject>(&j).unwrap();

        println!("got back {:#?}", parsed);

        assert_eq!(src, parsed);

        panic!();
    }
}
