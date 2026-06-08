//! Rule evaluation: trigger matching, condition checking, timerange evaluation,
//! and the background rules engine.

mod engine;
mod matching;
mod timerange;

pub use engine::spawn_rules_engine;
pub(crate) use matching::{condition_matches, trigger_matches_event};
pub use timerange::spawn_timerange_evaluator;
