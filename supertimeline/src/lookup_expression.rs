use crate::caps::{Cap, CapsBuilder};
use crate::events::{IsEvent, VecIsEventExt};
use crate::expression::{Expression, ExpressionObj, ExpressionOperator};
use crate::instance::TimelineObjectInstance;
use crate::references::ReferencesBuilder;
use crate::resolver::ResolveError;
use crate::resolver::ResolverContext;
use crate::resolver::{
    ObjectRefType, ResolvingTimelineObject, TimeWithReference, TimelineObjectResolvingStatus,
};
use crate::util::{clean_instances, invert_instances, operate_on_arrays, Time};
use regex::Regex;
use std::collections::HashSet;

lazy_static::lazy_static! {
    static ref MATCH_ID_REGEX: Regex = Regex::new(r"^\W*#([^.]+)(.*)").unwrap();
    static ref MATCH_CLASS_REGEX: Regex = Regex::new(r"^\W*\.([^.]+)(.*)").unwrap();
    static ref MATCH_LAYER_REGEX: Regex = Regex::new(r"^\W*\$([^.]+)(.*)").unwrap();
}

#[derive(Debug)]
pub enum LookupExpressionResultType {
    Instances(Vec<TimelineObjectInstance>),
    TimeRef(TimeWithReference),
    Null,
}

pub struct LookupExpressionResult {
    pub result: LookupExpressionResultType,
    pub all_references: HashSet<String>,
}
impl LookupExpressionResult {
    pub fn null() -> LookupExpressionResult {
        LookupExpressionResult {
            result: LookupExpressionResultType::Null,
            all_references: HashSet::new(),
        }
    }
}

pub fn lookup_expression(
    ctx: &ResolverContext,
    obj: &ResolvingTimelineObject,
    expr: &Expression,
    default_ref_type: &ObjectRefType,
) -> Result<LookupExpressionResult, ResolveError> {
    match expr {
        Expression::Null => Ok(LookupExpressionResult::null()),
        Expression::Number(time) => Ok(LookupExpressionResult {
            result: LookupExpressionResultType::TimeRef(TimeWithReference {
                value: 0i64.max(*time).unsigned_abs(), // Clamp to not go below 0
                references: HashSet::new(),
            }),
            all_references: HashSet::new(),
        }),
        Expression::Bool(val) => {
            if *val {
                Ok(LookupExpressionResult {
                    result: LookupExpressionResultType::TimeRef(TimeWithReference {
                        value: 0,
                        references: HashSet::new(),
                    }),
                    all_references: HashSet::new(),
                })
            } else {
                Ok(LookupExpressionResult {
                    result: LookupExpressionResultType::Instances(vec![]),
                    all_references: HashSet::new(),
                })
            }
        }
        Expression::String(str) => lookup_expression_str(ctx, obj, str, default_ref_type),
        Expression::Expression(expr_obj) => {
            lookup_expression_obj(ctx, obj, expr_obj, default_ref_type)
        }
        Expression::Invert(inner_expr) => {
            let inner_res = lookup_expression(ctx, obj, inner_expr, default_ref_type)?;

            let inner_res2 = match inner_res.result {
                LookupExpressionResultType::Null => {
                    LookupExpressionResultType::Instances(invert_instances(ctx, &[]))
                }
                LookupExpressionResultType::TimeRef(time_ref) => {
                    LookupExpressionResultType::TimeRef(time_ref)
                } // Can't invert a time
                LookupExpressionResultType::Instances(instances) => {
                    LookupExpressionResultType::Instances(invert_instances(ctx, &instances))
                }
            };

            Ok(LookupExpressionResult {
                result: inner_res2,
                all_references: inner_res.all_references,
            })
        }
    }
}

