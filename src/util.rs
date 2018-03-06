use failure::Error;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::result;

pub static APP_VERSION: &'static str = crate_version!();
pub static APP_NAME: &'static str = crate_name!();

#[derive(Debug, Fail)]
pub struct DotfilesError {
    pub description: String
}

impl Display for DotfilesError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // Use `self.number` to refer to each positional data point.
        write!(f, "{}", self.description)
    }
}

impl DotfilesError {
    pub fn new(description: String) -> DotfilesError {
        DotfilesError { description }
    }
}

pub type Result<T> = result::Result<T, Error>;

pub fn result_from_option<T>(opt: Option<T>, msg: String) -> Result<T> {
    let t = opt.ok_or_else(|| DotfilesError::new(msg))?;
    Ok(t)
}
