use std::cmp::Ordering;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Mutex;
use supertimeline::TimelineObjectInstance;
use supertimeline::TimelineState;

pub fn assert_instances(
    result: &HashMap<String, Rc<Mutex<TimelineObjectInstance>>>,
    expected: &Vec<Rc<TimelineObjectInstance>>,
) {
    let mut result_vec: Vec<TimelineObjectInstance> = result
        .values()
        .map(|v| {
            let v2 = v.try_lock().unwrap();
            v2.clone()
        })
        .collect();
    result_vec.sort_by(|a, b| {
        if a.start < b.start {
            Ordering::Less
        } else if a.start > b.start {
            Ordering::Greater
        } else {
            b.id.cmp(&a.id)
        }
    });

    assert_instances2(&result_vec, expected);
}

pub fn assert_instances2(
    result: &Vec<TimelineObjectInstance>,
    expected: &Vec<Rc<TimelineObjectInstance>>,
) {
    assert_eq!(result.len(), expected.len());

    for (val, exp) in result.iter().zip(expected) {
        assert_eq!(val.start, exp.start);
        assert_eq!(val.end, exp.end);

        if exp.original_start.is_some() {
            assert_eq!(val.original_start, exp.original_start);
        }
        if exp.original_end.is_some() {
            assert_eq!(val.original_end, exp.original_end);
        }

        // TODO - more props
    }
}

pub fn assert_obj_on_layer(state: &TimelineState, layer: &str, id: &str) {
    let obj = state
        .layers
        .get(layer)
        .expect(&format!("Expected '{}' on layer '{}'", id, layer));

    assert_eq!(obj.object_id, id.to_string());
}
