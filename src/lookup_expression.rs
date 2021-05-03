use crate::events::sort_events;
use crate::expression::{Expression, ExpressionObj, ExpressionOperator};
use crate::instance::TimelineObjectInstance;
use crate::resolver::{resolve_timeline_obj, ObjectRefType, TimeWithReference};
use crate::state;
use crate::util::{
    clean_instances, getId, invert_instances, join_caps, join_hashset, join_maybe_hashset,
    operate_on_arrays, Time,
};
use regex::Regex;
use std::collections::HashSet;
use std::iter::FromIterator;

lazy_static::lazy_static! {
    static ref MATCH_ID_REGEX: Regex = Regex::new(r"^\W*#([^.]+)(.*)").unwrap();
    static ref MATCH_CLASS_REGEX: Regex = Regex::new(r"^\W*\.([^.]+)(.*)").unwrap();
    static ref MATCH_LAYER_REGEX: Regex = Regex::new(r"^\W*\$([^.]+)(.*)").unwrap();
}

pub enum LookupExpressionResultType {
    Instances(Vec<TimelineObjectInstance>),
    TimeRef(TimeWithReference),
    Null,
}

pub struct LookupExpressionResult {
    //pub instances: Vec
    // pub instances: Option<TimeWithReference>,
    // pub instances2: Option<Vec<TimelineObjectInstance>>,
    pub result: LookupExpressionResultType,
    pub all_references: HashSet<String>,
}
impl LookupExpressionResult {
    pub fn Null() -> LookupExpressionResult {
        LookupExpressionResult {
            result: LookupExpressionResultType::Null,
            all_references: HashSet::new(),
        }
    }
}

pub fn lookup_expression(
    resolved_timeline: &state::ResolvedTimeline,
    obj: &mut state::ResolvedTimelineObject,
    expr: &Expression,
    default_ref_type: &ObjectRefType,
) -> LookupExpressionResult {
    match expr {
        Expression::Null => LookupExpressionResult::Null(),
        Expression::Number(time) => LookupExpressionResult {
            result: LookupExpressionResultType::TimeRef(TimeWithReference {
                value: 0i64.max(*time).unsigned_abs(), // Clamp to not go below 0
                references: HashSet::new(),
            }),
            all_references: HashSet::new(),
        },
        Expression::String(str) => {
            lookup_expression_str(resolved_timeline, obj, str, default_ref_type)
        }
        Expression::Expression(exprObj) => {
            lookup_expression_obj(resolved_timeline, obj, exprObj, default_ref_type)
        }
        Expression::Invert(innerExpr) => {
            let inner_res = lookup_expression(resolved_timeline, obj, innerExpr, default_ref_type);

            LookupExpressionResult {
                instances: inner_res.instances, // We can't invert a value
                instances2: inner_res
                    .instances2
                    .and_then(|instances| Some(invert_instances(&instances))),
                all_references: inner_res.all_references,
            }
        }
    }
}

struct MatchExpressionReferences {
    pub remaining_expression: String,
    pub object_ids_to_reference: Vec<String>, // TODO - should this be a set?
    pub all_references: HashSet<String>,
}
fn match_expression_references(
    resolved_timeline: &state::ResolvedTimeline,
    expr_str: &str,
) -> Option<MatchExpressionReferences> {
    if let Some(id_match) = MATCH_ID_REGEX.captures(expr_str) {
        let id = id_match.get(1).unwrap().as_str();

        Some(MatchExpressionReferences {
            remaining_expression: id_match.get(2).unwrap().as_str().to_string(),
            object_ids_to_reference: vec![id.to_string()],
            all_references: set![format!("#{}", id)],
        })
    } else if let Some(class_match) = MATCH_CLASS_REGEX.captures(expr_str) {
        let class_name = class_match.get(1).unwrap().as_str();

        Some(MatchExpressionReferences {
            remaining_expression: class_match.get(2).unwrap().as_str().to_string(),
            object_ids_to_reference: resolved_timeline
                .classes
                .get(class_name)
                .cloned()
                .unwrap_or_default(),
            all_references: set![format!(".{}", class_name)],
        })
    } else if let Some(layer_match) = MATCH_LAYER_REGEX.captures(expr_str) {
        let layer_id = layer_match.get(1).unwrap().as_str();

        Some(MatchExpressionReferences {
            remaining_expression: layer_match.get(2).unwrap().as_str().to_string(),
            object_ids_to_reference: resolved_timeline
                .layers
                .get(layer_id)
                .cloned()
                .unwrap_or_default(),
            all_references: set![format!("${}", layer_id)],
        })
    } else {
        None
    }
}

fn lookup_expression_str(
    resolved_timeline: &state::ResolvedTimeline,
    obj: &mut state::ResolvedTimelineObject,
    expr_str: &str,
    default_ref_type: &ObjectRefType,
) -> LookupExpressionResult {
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
            referencedObjs = referencedObjs
                .into_iter()
                .filter(|&ref_obj| !ref_obj.isSelfReferencing)
                .collect::<Vec<_>>();
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
                    default_ref_type.clone()
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

                let mut result = LookupExpressionResultType::Null;
                for d in instance_durations {
                    match &result {
                        Some(first2) => {
                            if d.value < first2.value {
                                result = LookupExpressionResultType::TimeRef(d);
                            }
                        }
                        None => result = LookupExpressionResultType::TimeRef(d),
                    };
                }

                return LookupExpressionResult {
                    result,
                    all_references: expression_references.all_references,
                };
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
                        return_instances = invert_instances(&return_instances);

                        if let Some(first) = return_instances.first() {
                            if first.start == 0 {
                                return_instances.remove(0);
                            }
                        }
                    } else {
                        return_instances = clean_instances(&return_instances, true, true);
                    }

                    return LookupExpressionResult {
                        result: LookupExpressionResultType::Instances(return_instances),
                        all_references: expression_references.all_references,
                    };
                } else {
                    return LookupExpressionResult {
                        result: LookupExpressionResultType::Null,
                        all_references: expression_references.all_references,
                    };
                }
            }
        } else {
            LookupExpressionResult::Null()
        }
    } else {
        LookupExpressionResult::Null()
    }
}

