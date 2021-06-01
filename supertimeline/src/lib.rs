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

pub use api::{resolve_timeline, IsTimelineKeyframe, IsTimelineObject, ResolveOptions};
pub use expression::{Expression, ExpressionError, ExpressionObj, ExpressionOperator};
pub use instance::{TimelineEnable, TimelineObjectInstance};
pub use state::{
    get_state, resolve_all_states, EventType, NextEvent, ResolvedStates, ResolvedStatesError,
    ResolvedTimelineObject, ResolvedTimelineObjectInstance, TimelineState,
};
pub use util::Time;
pub use caps::Cap;

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
