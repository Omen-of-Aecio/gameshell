//! Feedback type for informing gameshell about what action to take
/// Feedback type for informing gameshell about what action to take
#[derive(Clone, Debug, PartialEq)]
pub enum Feedback {
    /// Inform gameshell that this action errored, causing gameshell to abort a nested call and
    /// report the error
    Err(String),
    /// Inform gameshell that a help message should be returned to the user
    Help(String),
    /// Inform gameshell that a computation succeeded
    Ok(String),
}

impl Default for Feedback {
    fn default() -> Self {
        Feedback::Ok(String::default())
    }
}
