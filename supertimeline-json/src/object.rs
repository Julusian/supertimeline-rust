use serde::{Deserialize, Serialize};
// use supertimeline::ExpressionObj;
// use supertimeline::ExpressionOperator;
use supertimeline::IsTimelineKeyframe;
use supertimeline::IsTimelineObject;
use supertimeline::TimelineEnable;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct JsonTimelineObjectKeyframe {
    pub id: String,
    // #[serde(default, with = "vec_timeline_enable")]
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
    //fn duration (&self) -> Option<TimelineKeyframeDuration>;
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
    // #[serde(default, with = "vec_timeline_enable")]
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

// mod vec_timeline_enable {
//     use serde::ser::{SerializeSeq, SerializeStruct};
//     use serde::{Deserialize, Deserializer, Serialize, Serializer};
//     use supertimeline::Expression;
//     use supertimeline::TimelineEnable;

//     // fn serialize_expression<S>(value: &Option<Expression>, serializer: S) -> Result<S::Ok, S::Error>  where
//     // S: Serializer,{
//     //     // TODO
//     // }

//     pub fn serialize<S>(value: &Vec<TimelineEnable>, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         let mut seq = serializer.serialize_seq(Some(value.len()))?;
//         for e in value {
//             // let mut state = seq.serialize_struct("TimelineEnable", 5)?;
//             // // state.serialize_field()
//             // // state.serialize_field("r", &self.r)?;
//             // // state.serialize_field("g", &self.g)?;
//             // // state.serialize_field("b", &self.b)?;
//             // state.end()?;
//         }
//         seq.end()
//     }

//     pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<TimelineEnable>, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         // #[derive(Deserialize)]
//         // struct Helper(#[serde(with = "ExternalStructDef")] ExternalStruct);

//         // let helper = Option::deserialize(deserializer)?;
//         // Ok(helper.map(|Helper(external)| external))
//         // TODO
//         Ok(vec![])
//     }
// }

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

// #[derive(Serialize, Deserialize)]
// #[serde(remote = "ExpressionOperator")]
// enum ExpressionOperatorRef {
//     And,
//     Or,
//     Add,
//     Subtract,
//     Multiply,
//     Divide,
//     Remainder,
// }
// #[derive(Serialize, Deserialize)]
// #[serde(remote = "Expression")]
// enum ExpressionRef {
//     Null,
//     Number(i64),
//     String(String),
//     Expression(Box<ExpressionObjRef>),
//     Invert(Box<ExpressionRef>),
// }

// #[derive(Serialize, Deserialize)]
// #[serde(remote = "ExpressionObj")]
// struct ExpressionObjRef {
//     l: ExpressionRef,
//     o: ExpressionOperatorRef,
//     r: ExpressionRef,
// }

// #[derive(Serialize, Deserialize)]
// #[serde(remote = "TimelineEnable")]
// struct TimelineEnableRef {
//     #[serde(with = "ExpressionRef")]
//     enable_start: Option<Expression>,
//     #[serde(with = "ExpressionRef")]
//     enable_end: Option<Expression>,
//     #[serde(with = "ExpressionRef")]
//     enable_while: Option<Expression>,
//     #[serde(with = "ExpressionRef")]
//     duration: Option<Expression>,
//     #[serde(with = "ExpressionRef")]
//     repeating: Option<Expression>,
// }
