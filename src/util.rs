use std::fmt::{Display, Formatter, Result};

#[derive(Debug, Fail)]
pub struct DotfilesError {
    pub description: String
}

impl Display for DotfilesError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        // Use `self.number` to refer to each positional data point.
        write!(f, "{}", self.description)
    }
}

impl DotfilesError {
    pub fn new(description: String) -> DotfilesError {
        DotfilesError { description }
    }
}
