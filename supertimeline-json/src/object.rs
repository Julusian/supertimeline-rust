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
    #[serde(default)]
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
    // #[serde(deserialize_with = "some_enable")]
    pub enable: Vec<TimelineEnable>,
    pub layer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keyframes: Option<Vec<JsonTimelineObjectKeyframe>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classes: Option<Vec<String>>,
    #[serde(default)]
    pub disabled: bool,
    pub content: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<JsonTimelineObject>>,
    #[serde(default)]
    pub priority: i64,
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
    fn priority(&self) -> i64 {
        self.priority
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hack::mangle_json_enable;
    use std::time::Instant;
    use supertimeline::get_state;
    use supertimeline::resolve_all_states;
    use supertimeline::resolve_timeline;
    use supertimeline::Expression;
    use supertimeline::ExpressionObj;
    use supertimeline::ExpressionOperator;
    use supertimeline::ResolveOptions;

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

    #[test]
    // #[ignore]
    fn parse() {
        let j = mangle_json_enable(include_str!("../../dumps/real.json")).unwrap();

        let mut deserializer = serde_json::Deserializer::from_str(&j);
        let parsed: Vec<JsonTimelineObject> =
            serde_path_to_error::deserialize(&mut deserializer).unwrap();

        // println!("got back {:#?}", parsed);
        // panic!();

        let options = ResolveOptions {
            time: 1597158621470,
            limit_count: None,
            limit_time: None,
        };

        let start = Instant::now();

        let resolved = resolve_timeline(&parsed, options).unwrap();
        // let states = resolve_all_states(&resolved, None).unwrap();

        // let state = get_state(&states, 1597158621470 + 5000, None);

        let duration = start.elapsed();

        // println!("got back {:#?}", state);

        let result = serde_json::to_string(&resolved).unwrap();

        std::fs::write("output.json", result).unwrap();

        println!("it took {}", duration.as_millis());
        panic!();
    }
}
