//! Feedback type for informing gameshell about what action to take
/// Feedback type for informing gameshell about what action to take
#[must_use]
#[derive(Clone, Debug, PartialEq)]
pub enum Feedback {
    /// Inform gameshell that this action errored, causing gameshell to abort a nested call and
    /// report the error
    Err(String),
    /// Inform gameshell that a computation succeeded
    Ok(String),
}

impl Default for Feedback {
    fn default() -> Self {
        Feedback::Ok(String::default())
    }
}

impl Feedback {
    /// Moves the value `v` out of the `Feedback` if it is `Ok(v)`.
    pub fn unwrap(self) -> String {
        match self {
            Feedback::Err(err) => panic![
                "called `Feedback::unwrap()` on a `Err(String)` value, error={}",
                err
            ],
            Feedback::Ok(string) => string,
        }
    }
}
