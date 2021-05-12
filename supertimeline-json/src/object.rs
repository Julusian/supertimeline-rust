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

    fn mangle_json_enable(raw_str: &str) -> serde_json::Result<String> {
        let mut parsed = serde_json::from_str::<serde_json::Value>(raw_str)?;

        if let Some(arr) = parsed.as_array_mut() {
            for obj in arr {
                mangle_json_obj(obj)?;
            }
        } else {
            mangle_json_obj(&mut parsed)?;
        }

        serde_json::to_string(&parsed)
    }

    fn mangle_json_obj(obj: &mut serde_json::Value) -> serde_json::Result<()> {
        match obj {
            serde_json::Value::Object(map) => {
                if let Some(enable) = map.get_mut("enable") {
                    if !enable.is_array() {
                        match enable {
                            serde_json::Value::Object(en) => {
                                let v = en.get("start");
                                if let Some(v) = v {
                                    if let Some(v) = v.as_f64() {
                                        let v2 = serde_json::to_value(v.round() as i64)?;
                                        en.insert("start".to_string(), v2);
                                    }
                                }
                            },
                            _ => {}
                        }

                        let val = serde_json::to_value(vec![enable])?;
                        // let val = serde_json::to_value(Vec::<u64>::new())?;
                        map.insert("enable".to_string(), val);
                    }
                }

                if let Some(priority) = map.get("priority") {
                    if let Some(priority) = priority.as_i64() {
                        let val = serde_json::to_value(priority * 1000)?;
                        map.insert("priority".to_string(), val);
                    } else if let Some(priority) = priority.as_u64() {
                        let val = serde_json::to_value(priority * 1000)?;
                        map.insert("priority".to_string(), val);
                    } else if let Some(priority) = priority.as_f64() {
                        let val = serde_json::to_value((priority * 1000.0) as i64)?;
                        map.insert("priority".to_string(), val);
                    }
                }

                if let Some(children) = map.get_mut("children") {
                    if let Some(arr) = children.as_array_mut() {
                        for obj in arr {
                            mangle_json_obj(obj)?;
                        }
                    }
                }

                if let Some(keyframes) = map.get_mut("keyframes") {
                    if let Some(arr) = keyframes.as_array_mut() {
                        for obj in arr {
                            mangle_json_obj(obj)?;
                        }
                    }
                }
            }
            _ => {}
        };
        Ok(())
    }

    #[test]
    // #[ignore]
    fn parse() {
        let j = mangle_json_enable(include_str!("dump.json")).unwrap();

        let mut deserializer = serde_json::Deserializer::from_str(&j);
        let parsed:Vec<JsonTimelineObject >= serde_path_to_error::deserialize(&mut deserializer).unwrap();


        println!("got back {:#?}", parsed);
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

        println!("it took {}", duration.as_millis());
        panic!();
    }
}
