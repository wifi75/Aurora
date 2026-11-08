//! Automazione dei QSO: regole, salvaguardie e registro delle decisioni.

pub mod brain;
pub mod ghost;
pub mod runner;

pub use brain::{Brain, BrainState, Decision, DecisionKind, Limits, LiveNotAllowed, Mode, NewModeMatcher, Prefer, Rules, WantedMatcher, CONTINENTS};
pub use runner::{Automation, AutomationConfig, ClientGuard, LocalOffset, TxSwitch};