struct MatchExpressionReferences {
    pub remaining_expression: String,
    pub object_ids_to_reference: Vec<String>, // TODO: this could be a set, but it isn't modified after creation so is safe as is
    pub all_references: HashSet<String>,
}
fn match_expression_references(
    ctx: &ResolverContext,
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
            object_ids_to_reference: ctx
                .get_object_ids_for_class(class_name)
                .cloned()
                .unwrap_or_default(),
            all_references: set![format!(".{}", class_name)],
        })
    } else if let Some(layer_match) = MATCH_LAYER_REGEX.captures(expr_str) {
        let layer_id = layer_match.get(1).unwrap().as_str();

        Some(MatchExpressionReferences {
            remaining_expression: layer_match.get(2).unwrap().as_str().to_string(),
            object_ids_to_reference: ctx
                .get_object_ids_for_layer(layer_id)
                .cloned()
                .unwrap_or_default(),
            all_references: set![format!("${}", layer_id)],
        })
    } else {
        None
    }
}

fn lookup_expression_str(
    ctx: &ResolverContext,
    obj: &ResolvingTimelineObject,
    expr_str: &str,
    default_ref_type: &ObjectRefType,
) -> Result<LookupExpressionResult, ResolveError> {
    // Note: bool expressions are 'parsed' elsewhere

    if let Some(expression_references) = match_expression_references(ctx, expr_str) {
        let mut referenced_objs: Vec<&ResolvingTimelineObject> = Vec::new();
        for ref_obj_id in &expression_references.object_ids_to_reference {
            if ref_obj_id.eq(&obj.info.id) {
                let mut locked = obj.resolved.write().unwrap(); // TODO - handle error
                match &mut *locked {
                    TimelineObjectResolvingStatus::Pending => {
                        // This is fine, we will resolve it shortly
                    }
                    TimelineObjectResolvingStatus::InProgress(progress) => {
                        progress.is_self_referencing = true;
                    }
                    TimelineObjectResolvingStatus::Complete(_) => {
                        // This is fine. Very good actually
                    }
                };
            } else {
                if let Some(ref_obj) = ctx.get_object(ref_obj_id) {
                    referenced_objs.push(ref_obj);
                }
            }
        }

        if obj.is_self_referencing() {
            // Exclude any self-referencing objects:
            referenced_objs = referenced_objs
                .into_iter()
                .filter(|ref_obj| !ref_obj.is_self_referencing())
                .collect::<Vec<_>>();
        }

        if !referenced_objs.is_empty() {
            let ref_type = {
                // TODO - these should be looser regex
                if expression_references.remaining_expression == ".start" {
                    ObjectRefType::Start
                } else if expression_references.remaining_expression == ".end" {
                    ObjectRefType::End
                } else if expression_references.remaining_expression == ".duration" {
                    ObjectRefType::Duration
                } else {
                    default_ref_type.clone()
                }
            };

            if ref_type == ObjectRefType::Duration {
                let mut instance_durations = Vec::new();
                for ref_obj in referenced_objs {
                    ctx.resolve_object(ref_obj)?;

                    let obj_is_self_referencing = obj.is_self_referencing();
                    let locked_ref = ref_obj.resolved.read().unwrap(); // TODO - handle error
                    match &*locked_ref {
                        TimelineObjectResolvingStatus::Pending => {
                            // Nothing to do
                        }
                        TimelineObjectResolvingStatus::InProgress(_) => {
                            // Nothing to do
                        }
                        TimelineObjectResolvingStatus::Complete(res) => {
                            if obj_is_self_referencing && res.is_self_referencing {
                                // If the querying object is self-referencing, exclude any other self-referencing objects,
                                // ignore the object
                            } else {
                                if let Some(first_instance) = res.instances.first() {
                                    if let Some(end) = first_instance.end {
                                        let duration = end - first_instance.start;
                                        let mut references = HashSet::new();
                                        references
                                            .extend(first_instance.references.iter().cloned());
                                        references.insert(ref_obj.info.id.clone());

                                        instance_durations.push(TimeWithReference {
                                            value: duration,
                                            references,
                                        })
                                    }
                                }
                            }
                        }
                    }
                }

                let mut result: Option<TimeWithReference> = None;
                for d in instance_durations {
                    match &result {
                        Some(first2) => {
                            if d.value < first2.value {
                                result = Some(d);
                            }
                        }
                        None => result = Some(d),
                    };
                }

                Ok(LookupExpressionResult {
                    result: result
                        .map(|time_ref| LookupExpressionResultType::TimeRef(time_ref))
                        .unwrap_or(LookupExpressionResultType::Null),
                    all_references: expression_references.all_references,
                })
            } else {
                let mut return_instances: Vec<TimelineObjectInstance> = Vec::new();

                let invert_and_ignore_first_if_zero = ref_type == ObjectRefType::End;

                for ref_obj in referenced_objs {
                    ctx.resolve_object(ref_obj)?;

                    let obj_is_self_referencing = obj.is_self_referencing();
                    let locked_ref = ref_obj.resolved.read().unwrap(); // TODO - handle error
                    match &*locked_ref {
                        TimelineObjectResolvingStatus::Pending => {
                            // Nothing to do
                        }
                        TimelineObjectResolvingStatus::InProgress(_) => {
                            // Nothing to do
                        }
                        TimelineObjectResolvingStatus::Complete(res) => {
                            if obj_is_self_referencing && res.is_self_referencing {
                                // If the querying object is self-referencing, exclude any other self-referencing objects,
                                // ignore the object
                            } else {
                                return_instances.extend(res.instances.iter().cloned());
                            }
                        }
                    }
                }

                if !return_instances.is_empty() {
                    if invert_and_ignore_first_if_zero {
                        return_instances = invert_instances(ctx, &return_instances);

                        if let Some(first) = return_instances.first() {
                            if first.start == 0 {
                                return_instances.remove(0);
                            }
                        }
                    } else {
                        return_instances = clean_instances(ctx, &return_instances, true, true);
                    }

                    Ok(LookupExpressionResult {
                        result: LookupExpressionResultType::Instances(return_instances),
                        all_references: expression_references.all_references,
                    })
                } else {
                    Ok(LookupExpressionResult {
                        result: LookupExpressionResultType::Null,
                        all_references: expression_references.all_references,
                    })
                }
            }
        } else {
            Ok(LookupExpressionResult {
                result: LookupExpressionResultType::Null,
                all_references: expression_references.all_references,
            })
        }
    } else {
        Ok(LookupExpressionResult::null())
    }
}

