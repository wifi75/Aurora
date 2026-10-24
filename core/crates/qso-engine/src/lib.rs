//! Automazione dei QSO: regole, salvaguardie e registro delle decisioni.

pub mod brain;
pub mod ghost;
pub mod runner;

pub use brain::{Brain, BrainState, Decision, DecisionKind, Limits, LiveNotAllowed, Mode, Prefer, Rules, WantedMatcher};
pub use runner::{Automation, AutomationConfig, ClientGuard};
