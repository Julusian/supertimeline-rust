use regex::Regex;
use std::fmt::{Debug, Display, Error, Formatter};
#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

const OPERATORS: &[&str] = &["&", "|", "+", "-", "*", "/", "%", "!"];

lazy_static::lazy_static! {
    static ref OPERATOR_REGEX: Regex = {
        let operators = OPERATORS.iter().map(|o| regex::escape(o)).collect::<Vec<String>>().join("");

        Regex::new(&format!(r"([{}\\(\\)])", operators)).unwrap()
    };
}

#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum ExpressionOperator {
    #[cfg_attr(feature = "serde_support", serde(rename = "+"))]
    And,
    #[cfg_attr(feature = "serde_support", serde(rename = "|"))]
    Or,
    #[cfg_attr(feature = "serde_support", serde(rename = "&"))]
    Add,
    #[cfg_attr(feature = "serde_support", serde(rename = "-"))]
    Subtract,
    #[cfg_attr(feature = "serde_support", serde(rename = "*"))]
    Multiply,
    #[cfg_attr(feature = "serde_support", serde(rename = "/"))]
    Divide,
    #[cfg_attr(feature = "serde_support", serde(rename = "%"))]
    Remainder,
}
impl Display for ExpressionOperator {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            ExpressionOperator::And => write!(f, "&"),
            ExpressionOperator::Or => write!(f, "|"),
            ExpressionOperator::Add => write!(f, "+"),
            ExpressionOperator::Subtract => write!(f, "-"),
            ExpressionOperator::Multiply => write!(f, "*"),
            ExpressionOperator::Divide => write!(f, "/"),
            ExpressionOperator::Remainder => write!(f, "%"),
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde_support", serde(untagged))]
pub enum Expression {
    Null,
    Number(i64),
    String(String),
    Expression(Box<ExpressionObj>),
    Invert(Box<Expression>),
}
impl Display for Expression {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            Expression::Null => write!(f, "null"),
            Expression::Number(val) => write!(f, "{}", val),
            Expression::String(val) => write!(f, "\"{}\"", val),
            Expression::Expression(val) => write!(f, "{}", val),
            Expression::Invert(val) => write!(f, "!{}", val),
        }
    }
}

#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
#[derive(PartialEq, Debug, Clone)]
pub struct ExpressionObj {
    pub l: Expression,
    pub o: ExpressionOperator,
    pub r: Expression,
}
impl ExpressionObj {
    pub fn wrap(self) -> Expression {
        Expression::Expression(Box::new(self))
    }
    pub fn create(l: Expression, o: ExpressionOperator, r: Expression) -> Expression {
        Expression::Expression(Box::new(ExpressionObj{
            l,o,r
        }))
    }
}
impl Display for ExpressionObj {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "({} {} {})", self.l, self.o, self.r)
    }
}

pub fn is_constant(expression: &Expression) -> bool {
    match expression {
        Expression::Null => true,
        Expression::Number(_) => true,
        Expression::String(_) => false,
        Expression::Expression(_) => true,
        Expression::Invert(inner_expr) => is_constant(inner_expr),
    }
}

pub fn simplify_expression(expression: &Expression) -> Result<Expression, ExpressionError> {
    match expression {
        Expression::String(str) => {
            let new_expr = interpret_expression_string(str)?;
            match &new_expr {
                // recurse only if it is not a string, to avoid an infinite loop
                Expression::String(_str) => Ok(new_expr),
                _ => simplify_expression(&new_expr),
            }
        }
        Expression::Null => Ok(Expression::Null),
        Expression::Invert(inner_expr) => {
            simplify_expression(inner_expr).map(|e| Expression::Invert(Box::new(e)))
        }

        Expression::Number(val) => Ok(Expression::Number(*val)),
        Expression::Expression(obj) => {
            let l = simplify_expression(&obj.l)?;
            let r = simplify_expression(&obj.r)?;

            if let Expression::Number(l2) = l {
                if let Expression::Number(r2) = r {
                    match obj.o {
                        ExpressionOperator::Remainder => Ok(Expression::Number(l2 % r2)), // TODO - can this panic?
                        ExpressionOperator::Add => Ok(Expression::Number(l2 + r2)),
                        ExpressionOperator::Subtract => Ok(Expression::Number(l2 - r2)),
                        ExpressionOperator::Multiply => Ok(Expression::Number(l2 * r2)),
                        ExpressionOperator::Divide => Ok(Expression::Number(l2 / r2)), // TODO - can this panic?
                        // boolean operators arent supported
                        _ => Ok(ExpressionObj { l, o: obj.o, r }.wrap()),
                    }
                } else {
                    Ok(ExpressionObj { l, o: obj.o, r }.wrap())
                }
            } else {
                Ok(ExpressionObj { l, o: obj.o, r }.wrap())
            }
        }
    }
}

