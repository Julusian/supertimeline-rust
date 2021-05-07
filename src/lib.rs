mod macros;

mod api;
mod caps;
mod events;
mod expression;
mod instance;
mod lookup_expression;
mod references;
mod resolver;
mod state;
mod util;

//use crate::types::{Expression, ExpressionObj};

pub use state::{get_state, resolve_states};
pub use api::resolve_timeline;

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
