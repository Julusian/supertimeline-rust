use crate::state;
use crate::expression::Expression;
use regex::Regex;
use crate::state::{Time, TimelineObjectInstance};
use std::collections::HashSet;

lazy_static::lazy_static! {
    static ref MATCH_ID_REGEX: Regex = Regex::new(r"^\W*#([^.]+)(.*)").unwrap();
    static ref MATCH_CLASS_REGEX: Regex = Regex::new(r"^\W*\.([^.]+)(.*)").unwrap();
    static ref MATCH_LAYER_REGEX: Regex = Regex::new(r"^\W*\$([^.]+)(.*)").unwrap();
}

#[derive(PartialEq, Debug, Clone)]
pub enum ObjectRefType {
    Start,
    End,
    Duration,
}

pub struct LookupExpressionResult {
    //pub instances: Vec
    pub instances: Option<TimeWithReference>,
    pub instances2: Option<Vec<TimelineObjectInstance>>,
    pub all_references: Vec<String>,
}
impl LookupExpressionResult {
    pub fn Null() -> LookupExpressionResult{
        LookupExpressionResult {
            instances: None,
            instances2: None,
            all_references: Vec::new(),
        }
    }
}

pub struct TimeWithReference {
    pub value: Time,
    pub references: HashSet<String>,
}

struct MatchExpressionReferences {
    pub remaining_expression: String,
    pub object_ids_to_reference: Vec<String>, // TODO - should this be a set?
    pub all_references: Vec<String>,
}
fn match_expression_references (resolved_timeline: &state::ResolvedTimeline, expr_str: &str) -> Option<MatchExpressionReferences> {
    if let Some(id_match) = MATCH_ID_REGEX.captures(expr_str) {
        let id = id_match.get(1).unwrap().as_str();

        Some(MatchExpressionReferences{
            remaining_expression: id_match.get(2).unwrap().as_str().to_string(),
            object_ids_to_reference: vec![id.to_string()],
            all_references: vec![format!("#{}", id)],
        })
    } else if let Some(class_match) = MATCH_CLASS_REGEX.captures(expr_str) {
        let class_name = class_match.get(1).unwrap().as_str();

        Some(MatchExpressionReferences{
            remaining_expression: class_match.get(2).unwrap().as_str().to_string(),
            object_ids_to_reference: resolved_timeline.classes.get(class_name).cloned().unwrap_or_default(),
            all_references: vec![format!(".{}", class_name)],
        })
    } else if let Some(layer_match) = MATCH_LAYER_REGEX.captures(expr_str) {
        let layer_id = layer_match.get(1).unwrap().as_str();

        Some(MatchExpressionReferences{
            remaining_expression: layer_match.get(2).unwrap().as_str().to_string(),
            object_ids_to_reference: resolved_timeline.layers.get(layer_id).cloned().unwrap_or_default(),
            all_references: vec![format!("${}", layer_id)],
        })

    } else {
        None
    }
}

pub fn resolve_timeline_obj(resolved_timeline: &state::ResolvedTimeline, obj: &state::ResolvedTimelineObject) {

}

pub fn lookup_expression(resolved_timeline: &state::ResolvedTimeline, obj: &mut state::ResolvedTimelineObject, expr_str: &str, default_ref_type: ObjectRefType) -> LookupExpressionResult {
    // TODO
    // if (isConstant(expr)) {
    //     if (expr.match(/^true$/i)) {
    //     return {
    //     instances: {
    //     value: 0,
    //     references: []
    //     },
    //     allReferences: []
    //     }
    //     } else if (expr.match(/^false$/i)) {
    //     return {
    //     instances: [],
    //     allReferences: []
    //     }
    //     }
    // }

    // Look up string
    let invert = false;
    let ignoreFirstIfZero = false;
    let mut referencedObjs: Vec<&state::ResolvedTimelineObject> = Vec::new();

    if let Some(expression_references) = match_expression_references(resolved_timeline, expr_str) {
        let mut referencedObjs: Vec<&state::ResolvedTimelineObject> = Vec::new();
        for ref_obj_id in &expression_references.object_ids_to_reference {
            if ref_obj_id != &obj.id {
                if let Some(ref_obj) = resolved_timeline.objects.get(ref_obj_id) {
                    referencedObjs.push(ref_obj);
                }
            } else {
                if obj.resolving {
                    obj.isSelfReferencing = true
                }
            }
        }

        if obj.isSelfReferencing {
            // Exclude any self-referencing objects:
            referencedObjs = referencedObjs.into_iter().filter(|&ref_obj| !ref_obj.isSelfReferencing).collect::<Vec<_>>();
        }

        if referencedObjs.len() > 0 {
            let refType = {
                // TODO - these should be looser regex
                if expression_references.remaining_expression == "start" {
                    ObjectRefType::Start
                } else if expression_references.remaining_expression == "end" {
                    ObjectRefType::End
                } else if expression_references.remaining_expression == "duration" {
                    ObjectRefType::Duration
                } else {
                    default_ref_type
                }
            };

            if refType == ObjectRefType::Duration {
                let mut instance_durations = Vec::new();
                for ref_obj in referencedObjs {
                    resolve_timeline_obj(resolved_timeline, ref_obj);
                    if ref_obj.resolved {
                        if obj.isSelfReferencing && ref_obj.isSelfReferencing {
                            // If the querying object is self-referencing, exclude any other self-referencing objects,
                            // ignore the object
                        } else {
                            if let Some(first_instance) = ref_obj.resolved_instances.first() {
                                if let Some(end) = first_instance.end {
                                    let duration = end - first_instance.start;
                                    let mut references = HashSet::new();
                                    references.extend(first_instance.references.iter().cloned());
                                    references.insert(ref_obj.id.clone());

                                    instance_durations.push(TimeWithReference {
                                        value: duration,
                                        references,
                                    })
                                }
                            }
                        }
                    }
                }

                let mut first_duration: Option<TimeWithReference> = None;
                for d in instance_durations {
                    match &first_duration {
                        Some(first2) => if d.value < first2.value {
                            first_duration = Some(d);
                        },
                        None => first_duration = Some(d)
                    };
                }

                return LookupExpressionResult{
                    instances: first_duration,
                    instances2: None,
                    all_references: expression_references.all_references,
                }
            } else {
                let mut return_instances = Vec::new();

                let invertAndIgnoreFirstIfZero = refType == ObjectRefType::End;

                for ref_obj in referencedObjs {
                    resolve_timeline_obj(resolved_timeline, ref_obj);
                    if ref_obj.resolved {
                        if obj.isSelfReferencing && ref_obj.isSelfReferencing {
                            // If the querying object is self-referencing, exclude any other self-referencing objects,
                            // ignore the object
                        } else {
                            return_instances.extend(ref_obj.resolved_instances.iter().cloned());
                        }
                    }
                }

                if return_instances.len() > 0 {
                    if invertAndIgnoreFirstIfZero {
                        return_instances = invert_instances(return_instances);

                        if let Some(first) = return_instances.first() {
                            if first.start == 0 {
                                return_instances.remove(0);
                            }
                        }
                    } else {
                        return_instances = clean_instances(return_instances, true, true);
                    }

                    return LookupExpressionResult {
                        instances2: Some(return_instances),
                        instances: None,
                        all_references: expression_references.all_references,
                    }
                } else {
                    return LookupExpressionResult {
                        instances: None,
                        instances2: None,
                        all_references: expression_references.all_references,
                    }
                }
            }

        } else {
            return LookupExpressionResult::Null();
        }

        // TODO
        LookupExpressionResult::Null() // TODO
    } else {
        LookupExpressionResult::Null()
    }

}