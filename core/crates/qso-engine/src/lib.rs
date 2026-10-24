//! Automazione dei QSO: regole, salvaguardie e registro delle decisioni.

pub mod brain;
pub mod runner;

pub use brain::{Brain, BrainState, Decision, DecisionKind, Limits, LiveNotAllowed, Mode, Rules};
pub use runner::{Automation, AutomationConfig, ClientGuard};
