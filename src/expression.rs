use regex::Regex;
use std::fmt::{Debug, Display, Error, Formatter};

lazy_static::lazy_static! {
    static ref OPERATORS: &'static [&'static str] = &["&", "|", "+", "-", "*", "/", "%", "!"];
    static ref OPERATOR_REGEX: Regex = {
        let operators = OPERATORS.iter().map(|o| regex::escape(o)).collect::<Vec<String>>().join("");

        Regex::new(&format!(r"([{}\\(\\)])", operators)).unwrap()
    };
}

#[derive(PartialEq, Debug, Clone)]
pub enum ExpressionOperator {
    And,
    Or,
    Add,
    Subtract,
    Multiply,
    Divide,
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

pub struct ParsedExpression {
    pub l: Box<ParsedExpression>,
    pub o: ExpressionOperator,
    pub r: Box<ParsedExpression>,
}

#[derive(PartialEq, Debug, Clone)]
pub struct ExpressionObj {
    pub l: Expression,
    // pub o: String,
    pub o: ExpressionOperator,
    pub r: Expression,
}
impl ExpressionObj {
    pub fn wrap(self) -> Expression {
        Expression::Expression(Box::new(self))
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
                Expression::String(str) => Ok(new_expr),
                _ => simplify_expression(&new_expr),
            }
        }
        Expression::Null => Ok(Expression::Null),
        Expression::Invert(innerExpr) => {
            simplify_expression(innerExpr).and_then(|e| Ok(Expression::Invert(Box::new(e))))
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
        Expression::Expression(_) => Ok(expression.clone()), // TODO - recurse?
        Expression::Invert(_) => Ok(expression.clone()),     // TODO - recurse?
    }
}

pub fn interpret_expression_string(expression_str: &str) -> Result<Expression, ExpressionError> {
    let expression_str2 = OPERATOR_REGEX.replace_all(expression_str, " $1 ");

    let words: Vec<&str> = expression_str2.split_whitespace().collect();
    if words.len() == 0 {
        return Ok(Expression::Null);
    }

    let wrapped = wrap_expression(words)?;
    interpret_words(wrapped)
}

fn interpret_word(
    words_before: &mut Vec<WrappedWords>,
    word: WrappedWords,
) -> Result<Expression, ExpressionError> {
    // Look at the word before to check if this should be inverted
    let invert = {
        if let Some(word_before) = words_before.last() {
            match word_before {
                WrappedWords::Single(word) => {
                    if *word == "!" {
                        words_before.pop();
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            }
        } else {
            false
        }
    };

    let inner = match word {
        WrappedWords::Single(word) => {
            if let Ok(num) = word.parse::<i64>() {
                Ok(Expression::Number(ensure_number_polarity(
                    words_before,
                    num,
                )))
            } else {
                Ok(Expression::String(word.to_string()))
            }
        }
        WrappedWords::Group(grp) => interpret_words(grp),
    };

    if invert {
        inner.map(|e| Expression::Invert(Box::new(e)))
    } else {
        inner
    }
}

fn is_operator(word: &WrappedWords) -> bool {
    if let WrappedWords::Single(word) = word {
        word.len() == 1 && OPERATORS.contains(word)
    } else {
        false
    }
}

fn process_number_polarity(val: i64, polarity: &WrappedWords) -> Option<i64> {
    if let WrappedWords::Single(op) = polarity {
        if *op == "+" {
            Some(val)
        } else if *op == "-" {
            Some(-val)
        } else {
            None
        }
    } else {
        None
    }
}

fn ensure_number_polarity(phrase: &mut Vec<WrappedWords>, val: i64) -> i64 {
    if let Some(polarity) = phrase.last() {
        if is_operator(polarity) {
            if phrase.len() == 1 {
                // TODO this must be polarity
                if let Some(res) = process_number_polarity(val, polarity) {
                    phrase.pop();
                    res
                } else {
                    val
                }
            } else {
                let prev_word = &phrase[phrase.len() - 2];
                if is_operator(prev_word) {
                    if let Some(res) = process_number_polarity(val, polarity) {
                        phrase.pop();
                        res
                    } else {
                        val
                    }
                } else {
                    val
                }
            }
        } else {
            val
        }
    } else {
        val
    }
}

fn match_operator(str: &str) -> Option<ExpressionOperator> {
    if str == "&" {
        Some(ExpressionOperator::Add)
    } else {
        None
    }
}

fn interpret_words(mut phrase: Vec<WrappedWords>) -> Result<Expression, ExpressionError> {
    if let Some(last_word) = phrase.pop() {
        let mut current_expression = interpret_word(&mut phrase, last_word)?;

        while phrase.len() > 0 {
            if let Some(operator) = phrase.pop() {
                if let WrappedWords::Single(op) = operator {
                    if let Some(op2) = match_operator(op) {
                        // Catch any remaining negations
                        if op == "!" {
                            current_expression = Expression::Invert(Box::new(current_expression));
                            continue;
                        }

                        let left = phrase
                            .pop()
                            .ok_or(ExpressionError::Invalid)
                            .and_then(|x| interpret_word(&mut phrase, x))?;
                        current_expression = ExpressionObj {
                            l: left,
                            o: op2,
                            r: current_expression,
                        }
                        .wrap();
                    } else {
                        return Err(ExpressionError::InvalidOperator);
                    }
                } else {
                    return Err(ExpressionError::MissingOperator);
                }
            }
        }

        Ok(current_expression)
    } else {
        Ok(Expression::Null)
    }
}

#[derive(PartialEq, Debug, Clone)]
enum WrappedWords<'a> {
    Single(&'a str),
    Group(Vec<WrappedWords<'a>>),
}

#[derive(Debug, PartialEq)]
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

    if stack.len() > 0 {
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

        //        assert_eq!(
        //            interpret_expression_string("1 * 2 + 3").expect("Expected success"),
        //            ExpressionObj {
        //                l: ExpressionObj {
        //                    l: Expression::Number(1),
        //                    o: "*".to_string(),
        //                    r: Expression::Number(2)
        //                }
        //                .wrap(),
        //                o: "+".to_string(),
        //                r: Expression::Number(3)
        //            }
        //            .wrap()
        //        );

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
    }

    #[test]
    fn wrap_inner_expressions1() {
        let input = vec!["a", "(", "b", "c", ")"];
        let expected = vec![
            WrappedWords::Single("a"),
            WrappedWords::Group(vec![WrappedWords::Single("b"), WrappedWords::Single("c")]),
        ];
        assert_eq!(wrap_expression(input).unwrap(), expected);
    }

    #[test]
    fn wrap_inner_expressions2() {
        let input = vec!["a", "&", "!", "b"];
        let expected = vec![
            WrappedWords::Single("a"),
            WrappedWords::Single("&"),
            WrappedWords::Group(vec![
                WrappedWords::Single(""),
                WrappedWords::Single("!"),
                WrappedWords::Single("b"),
            ]),
        ];
        assert_eq!(wrap_expression(input).unwrap(), expected);
    }
}
