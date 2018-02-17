use std::fmt::{Display, Formatter, Result};

pub static APP_VERSION: &'static str = crate_version!();
pub static APP_NAME: &'static str = crate_name!();

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
