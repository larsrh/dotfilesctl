use config::*;
use failure::Error;
use std::collections::HashMap;
use std::fs::{OpenOptions};
use std::io::{Read, Write};
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
    pub status: SymlinkStatus
}

impl Symlink {
    pub fn new(expected: PathBuf, status: SymlinkStatus) -> Symlink {
        Symlink { expected, status }
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
                            if expected == actual { SymlinkStatus::Ok } else { SymlinkStatus::Wrong }
                        ),
                    Err(_) =>
                        Symlink::new(expected, SymlinkStatus::Wrong)
                },
            Err(err) =>
                Symlink::new(expected, SymlinkStatus::Absent(Error::from(err)))
        }
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

    // TODO implement thorough checking
    pub fn check(&self, config: &Config, _thorough: bool, repair: bool) -> Result<(), Error> {
        info!("Checking for absent content in {:?}", config.contents());
        let absent_contents = self.get_absent_files(config.contents().as_path());
        if absent_contents.is_empty() {
            info!("No absent content.")
        }
            else {
                if repair {
                    warn!("Cannot fix absent content.");
                }
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
                    let msg = format!("{:?} is not a symbolic link or symbolic link with wrong target, expected: {:?}", dotfile, symlink.expected);
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
        info!("{} symlinks correct.", symlinks.len());

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

    #[test]
    fn test_check_success() {
        let (_dir, config) = setup_config();
        let files = vec![".test1", ".test2"];
        for f in &files {
            let path = config.contents().join(f);
            let msg = format!("{:?} can be created", path);
            File::create(path).expect(msg.as_ref());
            unix::symlink(config.contents().join(f), config.get_home().unwrap().join(f)).unwrap();
        }
        let dotfiles = Dotfiles::new(Some(files.iter().map(PathBuf::from).collect()));
        dotfiles.save(&config).unwrap();
        dotfiles.check(&config,false, false).unwrap();
    }
}