pub fn interpret_expression(expression: &Expression) -> Result<Expression, ExpressionError> {
    match expression {
        Expression::Null => Ok(Expression::Null),
        Expression::Number(val) => Ok(Expression::Number(*val)),
        Expression::String(val) => interpret_expression_string(&val),
        Expression::Expression(expr_obj) => {
            let l = interpret_expression(&expr_obj.l)?;
            let r = interpret_expression(&expr_obj.r)?;

            Ok(ExpressionObj {
                l,
                o: expr_obj.o,
                r,
            }
            .wrap())
        }
        Expression::Invert(inner_expr) => {
            interpret_expression(inner_expr).map(|res| Expression::Invert(Box::new(res)))
        }
    }
}

pub fn interpret_expression_string(expression_str: &str) -> Result<Expression, ExpressionError> {
    let expression_str2 = OPERATOR_REGEX.replace_all(expression_str, " $1 ");

    let words: Vec<&str> = expression_str2.split_whitespace().collect();
    if words.is_empty() {
        return Ok(Expression::Null);
    }

    let wrapped = wrap_expression(words)?;
    interpret_phrase(wrapped.as_slice(), None)
}

fn ensure_number_polarity(prev_op: Option<ExpressionOperator>, val: i64) -> Option<i64> {
    if let Some(op) = prev_op {
        match op {
            ExpressionOperator::Add => Some(val),
            ExpressionOperator::Subtract => Some(-val),
            _ => None,
        }
    } else {
        Some(val)
    }
}

fn match_operator(str: &str) -> Option<ExpressionOperator> {
    if str == "+" {
        Some(ExpressionOperator::Add)
    } else if str == "-" {
        Some(ExpressionOperator::Subtract)
    } else if str == "*" {
        Some(ExpressionOperator::Multiply)
    } else if str == "/" {
        Some(ExpressionOperator::Divide)
    } else if str == "%" {
        Some(ExpressionOperator::Remainder)
    } else if str == "&" {
        Some(ExpressionOperator::And)
    } else if str == "|" {
        Some(ExpressionOperator::Or)
    } else {
        None
    }
}

fn unwrap_word<'a>(word: &WrappedWords<'a>) -> Option<&'a str> {
    match word {
        WrappedWords::Group(_) => None,
        WrappedWords::Single(w) => Some(w),
    }
}