fn lookup_expression_obj(
    resolved_timeline: &state::ResolvedTimeline,
    obj: &mut state::ResolvedTimelineObject,
    expr: &ExpressionObj,
    default_ref_type: &ObjectRefType,
) -> LookupExpressionResult {
    if expr.l == Expression::Null || expr.r == Expression::Null {
        LookupExpressionResult::Null()
    } else {
        let l = lookup_expression(resolved_timeline, obj, &expr.l, default_ref_type);
        let r = lookup_expression(resolved_timeline, obj, &expr.r, default_ref_type);

        let all_references = HashSet::from_iter(
            l.all_references
                .iter()
                .chain(r.all_references.iter())
                .cloned(),
        );

        if expr.o == ExpressionOperator::And || expr.o == ExpressionOperator::Or {
            let events = {
                let mut events = Vec::new();
                events.extend(get_side_events(&l, true));
                events.extend(get_side_events(&r, false));
                sort_events(&mut events);
                events
            };

            let mut left_value = l
                .instances
                .and_then(|v| Some(v.value != 0))
                .unwrap_or(false);
            let mut right_value = r
                .instances
                .and_then(|v| Some(v.value != 0))
                .unwrap_or(false);

            let calc_result = match &expr.o {
                ExpressionOperator::And => |a, b| a && b,
                ExpressionOperator::Or => |a, b| a || b,
                _ => |a, b| false,
            };

            let mut result_value = calc_result(left_value, right_value);
            let result_references = join_hashset(&l.all_references, &r.all_references);

            let mut left_instance = None;
            let mut right_instance = None;

            let mut instances = Vec::new();
            let push_instance = |time, value, references, caps| {
                if value {
                    instances.push(TimelineObjectInstance {
                        id: getId(),
                        start: time,
                        end: None,
                        references,
                        caps,

                        isFirst: false,
                        originalStart: None,
                        originalEnd: None,
                        fromInstanceId: None,
                    });
                } else if let Some(last_instance) = instances.last_mut() {
                    last_instance.end = time;
                    // don't update reference on end
                }
            };
            push_instance(0, result_value, result_references, Vec::new());

            for (i, event) in events.iter().enumerate() {
                let next_time = events
                    .get(i + 1)
                    .and_then(|e| Some(e.time))
                    .unwrap_or(Time::MAX);

                if event.is_left {
                    left_value = event.is_start;
                    left_instance = Some(event.instance);
                } else {
                    right_value = event.is_start;
                    right_instance = Some(event.instance);
                }

                if next_time != event.time {
                    let new_result_value = calc_result(left_value, right_value);

                    if new_result_value != result_value {
                        let result_references = join_maybe_hashset(
                            left_instance.and_then(|i| Some(&i.references)),
                            right_instance.and_then(|i| Some(&i.references)),
                        );
                        let result_caps = join_caps(
                            left_instance
                                .and_then(|i| Some(&i.caps))
                                .unwrap_or_else(|| &Vec::new()),
                            right_instance
                                .and_then(|i| Some(&i.caps))
                                .unwrap_or_else(|| &Vec::new()),
                        );

                        push_instance(event.time, new_result_value, result_references, result_caps);
                        result_value = new_result_value;
                    }
                }
            }

            LookupExpressionResult {
                result: LookupExpressionResultType::Instances(instances),
                all_references,
            }
        } else {
            let operator = match expr.o {
                ExpressionOperator::Add => |a, b| {
                    Some(TimeWithReference {
                        value: a.value + b.value,
                        references: join_hashset(&a.references, &b.references),
                    })
                },
                ExpressionOperator::Subtract => |a, b| {
                    Some(TimeWithReference {
                        value: a.value - b.value,
                        references: join_hashset(&a.references, &b.references),
                    })
                },
                ExpressionOperator::Multiply => |a, b| {
                    Some(TimeWithReference {
                        value: a.value * b.value,
                        references: join_hashset(&a.references, &b.references),
                    })
                },
                ExpressionOperator::Divide => |a, b| {
                    Some(TimeWithReference {
                        value: a.value / b.value, // TODO - can this panic?
                        references: join_hashset(&a.references, &b.references),
                    })
                },
                ExpressionOperator::Remainder => |a, b| {
                    Some(TimeWithReference {
                        value: a.value % b.value, // TODO - can this panic?
                        references: join_hashset(&a.references, &b.references),
                    })
                },
                _ => |a, b| None,
            };

            let result = operate_on_arrays(&l.result, &r.result, operator);
            LookupExpressionResult {
                result: LookupExpressionResultType::Instances(result),
                all_references,
            }
        }
    }
}

struct SideEvent<'a> {
    time: Time,
    is_left: bool,
    is_start: bool,
    // references: &'a Vec<String>,
    instance: &'a TimelineObjectInstance,
}

fn get_side_events(res: &LookupExpressionResult, is_left: bool) -> Vec<SideEvent> {
    let mut events = Vec::new();

    if let Some(instances) = &res.instances2 {
        for instance in instances {
            if let Some(end) = instance.end {
                if end == instance.start {
                    // event doesn't actually exist...
                    break;
                }

                events.push(SideEvent {
                    is_left,
                    time: end,
                    instance,
                    is_start: false,
                });
            }

            events.push(SideEvent {
                is_left,
                time: instance.start,
                instance,
                is_start: true,
            });
        }
    }

    events
}
