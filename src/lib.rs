mod expression;

//use crate::types::{Expression, ExpressionObj};

#[cfg(test)]
mod tests {
    //    use crate::expression::{ExpressionObj, Expression};

    #[test]
    fn it_works() {
        //        let tmp = ExpressionObj {
        //            l: Expression::String("0".to_string()),
        //            o: "+".to_string(),
        //            r: Expression::Number(4)
        //        };
        assert_eq!(2 + 2, 4);
    }
}
