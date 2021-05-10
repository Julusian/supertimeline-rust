use crate::expression::ExpressionError;
use crate::util::Time;
use std::collections::HashSet;

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

#[derive(Debug, Clone)]
pub enum ResolveError {
    CircularDependency(String),
    BadExpression((String, &'static str, ExpressionError)),
    InstancesArrayNotSupported((String, &'static str)),
    ResolvedWhilePending(String),
    ResolvedWhileResolvec(String),
}