fn interpret_phrase(
    phrase: &[WrappedWords],
    prev_op: Option<ExpressionOperator>,
) -> Result<Expression, ExpressionError> {
    if phrase.is_empty() {
        Ok(Expression::Null)
    } else if phrase.len() == 1 {
        match phrase.last().unwrap() {
            WrappedWords::Single(word) => {
                if let Ok(num) = word.parse::<i64>() {
                    let parsed =
                        ensure_number_polarity(prev_op, num).ok_or(ExpressionError::Invalid)?;
                    Ok(Expression::Number(parsed))
                } else {
                    Ok(Expression::String(word.to_string()))
                }
            }
            WrappedWords::Group(grp) => {
                if prev_op.is_some() {
                    Err(ExpressionError::Invalid)
                } else {
                    interpret_phrase(grp, None)
                }
            }
        }
    } else {
        let operator_index = {
            let mut found = None;
            for op in OPERATORS {
                let index = phrase.iter().rposition(|w| {
                    if let WrappedWords::Single(w2) = w {
                        w2.eq(op)
                    } else {
                        false
                    }
                });
                if let Some(index) = index {
                    found = Some(index);
                    break;
                }
            }

            found
        };

        if let Some(op_index) = operator_index {
            if op_index != phrase.len() - 1 {
                // Nothing is following, therefore must be bad..
                let raw_op =
                    unwrap_word(&phrase[op_index]).ok_or(ExpressionError::MissingOperator)?;

                let prev_as_op = {
                    if op_index > 0 {
                        if let Some(raw_op2) = unwrap_word(&phrase[op_index - 1]) {
                            match_operator(raw_op2)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };

                if op_index == 0 && raw_op == "!" {
                    let r2 = interpret_phrase(&phrase[1..], None)?;
                    Ok(Expression::Invert(Box::new(r2)))
                } else {
                    let new_op = match_operator(raw_op).ok_or(ExpressionError::InvalidOperator)?;

                    if op_index == 0 {
                        if prev_op.is_some() {
                            Err(ExpressionError::Invalid)
                        } else {
                            interpret_phrase(&phrase[1..], Some(new_op))
                        }
                    } else {
                        let real_op = prev_as_op.unwrap_or(new_op);

                        let index = if prev_as_op.is_some() {
                            op_index - 1
                        } else {
                            op_index
                        };

                        let l2 = interpret_phrase(&phrase[..index], prev_op)?;
                        let r2 = interpret_phrase(
                            &phrase[(op_index + 1)..],
                            if prev_as_op.is_some() {
                                Some(new_op)
                            } else {
                                None
                            },
                        )?;

                        Ok(ExpressionObj {
                            l: l2,
                            o: real_op,
                            r: r2,
                        }
                        .wrap())
                    }
                }
            } else {
                Err(ExpressionError::Invalid)
            }
        } else {
            Err(ExpressionError::MissingOperator)
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
enum WrappedWords<'a> {
    Single(&'a str),
    Group(Vec<WrappedWords<'a>>),
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum ExpressionError {
    MismatchedParenthesis,
    Invalid,
    MissingOperator,
    InvalidOperator,
}

fn wrap_expression(words: Vec<&str>) -> Result<Vec<WrappedWords>, ExpressionError> {
    let mut remaining = words.clone();

    let mut stack = Vec::new();
    let mut current = Vec::new();

    loop {
        let next_pos = remaining.iter().position(|&x| x == "(" || x == ")");
        if let Some(index) = next_pos {
            let (before, after) = remaining.split_at(index);
            let token = after[0];

            for &t in before {
                current.push(WrappedWords::Single(t));
            }

            // Trim remaining set
            remaining = after.iter().skip(1).cloned().collect();

            if token == "(" {
                // We need a new level
                stack.push(current);
                current = Vec::new();
            } else {
                // Must be ")"
                if let Some(parent) = stack.pop() {
                    // Pop a level, and add this as a child
                    let child = current;
                    current = parent;
                    current.push(WrappedWords::Group(child));
                } else {
                    return Err(ExpressionError::MismatchedParenthesis);
                }
            }
        } else {
            break;
        }
    }

    for t in remaining {
        current.push(WrappedWords::Single(t));
    }

    if !stack.is_empty() {
        Err(ExpressionError::MismatchedParenthesis)
    } else {
        Ok(current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_number_strings() {
        assert_eq!(
            interpret_expression_string("42").expect("Expected success"),
            Expression::Number(42)
        );
        assert_eq!(
            interpret_expression_string("+42").expect("Expected success"),
            Expression::Number(42)
        );
        assert_eq!(
            interpret_expression_string("-42").expect("Expected success"),
            Expression::Number(-42)
        );

        assert_eq!(
            interpret_expression_string("42 -").expect_err("Expected an error"),
            ExpressionError::Invalid
        );
        //        assert_eq!(interpret_expression_string("42-").expect_err("Expected an error"), ExpressionError::Invalid);
    }

    #[test]
    fn simple_expression_strings() {
        assert_eq!(
            interpret_expression_string("1+2").expect("Expected success"),
            ExpressionObj {
                l: Expression::Number(1),
                o: ExpressionOperator::Add,
                r: Expression::Number(2)
            }
            .wrap()
        );

        assert_eq!(
            interpret_expression_string("   1   *   2   ").expect("Expected success"),
            ExpressionObj {
                l: Expression::Number(1),
                o: ExpressionOperator::Multiply,
                r: Expression::Number(2)
            }
            .wrap()
        );

        assert_eq!(
            interpret_expression_string("1 + 2").expect("Expected success"),
            ExpressionObj {
                l: Expression::Number(1),
                o: ExpressionOperator::Add,
                r: Expression::Number(2)
            }
            .wrap()
        );

        assert_eq!(
            interpret_expression_string("1 - 2").expect("Expected success"),
            ExpressionObj {
                l: Expression::Number(1),
                o: ExpressionOperator::Subtract,
                r: Expression::Number(2)
            }
            .wrap()
        );

        assert_eq!(
            interpret_expression_string("1 * 2").expect("Expected success"),
            ExpressionObj {
                l: Expression::Number(1),
                o: ExpressionOperator::Multiply,
                r: Expression::Number(2)
            }
            .wrap()
        );

        assert_eq!(
            interpret_expression_string("1 / 2").expect("Expected success"),
            ExpressionObj {
                l: Expression::Number(1),
                o: ExpressionOperator::Divide,
                r: Expression::Number(2)
            }
            .wrap()
        );

        assert_eq!(
            interpret_expression_string("1 % 2").expect("Expected success"),
            ExpressionObj {
                l: Expression::Number(1),
                o: ExpressionOperator::Remainder,
                r: Expression::Number(2)
            }
            .wrap()
        );

        assert_eq!(
            interpret_expression_string("1 + 2 * 3").expect("Expected success"),
            ExpressionObj {
                l: Expression::Number(1),
                o: ExpressionOperator::Add,
                r: ExpressionObj {
                    l: Expression::Number(2),
                    o: ExpressionOperator::Multiply,
                    r: Expression::Number(3)
                }
                .wrap()
            }
            .wrap()
        );

        assert_eq!(
            interpret_expression_string("1 * 2 + 3").expect("Expected success"),
            ExpressionObj {
                l: ExpressionObj {
                    l: Expression::Number(1),
                    o: ExpressionOperator::Multiply,
                    r: Expression::Number(2)
                }
                .wrap(),
                o: ExpressionOperator::Add,
                r: Expression::Number(3)
            }
            .wrap()
        );

        assert_eq!(
            interpret_expression_string("1 * (2 + 3)").expect("Expected success"),
            ExpressionObj {
                l: Expression::Number(1),
                o: ExpressionOperator::Multiply,
                r: ExpressionObj {
                    l: Expression::Number(2),
                    o: ExpressionOperator::Add,
                    r: Expression::Number(3)
                }
                .wrap()
            }
            .wrap()
        );

        assert_eq!(
            interpret_expression_string("#first & #second").expect("Expected success"),
            ExpressionObj {
                l: Expression::String("#first".to_string()),
                o: ExpressionOperator::And,
                r: Expression::String("#second".to_string())
            }
            .wrap()
        );

        assert_eq!(
            interpret_expression_string("!thisOne").expect("Expected success"),
            Expression::Invert(Box::new(Expression::String("thisOne".to_string())))
        );

        assert_eq!(
            interpret_expression_string("!thisOne & !(that | !those)").expect("Expected success"),
            ExpressionObj {
                l: Expression::Invert(Box::new(Expression::String("thisOne".to_string()))),
                o: ExpressionOperator::And,
                r: Expression::Invert(Box::new(
                    ExpressionObj {
                        l: Expression::String("that".to_string()),
                        o: ExpressionOperator::Or,
                        r: Expression::Invert(Box::new(Expression::String("those".to_string()))),
                    }
                    .wrap()
                ))
            }
            .wrap()
        );

        assert_eq!(
            interpret_expression_string("(!.classA | !$layer.classB) & #obj")
                .expect("Expected success"),
            ExpressionObj {
                r: Expression::String("#obj".to_string()),
                o: ExpressionOperator::And,
                l: ExpressionObj {
                    l: Expression::Invert(Box::new(Expression::String(".classA".to_string()))),
                    o: ExpressionOperator::Or,
                    r: Expression::Invert(Box::new(Expression::String(
                        "$layer.classB".to_string()
                    ))),
                }
                .wrap()
            }
            .wrap()
        );

        assert_eq!(
            interpret_expression_string("#obj.start").expect("Expected success"),
            Expression::String("#obj.start".to_string())
        );

        assert_eq!(
            interpret_expression_string("19").expect("Expected success"),
            Expression::Number(19)
        );
        assert_eq!(
            interpret_expression_string("").expect("Expected success"),
            Expression::Null
        );

        assert_eq!(
            interpret_expression_string("1+2+3").expect("Expected success"),
            ExpressionObj {
                l: ExpressionObj {
                    l: Expression::Number(1),
                    o: ExpressionOperator::Add,
                    r: Expression::Number(2),
                }
                .wrap(),
                o: ExpressionOperator::Add,
                r: Expression::Number(3),
            }
            .wrap()
        );
    }

    #[test]
    fn simplify_expressions() {
        let expr1 = interpret_expression_string("1+2+3").expect("Expected success");
        assert_eq!(
            simplify_expression(&expr1).expect("Expected simplify"),
            Expression::Number(6)
        );

        let expr2 = interpret_expression_string("1+2*2+(4-2)").expect("Expected success");
        assert_eq!(
            simplify_expression(&expr2).expect("Expected simplify"),
            Expression::Number(7)
        );

        let expr3 = interpret_expression_string("10 / 2 + 1").expect("Expected success");
        assert_eq!(
            simplify_expression(&expr3).expect("Expected simplify"),
            Expression::Number(6)
        );

        let expr4 = interpret_expression_string("40+2+asdf").expect("Expected success");
        assert_eq!(
            simplify_expression(&expr4).expect("Expected simplify"),
            ExpressionObj {
                l: Expression::Number(42),
                o: ExpressionOperator::Add,
                r: Expression::String("asdf".to_string())
            }
            .wrap()
        );
    }
}