fn lookup_expression_obj(
    ctx: &ResolverContext,
    obj: &ResolvingTimelineObject,
    expr: &ExpressionObj,
    default_ref_type: &ObjectRefType,
) -> Result<LookupExpressionResult, ResolveError> {
    if expr.l == Expression::Null || expr.r == Expression::Null {
        Ok(LookupExpressionResult::null())
    } else {
        let l = lookup_expression(ctx, obj, &expr.l, default_ref_type)?;
        let r = lookup_expression(ctx, obj, &expr.r, default_ref_type)?;

        let all_references = l
            .all_references
            .iter()
            .chain(r.all_references.iter())
            .cloned()
            .collect();

        if expr.o == ExpressionOperator::And || expr.o == ExpressionOperator::Or {
            let events = {
                let mut events = Vec::new();
                // TODO - this looks to be getting a load of 'junk' events?
                events.extend(get_side_events(&l, true));
                events.extend(get_side_events(&r, false));
                events.sort();
                events
            };

            let mut left_value = if let LookupExpressionResultType::TimeRef(time_ref) = &l.result {
                time_ref.value != 0
            } else {
                false
            };
            let mut right_value = if let LookupExpressionResultType::TimeRef(time_ref) = &r.result {
                time_ref.value != 0
            } else {
                false
            };

            let calc_result: fn(a: bool, b: bool) -> bool = match &expr.o {
                ExpressionOperator::And => |a, b| a && b,
                ExpressionOperator::Or => |a, b| a || b,
                _ => |_a, _b| false,
            };

            let mut result_value = calc_result(left_value, right_value);
            let result_references = ReferencesBuilder::new()
                .add(&l.all_references)
                .add(&r.all_references)
                .done();

            let mut left_instance = None;
            let mut right_instance = None;

            let mut instances = Vec::new();
            let mut push_instance =
                |time: Time, value: bool, references: HashSet<String>, caps: Vec<Cap>| {
                    if value {
                        instances.push(TimelineObjectInstance {
                            id: ctx.generate_id(),
                            start: time,
                            end: None,
                            references,
                            caps,

                            is_first: false,
                            original_start: None,
                            original_end: None,
                            from_instance_id: None,
                        });
                    } else if let Some(last_instance) = instances.last_mut() {
                        last_instance.end = Some(time);
                        // don't update reference on end
                    }
                };
            push_instance(0, result_value, result_references, Vec::new());

            for (i, event) in events.iter().enumerate() {
                let next_time = events.get(i + 1).map(|e| e.time).unwrap_or(Time::MAX);

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
                        let result_references = ReferencesBuilder::new()
                            .add_some(left_instance.map(|i| &i.references))
                            .add_some(right_instance.map(|i| &i.references))
                            .done();

                        let result_caps = CapsBuilder::new()
                            .add_some(left_instance.map(|i| i.caps.iter().cloned()))
                            .add_some(right_instance.map(|i| i.caps.iter().cloned()))
                            .done();

                        push_instance(event.time, new_result_value, result_references, result_caps);
                        result_value = new_result_value;
                    }
                }
            }

            Ok(LookupExpressionResult {
                result: LookupExpressionResultType::Instances(instances),
                all_references,
            })
        } else {
            let operator: fn(a: &TimeWithReference, b: &TimeWithReference) -> Option<Time> =
                match expr.o {
                    ExpressionOperator::Add => |a, b| Some(a.value + b.value),
                    ExpressionOperator::Subtract => |a, b| Some(a.value - b.value),
                    ExpressionOperator::Multiply => |a, b| Some(a.value * b.value),
                    ExpressionOperator::Divide => |a, b| {
                        Some(
                            a.value / b.value, // TODO - can this panic?
                        )
                    },
                    ExpressionOperator::Remainder => |a, b| {
                        Some(
                            a.value % b.value, // TODO - can this panic?
                        )
                    },
                    _ => |_a, _b| None,
                };

            let operator2 = |a: Option<&TimeWithReference>,
                             b: Option<&TimeWithReference>|
             -> Option<TimeWithReference> {
                if let Some(a2) = a {
                    if let Some(b2) = b {
                        operator(a2, b2).map(|value| TimeWithReference {
                            value,
                            references: ReferencesBuilder::new()
                                .add(&a2.references)
                                .add(&b2.references)
                                .done(),
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            let result = operate_on_arrays(ctx, &l.result, &r.result, &operator2);

            Ok(LookupExpressionResult {
                result,
                all_references,
            })
        }
    }
}

#[derive(Debug)]
struct SideEvent<'a> {
    time: Time,
    is_left: bool,
    is_start: bool,
    // references: &'a Vec<String>,
    instance: &'a TimelineObjectInstance,
}
impl<'a> IsEvent for SideEvent<'a> {
    fn time(&self) -> u64 {
        self.time
    }

    fn is_start(&self) -> bool {
        self.is_start
    }

    fn id(&self) -> &String {
        &self.instance.id
    }
}

fn get_side_events(res: &LookupExpressionResult, is_left: bool) -> Vec<SideEvent> {
    let mut events = Vec::new();

    match &res.result {
        LookupExpressionResultType::Instances(instances) => {
            for instance in instances {
                if let Some(end) = instance.end {
                    if end == instance.start {
                        // event doesn't actually exist...
                        continue;
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
        _ => {}
    };

    events
}
