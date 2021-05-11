use serde::{Deserialize, Serialize};
use supertimeline::Expression;
use supertimeline::ExpressionObj;
use supertimeline::ExpressionOperator;
use supertimeline::IsTimelineKeyframe;
use supertimeline::IsTimelineObject;
use supertimeline::TimelineEnable;

#[derive(Serialize, Deserialize)]
pub struct JsonTimelineObject {
    pub id: String,
    // #[serde(default, with = "vec_timeline_enable")]
    pub enable: Vec<TimelineEnable>,
    pub layer: String,
    // pub keyframes: Vec<Box<dyn IsTimelineKeyframe>>,
    pub classes: Vec<String>,
    pub disabled: bool,
    // pub children: Option<Vec<Box<dyn IsTimelineObject>>>,
    pub priority: u64,
}
impl IsTimelineObject for JsonTimelineObject {
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
        // Some(self.keyframes.as_ref())
        None
    }
    fn classes(&self) -> Option<&Vec<String>> {
        Some(self.classes.as_ref())
    }
    fn disabled(&self) -> bool {
        self.disabled
    }
    fn children(&self) -> Option<&Vec<Box<dyn IsTimelineObject>>> {
        // self.children.as_ref()
        None
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

    #[test]
    fn demo() {
        let src = JsonTimelineObject {
            id: "test".to_string(),
            layer: "lay".to_string(),
            // pub keyframes: Vec<Box<dyn IsTimelineKeyframe>>,
            classes: vec![],
            disabled: false,
            // pub children: Option<Vec<Box<dyn IsTimelineObject>>>,
            priority: 0,
            enable: vec![TimelineEnable {
                enable_start: Some(Expression::Number(4)),
                enable_end: Some(Expression::Null),
                ..Default::default()
            }],
        };

        let j = serde_json::to_string(&src).unwrap();

        // Print, write to a file, or send to an HTTP server.
        println!("output {}", j);
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
