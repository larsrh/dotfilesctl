use crate::config::*;
use crate::paths;
use crate::util::*;
use anyhow::Error;
use fs_extra;
use fs_extra::dir::CopyOptions;
use std::collections::HashMap;
use std::fs;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::os::unix::fs as unix;
use std::path::{Path, PathBuf};
use std::vec::Vec;
use toml;
use toml::Value;

pub enum SymlinkStatus {
    Ok,
    Absent(Error),
    Wrong,
}

pub enum RepairAction {
    Skip,
    Delete,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum RepairResult {
    Successful,
    Skipped,
}

impl RepairResult {
    pub fn coalesce(self: RepairResult, that: &RepairResult) -> RepairResult {
        match self {
            RepairResult::Skipped => RepairResult::Skipped,
            _ => that.clone(),
        }
    }

    pub fn coalesce_all(results: Vec<RepairResult>) -> RepairResult {
        results
            .iter()
            .fold(RepairResult::Successful, RepairResult::coalesce)
    }
}

pub struct Symlink {
    pub expected: PathBuf,
    pub path: PathBuf,
    pub status: SymlinkStatus,
}

impl Symlink {
    fn new(expected: PathBuf, path: PathBuf, status: SymlinkStatus) -> Symlink {
        Symlink {
            expected,
            path,
            status,
        }
    }

    pub fn get(contents: &Path, home: &Path, dotfile: &PathBuf) -> Symlink {
        let expected = contents.join(dotfile);
        let symlink = home.join(dotfile);
        match symlink.symlink_metadata() {
            Ok(_) => match symlink.read_link() {
                Ok(actual) => Symlink::new(
                    expected.clone(),
                    symlink,
                    if expected == actual {
                        SymlinkStatus::Ok
                    } else {
                        SymlinkStatus::Wrong
                    },
                ),
                Err(_) => Symlink::new(expected, symlink, SymlinkStatus::Wrong),
            },
            Err(err) => Symlink::new(expected, symlink, SymlinkStatus::Absent(Error::from(err))),
        }
    }

    pub fn create(&self) -> Result<()> {
        info!("Creating symlink {:?}", self.expected);
        unix::symlink(self.expected.clone(), self.path.clone())?;
        Ok(())
    }

