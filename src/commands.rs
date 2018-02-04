use failure::Error;
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
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
        let mut buf = PathBuf::new();
        buf.push(self.target.clone());
        buf.push("dotfiles.toml");
        buf
    }

    fn contents(&self) -> PathBuf {
        let mut buf = PathBuf::new();
        buf.push(self.target.clone());
        buf.push("contents");
        buf
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

    fn canonicalize(&self) -> Dotfiles {
        match self.files {
            Some(_) => Dotfiles::new(self.files.clone()),
            None => Dotfiles::new(Some(vec![]))
        }
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
        let err = DotfilesError::new(format!("{} is not a directory", target.to_string_lossy()));
        Err(err)?
    }
    else {
        let target = target.canonicalize()?;
        println!("Installing a fresh config in {}", config.to_string_lossy());
        if !config.is_file() || force {
            let contents = toml::to_string(&Config::new(target))?;
            File::create(config)?.write(contents.as_bytes())?;
            Ok(())
        }
        else {
            let err = DotfilesError::new(format!("{} exists but --force has not been specified", config.to_string_lossy()));
            Err(err)?
        }
    }
}

pub fn watch(config: PathBuf) -> Result<(), Error> {
    let config = check_config(&config)?;
    let (tx, rx) = channel();
    let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_secs(2))?;
    println!("Watching file changes in target {}", config.target.to_string_lossy());
    watcher.watch(config.target.clone(), RecursiveMode::Recursive)?;
    loop {
        let event = rx.recv()?;
        match event {
            DebouncedEvent::Create(created) => {
                let relative = relative_to(config.target.as_path(), created.as_path());
                println!("File created: {}", relative.to_string_lossy())
            },
            _ => {}
        }
    }
}

pub fn check(config: PathBuf, thorough: bool) -> Result<(), Error> {
    let config = check_config(&config)?;
    let dotfiles = load_dotfiles(&config)?;

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