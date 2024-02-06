use crate::perm::{Perm, Perms};
use anyhow::{Error, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub static APP_VERSION: &str = crate_version!();
pub static APP_NAME: &str = crate_name!();

pub fn remove_item<T: Eq>(vec: &mut Vec<T>, item: &T) {
    if let Some(pos) = vec.iter().position(|x| *x == *item) {
        vec.remove(pos);
    }
}

pub fn result_from_option<T>(opt: Option<T>, msg: String) -> Result<T> {
    opt.ok_or_else(|| Error::msg(msg))
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

fn file_meta(buf: &PathBuf) -> Result<fs::Metadata> {
    let meta = fs::metadata(buf.clone())?;
    if !meta.is_file() {
        Err(anyhow!("Not a simple file: {:?}", buf))?
    }
    Ok(meta)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Executable {
    Yes,
    No,
}

impl From<bool> for Executable {
    fn from(value: bool) -> Self {
        if value {
            Executable::Yes
        } else {
            Executable::No
        }
    }
}

impl Executable {
    pub fn update_perms(self, perms: Perms) -> Perms {
        fn add_x_if_r(perm: Perm) -> Perm {
            if perm.contains(Perm::R) {
                perm | Perm::X
            } else {
                perm
            }
        }

        match self {
            Executable::Yes => perms.map(add_x_if_r, add_x_if_r, add_x_if_r),
            Executable::No => perms.difference(Perms::UX | Perms::GX | Perms::OX),
        }
    }

    pub fn get(buf: &PathBuf) -> Result<Executable> {
        let meta = file_meta(buf)?;
        let user = Perms::try_from(meta.permissions())?.user();
        Ok(user.contains(Perm::X).into())
    }

    pub fn set(self, buf: &PathBuf) -> Result<()> {
        let meta = file_meta(buf)?;
        let updated = fs::Permissions::from(self.update_perms(meta.permissions().try_into()?));
        fs::set_permissions(buf, updated)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {

    use crate::perm::{Perms, MODE_MASK};
    use crate::util::{is_unique, Executable};
    use std::fs;
    use std::fs::Permissions;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::NamedTempFile;

    #[test]
    fn test_unique() {
        assert!(is_unique(&vec![1, 2, 3]));
        assert!(!is_unique(&vec![1, 2, 2]));
        assert!(!is_unique(&vec![2, 1, 2]));
    }

    #[test]
    fn test_add_executable() {
        assert_eq!(
            Perms::UR | Perms::UX,
            Executable::Yes.update_perms(Perms::UR)
        );
        assert_eq!(
            Perms::UR | Perms::UW | Perms::UX,
            Executable::Yes.update_perms(Perms::UR | Perms::UW)
        );
        assert_eq!(Perms::UW, Executable::Yes.update_perms(Perms::UW));
    }

    #[test]
    fn test_remove_executable() {
        assert_eq!(
            Perms::UR,
            Executable::No.update_perms(Perms::UR | Perms::UX)
        );
        assert_eq!(
            Perms::UR | Perms::GR | Perms::OR,
            Executable::No.update_perms(
                Perms::UR | Perms::UX | Perms::GR | Perms::GX | Perms::OR | Perms::OX
            )
        );
    }

    #[test]
    fn test_set_executable() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path();
        fs::set_permissions(path, Permissions::from_mode(0o0644)).unwrap();
        Executable::Yes.set(&path.to_path_buf()).unwrap();
        assert_eq!(
            0o755,
            MODE_MASK & fs::metadata(path).unwrap().permissions().mode()
        )
    }
}
