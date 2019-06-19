#[derive(Clone, Debug, PartialEq)]
pub enum Feedback {
    Err(String),
    Help(String),
    Ok(String),
}

impl Default for Feedback {
    fn default() -> Self {
        Feedback::Ok(String::default())
    }
}
