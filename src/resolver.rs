use crate::expression::{Expression, ExpressionObj, ExpressionOperator, interpret_expression, ExpressionError};
use crate::instance::TimelineObjectInstance;
use crate::state;
use crate::util::{
    clean_instances, getId, invert_instances, join_caps, join_references, join_references2,
    join_references3, join_references4, Time
};
use regex::Regex;
use std::collections::HashSet;
use std::iter::FromIterator;
use std::fmt::Error;
use crate::lookup_expression::lookup_expression;

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

pub struct TimeWithReference {
    pub value: Time,
    pub references: HashSet<String>,
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

pub enum ResolveError {
    CircularDependency(String),
    BadExpression((String, &'static str, ExpressionError))
}

pub fn resolve_timeline_obj(
    resolved_timeline: &state::ResolvedTimeline,
    obj: &mut state::ResolvedTimelineObject,
) -> Result<(), ResolveError> {
    if obj.resolved.resolved {
        Ok(())
    } else if obj.resolved.resolving {
        Err(ResolveError.CircularDependency(obj.object.id().to_string()))
    } else {
        // TODO
        obj.resolved.resolving = true;


        let mut direct_references = HashSet::new();

        let obj_id = obj.object.id();
        for enable in obj.object.enable() {
            let repeating_expr = if let Some(expr) = &enable.repeating {
                match interpret_expression(expr) {
                    Ok(val) => val,
                    Err(err) => return Err(ResolveError::BadExpression((obj_id.to_string(), "repeating", err))),
                }
            } else {
                Expression::Null
            };

            let looked_repeating = lookup_expression(resolved_timeline, obj, &repeating_expr, &ObjectRefType::Duration);
            direct_references.extend(looked_repeating.all_references);

            // TODO


        }

        // // filter out zero-length instances:
        // instances = _.filter(instances, (instance) => {
        //     return ((instance.end || Infinity) > instance.start)
        // })

        obj.resolved.resolved = true;
        obj.resolved.resolving = false;
        obj.resolved.instances = instances;
        obj.resolved.directReferences = direct_references;

        Ok(())
    }
}
