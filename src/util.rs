use anyhow::Result;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};

pub static APP_VERSION: &str = crate_version!();
pub static APP_NAME: &str = crate_name!();

#[derive(Debug, Error)]
pub struct DotfilesError {
    pub description: String,
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

pub fn result_from_option<T>(opt: Option<T>, msg: String) -> Result<T> {
    let t = opt.ok_or_else(|| DotfilesError::new(msg))?;
    Ok(t)
}

pub fn is_unique<T: Ord + Clone>(vec: &Vec<T>) -> bool {
    let mut vec2 = vec.clone();
    vec2.sort();
    vec2.dedup();
    vec2.len() == vec.len()
}

pub fn unexpected_files(dir: &Path, files: &[PathBuf], expect_exists: bool) -> Vec<PathBuf> {
    files
        .iter()
        .filter_map(|file| {
            let expected = dir.join(file);
            if expected.exists() == expect_exists {
                None
            } else {
                Some(file.clone())
            }
        })
        .collect()
}

#[cfg(test)]
mod test {

    use crate::util::is_unique;

    #[test]
    fn test_unique() {
        assert!(is_unique(&vec![1, 2, 3]));
        assert!(!is_unique(&vec![1, 2, 2]));
        assert!(!is_unique(&vec![2, 1, 2]));
    }
}
