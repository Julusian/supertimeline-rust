pub fn mangle_json_enable(raw_str: &str) -> serde_json::Result<String> {
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
                        }
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
