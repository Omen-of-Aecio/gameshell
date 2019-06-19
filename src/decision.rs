//! Decision used by the decider to inform gameshell
/// Decision used by the decider to inform gameshell
pub enum Decision {
    /// Decider failed but has a help message to display
    Help(String),
    /// Decider failed on this decision
    Err(String),
}