    pub fn repair(
        &self,
        wrong_behaviour: fn(&PathBuf) -> Result<RepairAction>,
    ) -> Result<RepairResult> {
        let result = match self.status {
            SymlinkStatus::Wrong => {
                let action = wrong_behaviour(&self.path)?;
                match action {
                    RepairAction::Skip => {
                        warn!("Skipping file {:?}", self.path);
                        RepairResult::Skipped
                    }
                    RepairAction::Delete => {
                        info!("Deleting file {:?}", self.path);
                        fs::remove_file(self.path.clone())?;
                        self.create()?;
                        RepairResult::Successful
                    }
                }
            }
            SymlinkStatus::Absent(_) => {
                self.create()?;
                RepairResult::Successful
            }
            SymlinkStatus::Ok => RepairResult::Successful,
        };

        Ok(result)
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Dotfiles {
    version: Option<i64>, // always 1
    files: Option<Vec<PathBuf>>,
    deleted: Option<Vec<PathBuf>>
}

impl Dotfiles {
    pub fn new(files: Option<Vec<PathBuf>>, deleted: Option<Vec<PathBuf>>) -> Dotfiles {
        Dotfiles {
            files,
            deleted,
            version: Some(1)
        }
    }

    pub fn get_files(&self) -> Vec<PathBuf> {
        match self.files {
            Some(ref files) => files.clone(),
            None => vec![]
        }
    }

    pub fn get_deleted(&self) -> Vec<PathBuf> {
        match self.deleted {
            Some(ref deleted) => deleted.clone(),
            None => vec![]
        }
    }

    pub fn canonicalize(&self) -> Dotfiles {
        Dotfiles::new(Some(self.get_files()), Some(self.get_deleted()))
    }

    pub fn get_absent_files(&self, contents: &Path) -> Vec<PathBuf> {
        unexpected_files(contents, &self.get_files(), true)
    }

    pub fn get_spurious_files(&self, contents: &Path) -> Vec<PathBuf> {
        unexpected_files(contents, &self.get_deleted(), false)
    }

    pub fn get_symlinks(&self, contents: &Path, home: &Path) -> HashMap<PathBuf, Symlink> {
        self.get_files()
            .iter()
            .map(|dotfile| (dotfile.clone(), Symlink::get(contents, home, dotfile)))
            .collect()
    }

    pub fn load(config: &Config) -> Result<Dotfiles> {
        let mut contents = String::new();
        OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .open(config.dotfiles())?
            .read_to_string(&mut contents)?;
        let toml = toml::from_str::<Value>(contents.as_ref())?;
        if let Some(table) = toml.as_table() {
            let version = if let Some(value) = table.get("version") {
                value.clone().try_into::<i64>()?
            } else {
                // use initial version that had no version tag
                1
            };

            match version {
                1 => {
                    let dotfiles = toml.clone().try_into::<Dotfiles>()?;
                    Ok(dotfiles)
                }
                _ => {
                    let msg = format!("Invalid version number {:?}", version);
                    let err = DotfilesError::new(msg);
                    Err(err)?
                }
            }
        } else {
            let msg = format!("Expected table, got {:?}", toml.type_str());
            let err = DotfilesError::new(msg);
            Err(err)?
        }
    }

    pub fn save(&self, config: &Config) -> Result<()> {
        let contents = toml::to_string(&self.canonicalize())?;
        OpenOptions::new()
            .truncate(true)
            .write(true)
            .create(true)
            .open(config.dotfiles())?
            .write_all(contents.as_bytes())?;
        Ok(())
    }

    pub fn check(&self, config: &Config) -> Result<()> {
        info!("Checking consistency");
        let files = self.get_files();
        let deleted = self.get_deleted();
        if !is_unique(&files) || !is_unique(&deleted) {
            let msg = format!("Duplicate files");
            let err = DotfilesError::new(msg);
            Err(err)?
        }

        if let Some(f) = files.iter().find(|f| deleted.contains(f)) {
            let msg = format!("File {:?} is both listed and deleted", f);
            let err = DotfilesError::new(msg);
            Err(err)?
        }

        info!("Consistent.");

        info!("Checking for absent content in {:?}", config.contents());
        let absent_contents = self.get_absent_files(config.contents().as_path());
        if absent_contents.is_empty() {
            info!("No absent content.")
        } else {
            let msg = format!("Absent content: {:?}", absent_contents);
            let err = DotfilesError::new(msg);
            Err(err)?
        }

        info!("Checking for spurious content in {:?}", config.contents());
        let spurious_contents = self.get_spurious_files(config.contents().as_path());
        if spurious_contents.is_empty() {
            info!("No spurious content.")
        } else {
            let msg = format!("Spurious content: {:?}", spurious_contents);
            let err = DotfilesError::new(msg);
            Err(err)?
        }

        let home = config.get_home()?;
        info!("Checking for symlinks in {:?}", home);
        let symlinks = self.get_symlinks(config.contents().as_path(), home.as_path());
        for (dotfile, symlink) in &symlinks {
            match symlink.status {
                SymlinkStatus::Wrong => {
                    let msg = format!(
                        "{:?} is not a symlink or symlink with wrong target, expected: {:?}",
                        dotfile, symlink.expected
                    );
                    let err = DotfilesError::new(msg);
                    Err(err)?
                }
                SymlinkStatus::Absent(ref err) => {
                    let msg = format!(
                        "{:?} does not exist, expected symbolic link to {:?} ({:?})",
                        dotfile, symlink.expected, err
                    );
                    Err(DotfilesError::new(msg))?
                }
                SymlinkStatus::Ok => (),
            }
        }
        info!("{} symlink(s) correct.", symlinks.len());

        Ok(())
    }

    pub fn track(
        &self,
        config: &Config,
        file: &PathBuf,
        validate_relative: fn(&PathBuf) -> Result<()>,
    ) -> Result<Dotfiles> {
        let file_type = file.symlink_metadata()?.file_type();
        if file_type.is_symlink() {
            let msg = format!("Cannot track {:?} because it is a symlink", file);
            Err(DotfilesError::new(msg))?
        }

        let file = file.canonicalize()?;
        let home = config.get_home()?;
        if !file.starts_with(home.clone()) {
            let msg = format!(
                "Cannot track {:?} because it is not in the home directory {:?}",
                file, home
            );
            Err(DotfilesError::new(msg))?
        }

        let mut files = self.get_files();
        let deleted = self.get_deleted();
        let relative = paths::relative_to(&home, &file);
        validate_relative(&relative)?;
        if files.contains(&relative) {
            let msg = format!("Cannot track {:?} because it is already tracked", file);
            Err(DotfilesError::new(msg))?
        }
        if deleted.contains(&relative) {
            let msg = format!("Cannot track {:?} because it has been deleted", file);
            Err(DotfilesError::new(msg))?
        }

        if file_type.is_file() {
            info!("Tracking {:?}", relative);
        } else {
            info!("Tracking {:?} and all its children", relative);
        }

        let mut dest = config.contents();
        dest.push(relative.clone());
        let content_path = dest.clone();
        dest.pop();
        fs::create_dir_all(dest.clone())?;
        fs_extra::move_items(&vec![file.clone()], dest, &CopyOptions::new())?;

        unix::symlink(content_path, file)?;

        files.push(relative);
        Ok(Dotfiles::new(Some(files), Some(deleted)))
    }

    pub fn untrack(
        &self,
        config: &Config,
        file: &PathBuf,
        confirm_delete: fn(&PathBuf) -> Result<()>,
    ) -> Result<Dotfiles> {
        let home = config.get_home()?;
        if !file.starts_with(home.clone()) {
            let msg = format!(
                "Cannot untrack {:?} because it is not in the home directory {:?}",
                file, home
            );
            Err(DotfilesError::new(msg))?
        }

        let mut files = self.get_files();
        let mut deleted = self.get_deleted();
        let relative = paths::relative_to(&home, &file);
        if !files.contains(&relative) {
            let msg = format!("Cannot untrack {:?} because it is not tracked", relative);
            Err(DotfilesError::new(msg))?
        }
        if deleted.contains(&relative) {
            let msg = format!(
                "Cannot untrack {:?} because it has already been deleted",
                relative
            );
            Err(DotfilesError::new(msg))?
        }

        let mut dest = config.contents();
        dest.push(relative.clone());

        confirm_delete(&dest)?;
        fs::remove_file(dest)?;

        confirm_delete(&file)?;
        fs::remove_file(file)?;

        files.remove(files.iter().position(|file| *file == relative).unwrap());
        deleted.push(relative);
        Ok(Dotfiles::new(Some(files), Some(deleted)))
    }

    pub fn repair(
        &self,
        config: &Config,
        wrong_behaviour: fn(&PathBuf) -> Result<RepairAction>,
    ) -> Result<RepairResult> {
        let home = config.get_home()?;
        info!("Attempting to repair broken symlinks in {:?}", home);

        let symlinks = self.get_symlinks(config.contents().as_path(), home.as_path());

        let skippeds: Result<_> = symlinks
            .values()
            .map(|symlink| symlink.repair(wrong_behaviour))
            .collect();

        Ok(RepairResult::coalesce_all(skippeds?))
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::os::unix::fs as unix;
    use crate::config::Config;
    use crate::config::test_util::*;
    use crate::dotfiles::*;

    #[test]
    fn test_empty_dotfiles() {
        let (_dir, config) = setup_config();
        let dotfiles = Dotfiles::load(&config).unwrap();
        dotfiles.save(&config).unwrap();
        let dotfiles = Dotfiles::load(&config).unwrap();
        assert_eq!(dotfiles, Dotfiles::new(Some(vec![]), Some(vec![])));
    }

    fn setup_content(config: &Config, file: &str) {
        let path = config.contents().join(file);
        let msg = format!("{:?} can be created", path);
        File::create(path).expect(msg.as_ref());
    }

    fn setup_dotfile(config: &Config, file: &str) -> PathBuf {
        let path = config.get_home().unwrap().join(file);
        let content = file.as_bytes();
        let mut file = File::create(path.clone()).unwrap();
        file.write_all(content).unwrap();
        path
    }

    fn setup_symlink(config: &Config, file: &str) {
        let src = config.contents().join(file);
        let dst = config.get_home().expect("home").join(file);
        let msg = format!("{:?} can be created", dst);
        unix::symlink(src, dst).expect(msg.as_ref());
    }

    fn setup_symlink_wrong(config: &Config, file: &str) {
        let path = config.get_home().expect("home").join(file);
        let msg = format!("{:?} can be created", path);
        File::create(path).expect(msg.as_ref());
    }

    // TODO refactor
    #[test]
    fn test_check_success() {
        let (_dir, config) = setup_config();
        let files = vec![".test1", ".test2"];
        for f in &files {
            setup_content(&config, f);
            setup_symlink(&config, f);
        }
        let dotfiles = Dotfiles::new(Some(files.iter().map(PathBuf::from).collect()), Some(vec![]));
        dotfiles.check(&config).unwrap();
    }

    #[test]
    #[should_panic(expected = "Absent content")]
    fn test_check_failure_missing() {
        let (_dir, config) = setup_config();
        let dotfiles = Dotfiles::new(Some(vec![PathBuf::from(".test")]), Some(vec![]));
        dotfiles.check(&config).unwrap();
    }

    #[test]
    #[should_panic(expected = "expected symbolic link to")]
    fn test_check_failure_symlink() {
        let (_dir, config) = setup_config();
        let file = ".test";
        setup_content(&config, file);
        let dotfiles = Dotfiles::new(Some(vec![PathBuf::from(file)]), Some(vec![]));
        dotfiles.check(&config).unwrap();
    }

    #[test]
    fn test_repair_absent() {
        let (_dir, config) = setup_config();
        let file = ".test";
        setup_content(&config, file);
        let dotfiles = Dotfiles::new(Some(vec![PathBuf::from(file)]), Some(vec![]));
        assert_eq!(
            RepairResult::Successful,
            dotfiles
                .repair(&config, |_| Ok(RepairAction::Skip))
                .unwrap()
        );
        dotfiles.check(&config).unwrap();
    }

    #[test]
    fn test_repair_wrong() {
        let (_dir, config) = setup_config();
        let file = ".test";
        setup_content(&config, file);
        setup_symlink_wrong(&config, file);
        let dotfiles = Dotfiles::new(Some(vec![PathBuf::from(file)]), Some(vec![]));
        assert_eq!(
            RepairResult::Successful,
            dotfiles
                .repair(&config, |_| Ok(RepairAction::Delete))
                .unwrap()
        );
        dotfiles.check(&config).unwrap();
    }

    #[test]
    #[should_panic(expected = "is not a symlink or symlink with wrong target")]
    fn test_repair_wrong_skip() {
        let (_dir, config) = setup_config();
        let file = ".test";
        setup_content(&config, file);
        setup_symlink_wrong(&config, file);
        let dotfiles = Dotfiles::new(Some(vec![PathBuf::from(file)]), Some(vec![]));
        assert_eq!(
            RepairResult::Skipped,
            dotfiles
                .repair(&config, |_| Ok(RepairAction::Skip))
                .unwrap()
        );
        dotfiles.check(&config).unwrap();
    }

    #[test]
    fn test_track_file() {
        let (_dir, config) = setup_config();
        let file = ".test";
        let path = setup_dotfile(&config, file);
        let dotfiles = Dotfiles::load(&config).unwrap();
        let dotfiles = dotfiles.track(&config, &path, |_| Ok(())).unwrap();
        dotfiles.check(&config).unwrap();

        let mut contents = String::new();
        File::open(config.contents().join(file))
            .unwrap()
            .read_to_string(&mut contents)
            .unwrap();
        assert_eq!(contents, file);
    }
}
