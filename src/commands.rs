use failure::Error;
use std::fmt;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use toml;

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    target: PathBuf
}

impl Config {
    fn new(target: PathBuf) -> Config {
        Config {
            target: target
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
        DotfilesError {
            description: description
        }
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
    let _ = check_config(&config);
    Ok(())
}