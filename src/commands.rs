use failure::Error;
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use std::fmt;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::time::Duration;
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

pub fn init(config: PathBuf, target: PathBuf, force: bool) -> Result<(), Error> {
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