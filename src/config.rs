use failure::Error;
use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use util::DotfilesError;
use toml;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub target: PathBuf,
    home: Option<PathBuf>
}

impl Config {
    pub fn new(target: PathBuf, home: Option<PathBuf>) -> Config {
        Config { target, home }
    }

    pub fn load(config: &PathBuf) -> Result<Config, Error> {
        let mut contents = String::new();
        File::open(config)?.read_to_string(&mut contents)?;
        let config = toml::from_str::<Config>(contents.as_ref())?;
        Ok(config)
    }

    pub fn get_home(&self) -> Result<PathBuf, DotfilesError> {
        match self.home.clone().or_else(env::home_dir) {
            Some(home) => Ok(home),
            None => {
                let msg = "No home directory configured and none could be detected";
                Err(DotfilesError::new(msg.to_string()))
            }
        }
    }

    pub fn dotfiles(&self) -> PathBuf {
        self.target.join("dotfiles.toml")
    }

    pub fn contents(&self) -> PathBuf {
        self.target.join("contents")
    }
}

pub fn init(
    config: &PathBuf,
    target: &PathBuf,
    home: Option<PathBuf>,
    force: bool
) -> Result<(), Error> {
    if !target.is_dir() {
        let err = DotfilesError::new(format!("{:?} is not a directory", target));
        Err(err)?
    }
    else {
        let target = target.canonicalize()?;
        info!("Installing a fresh config in {:?}", config);
        if !config.is_file() || force {
            let contents = toml::to_string(&Config::new(target, home))?;
            File::create(config)?.write_all(contents.as_bytes())?;
            Ok(())
        }
        else {
            let msg = format!("{:?} exists but --force has not been specified", config);
            let err = DotfilesError::new(msg);
            Err(err)?
        }
    }
}

#[cfg(test)]
pub mod test_util {
    use config::*;
    use std::fs;
    use tempdir::TempDir;

    pub fn setup_config() -> (TempDir, Config) {
        let dir = TempDir::new("dotfilesctl_test").unwrap();
        let home = dir.path().join("home");
        fs::create_dir(&home).unwrap();
        let target = dir.path().join("target");
        fs::create_dir(&target).unwrap();
        let config = dir.path().join("config.toml");
        init(&config, &target, Some(home), false).unwrap();
        let config = Config::load(&config).unwrap();
        assert_eq!(target, config.target);
        fs::create_dir(config.contents()).unwrap();
        (dir, config)
    }
}

#[cfg(test)]
mod tests {
    use config::test_util::*;

    #[test]
    fn test_setup() {
        setup_config();
    }
}
