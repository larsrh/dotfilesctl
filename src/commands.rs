use failure::Error;
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::env;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::time::Duration;
use std::vec::Vec;
use paths::*;
use toml;

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    target: PathBuf
}

impl Config {
    fn new(target: PathBuf) -> Config {
        Config { target }
    }

    fn dotfiles(&self) -> PathBuf {
        self.target.join("dotfiles.toml")
    }

    fn contents(&self) -> PathBuf {
        self.target.join("contents")
    }
}

enum SymlinkStatus {
    Ok,
    Absent(Error),
    Wrong
}

struct Symlink {
    expected: PathBuf,
    status: SymlinkStatus
}

impl Symlink {
    fn new(expected: PathBuf, status: SymlinkStatus) -> Symlink {
        Symlink { expected, status }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
struct Dotfiles {
    files: Option<Vec<PathBuf>>
}

impl Dotfiles {
    fn new(files: Option<Vec<PathBuf>>) -> Dotfiles {
        Dotfiles { files }
    }

    fn get_files(&self) -> Vec<PathBuf> {
        match self.files {
            Some(ref files) => files.clone(),
            None => vec![]
        }
    }

    fn canonicalize(&self) -> Dotfiles {
        Dotfiles::new(Some(self.get_files()))
    }

    fn get_absent_files(&self, contents: &Path) -> Vec<PathBuf> {
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

    fn get_symlinks(&self, contents: &Path, home: &Path) -> HashMap<PathBuf, Symlink> {
        self.get_files().iter().map(|dotfile| {
            let expected = contents.join(dotfile);
            let symlink = home.join(dotfile);
            let result = match symlink.symlink_metadata() {
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
            };
            (dotfile.clone(), result)
        }).collect()
    }
}

#[derive(Debug, Fail)]
struct DotfilesError {
    description: String
}

impl fmt::Display for DotfilesError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Use `self.number` to refer to each positional data point.
        write!(f, "{}", self.description)
    }
}

impl DotfilesError {
    fn new(description: String) -> DotfilesError {
        DotfilesError { description }
    }
}

fn check_config(config: &PathBuf) -> Result<Config, Error> {
    let mut contents = String::new();
    File::open(config)?.read_to_string(&mut contents)?;
    let config = toml::from_str::<Config>(contents.as_ref())?;
    Ok(config)
}

fn load_dotfiles(config: &Config) -> Result<Dotfiles, Error> {
    let mut contents = String::new();
    OpenOptions::new()
        .write(true).read(true).create(true)
        .open(config.dotfiles())?
        .read_to_string(&mut contents)?;
    let dotfiles = toml::from_str::<Dotfiles>(contents.as_ref())?;
    Ok(dotfiles)
}

fn save_dotfiles(config: &Config, dotfiles: Dotfiles) -> Result<(), Error> {
    let contents = toml::to_string(&dotfiles.canonicalize())?;
    OpenOptions::new()
        .truncate(true).write(true).create(true)
        .open(config.dotfiles())?.write(contents.as_bytes())?;
    Ok(())
}

pub fn init(config: &PathBuf, target: &PathBuf, force: bool) -> Result<(), Error> {
    if !target.is_dir() {
        let err = DotfilesError::new(format!("{:?} is not a directory", target));
        Err(err)?
    }
    else {
        let target = target.canonicalize()?;
        info!("Installing a fresh config in {:?}", config);
        if !config.is_file() || force {
            let contents = toml::to_string(&Config::new(target))?;
            File::create(config)?.write(contents.as_bytes())?;
            Ok(())
        }
        else {
            let msg = format!("{:?} exists but --force has not been specified", config);
            let err = DotfilesError::new(msg);
            Err(err)?
        }
    }
}

pub fn watch(config: PathBuf) -> Result<(), Error> {
    let config = check_config(&config)?;
    let (tx, rx) = channel();
    let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_secs(2))?;
    info!("Watching file changes in target {:?}", config.target);
    watcher.watch(config.target.clone(), RecursiveMode::Recursive)?;
    loop {
        let event = rx.recv()?;
        match event {
            DebouncedEvent::Create(created) => {
                let relative = relative_to(config.target.as_path(), created.as_path());
                info!("File created: {:?}", relative)
            },
            _ => {}
        }
    }
}

pub fn check(config: PathBuf, thorough: bool) -> Result<(), Error> {
    let config = check_config(&config)?;
    let dotfiles = load_dotfiles(&config)?;

    info!("Checking for absent content in {:?}", config.contents());
    let absent_contents = dotfiles.get_absent_files(config.contents().as_path());
    if absent_contents.is_empty() {
        info!("No absent content.")
    }
    else {
        let msg = format!("Absent content: {:?}", absent_contents);
        let err = DotfilesError::new(msg);
        Err(err)?
    }

    let home = env::home_dir().unwrap();
    info!("Checking for symlinks in {:?}", home);
    let symlinks = dotfiles.get_symlinks(config.contents().as_path(), home.as_path());
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

    save_dotfiles(&config, dotfiles)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use commands::*;
    use std::fs;
    use tempdir::TempDir;

    fn setup_config() -> (TempDir, Config) {
        let dir = TempDir::new("dotfilesctl_test").unwrap();
        let target = dir.path().join("target");
        fs::create_dir(&target).unwrap();
        let config = dir.path().join("config.toml");
        init(&config, &target, false).unwrap();
        let config = check_config(&config).unwrap();
        assert_eq!(target, config.target);
        (dir, config)
    }

    #[test]
    fn test_empty_dotfiles() {
        let (_dir, config) = setup_config();
        let dotfiles = load_dotfiles(&config).unwrap();
        save_dotfiles(&config, dotfiles).unwrap();
        let dotfiles = load_dotfiles(&config).unwrap();
        assert_eq!(dotfiles, Dotfiles::new(Some(vec![])));
    }
}