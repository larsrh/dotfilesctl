use config::*;
use failure::Error;
use std::collections::HashMap;
use std::fs;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::os::unix::fs as unix;
use std::path::{Path, PathBuf};
use std::vec::Vec;
use toml;
use util::DotfilesError;

pub enum SymlinkStatus {
    Ok,
    Absent(Error),
    Wrong
}

pub struct Symlink {
    pub expected: PathBuf,
    pub path: PathBuf,
    pub status: SymlinkStatus
}

impl Symlink {
    fn new(expected: PathBuf, path: PathBuf, status: SymlinkStatus) -> Symlink {
        Symlink { expected, path, status }
    }

    pub fn get(contents: &Path, home: &Path, dotfile: &PathBuf) -> Symlink {
        let expected = contents.join(dotfile);
        let symlink = home.join(dotfile);
        match symlink.symlink_metadata() {
            Ok(_) =>
                match symlink.read_link() {
                    Ok(actual) =>
                        Symlink::new(
                            expected.clone(),
                            symlink,
                            if expected == actual { SymlinkStatus::Ok } else { SymlinkStatus::Wrong }
                        ),
                    Err(_) =>
                        Symlink::new(expected, symlink, SymlinkStatus::Wrong)
                },
            Err(err) =>
                Symlink::new(expected, symlink, SymlinkStatus::Absent(Error::from(err)))
        }
    }

    pub fn create(&self) -> Result<(), Error> {
        info!("Creating symlink {:?}", self.expected);
        unix::symlink(self.expected.clone(), self.path.clone())?;
        Ok(())
    }

    pub fn repair(&self) -> Result<(), Error> {
        match self.status {
            SymlinkStatus::Wrong => {
                info!("Deleting file {:?}", self.path);
                fs::remove_file(self.path.clone())?;
                self.create()?
            },
            SymlinkStatus::Absent(_) => self.create()?,
            SymlinkStatus::Ok => ()
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Dotfiles {
    files: Option<Vec<PathBuf>>
}

impl Dotfiles {
    pub fn new(files: Option<Vec<PathBuf>>) -> Dotfiles {
        Dotfiles { files }
    }

    pub fn get_files(&self) -> Vec<PathBuf> {
        match self.files {
            Some(ref files) => files.clone(),
            None => vec![]
        }
    }

    pub fn canonicalize(&self) -> Dotfiles {
        Dotfiles::new(Some(self.get_files()))
    }

    pub fn get_absent_files(&self, contents: &Path) -> Vec<PathBuf> {
        self.get_files().iter().filter_map(|dotfile| {
            let expected = contents.join(dotfile);
            if expected.exists() {
                None
            }
                else {
                    Some(dotfile.clone())
                }
        }).collect()
    }

    pub fn get_symlinks(&self, contents: &Path, home: &Path) -> HashMap<PathBuf, Symlink> {
        self.get_files().iter().map(|dotfile| {
            (dotfile.clone(), Symlink::get(contents, home, dotfile))
        }).collect()
    }

    pub fn load(config: &Config) -> Result<Dotfiles, Error> {
        let mut contents = String::new();
        OpenOptions::new()
            .write(true).read(true).create(true)
            .open(config.dotfiles())?
            .read_to_string(&mut contents)?;
        let dotfiles = toml::from_str::<Dotfiles>(contents.as_ref())?;
        Ok(dotfiles)
    }

    pub fn save(&self, config: &Config) -> Result<(), Error> {
        let contents = toml::to_string(&self.canonicalize())?;
        OpenOptions::new()
            .truncate(true).write(true).create(true)
            .open(config.dotfiles())?.write(contents.as_bytes())?;
        Ok(())
    }

    pub fn check(&self, config: &Config) -> Result<(), Error> {
        info!("Checking for absent content in {:?}", config.contents());
        let absent_contents = self.get_absent_files(config.contents().as_path());
        if absent_contents.is_empty() {
            info!("No absent content.")
        }
        else {
            let msg = format!("Absent content: {:?}", absent_contents);
            let err = DotfilesError::new(msg);
            Err(err)?
        }

        let home = config.get_home()?;
        info!("Checking for symlinks in {:?}", home);
        let symlinks = self.get_symlinks(config.contents().as_path(), home.as_path());
        for (dotfile, symlink) in &symlinks {
            match symlink.status {
                SymlinkStatus::Wrong => {
                    let msg = format!("{:?} is not a symlink or symlink with wrong target, expected: {:?}", dotfile, symlink.expected);
                    let err = DotfilesError::new(msg);
                    Err(err)?
                },
                SymlinkStatus::Absent(ref err) => {
                    let msg = format!("{:?} does not exist, expected symbolic link to {:?} ({:?})", dotfile, symlink.expected, err);
                    Err(DotfilesError::new(msg))?
                },
                SymlinkStatus::Ok => ()
            }
        }
        info!("{} symlink(s) correct.", symlinks.len());

        Ok(())
    }

    pub fn repair(&self, config: &Config) -> Result<(), Error> {
        let home = config.get_home()?;
        info!("Attempting to repair broken symlinks in {:?}", home);

        let symlinks = self.get_symlinks(config.contents().as_path(), home.as_path());
        for (_, symlink) in &symlinks {
            symlink.repair()?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use config::test_util::*;
    use dotfiles::*;
    use std::fs::File;
    use std::os::unix::fs as unix;

    #[test]
    fn test_empty_dotfiles() {
        let (_dir, config) = setup_config();
        let dotfiles = Dotfiles::load(&config).unwrap();
        dotfiles.save(&config).unwrap();
        let dotfiles = Dotfiles::load(&config).unwrap();
        assert_eq!(dotfiles, Dotfiles::new(Some(vec![])));
    }

    fn setup_content(config: &Config, file: &str) {
        let path = config.contents().join(file);
        let msg = format!("{:?} can be created", path);
        File::create(path).expect(msg.as_ref());
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
        let dotfiles = Dotfiles::new(Some(files.iter().map(PathBuf::from).collect()));
        dotfiles.check(&config).unwrap();
    }

    #[test]
    #[should_panic(expected = "Absent content")]
    fn test_check_failure_missing() {
        let (_dir, config) = setup_config();
        let dotfiles = Dotfiles::new(Some(vec![PathBuf::from(".test")]));
        dotfiles.check(&config).unwrap();
    }

    #[test]
    #[should_panic(expected = "expected symbolic link to")]
    fn test_check_failure_symlink() {
        let (_dir, config) = setup_config();
        let file = ".test";
        setup_content(&config, file);
        let dotfiles = Dotfiles::new(Some(vec![PathBuf::from(file)]));
        dotfiles.check(&config).unwrap();
    }

    #[test]
    fn test_repair_absent() {
        let (_dir, config) = setup_config();
        let file = ".test";
        setup_content(&config, file);
        let dotfiles = Dotfiles::new(Some(vec![PathBuf::from(file)]));
        dotfiles.repair(&config).unwrap();
        dotfiles.check(&config).unwrap();
    }

    #[test]
    fn test_repair_wrong() {
        let (_dir, config) = setup_config();
        let file = ".test";
        setup_content(&config, file);
        setup_symlink_wrong(&config, file);
        let dotfiles = Dotfiles::new(Some(vec![PathBuf::from(file)]));
        dotfiles.repair(&config).unwrap();
        dotfiles.check(&config).unwrap();
    }
}
