use config::*;
use dotfiles::*;
use failure::Error;
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use paths::*;
use std::io;
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::time::Duration;
use util::DotfilesError;

pub use config::init;

pub fn watch(config: &PathBuf) -> Result<(), Error> {
    let config = Config::load(config)?;
    let (tx, rx) = channel();
    let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_secs(2))?;
    info!("Watching file changes in target {:?}", config.target);
    watcher.watch(config.target.clone(), RecursiveMode::Recursive)?;
    loop {
        let event = rx.recv()?;
        if let DebouncedEvent::Create(created) = event {
            let relative = relative_to(config.target.as_path(), created.as_path());
            info!("File created: {:?}", relative)
        }
    }
}

// TODO implement thorough checking
pub fn check(config: &PathBuf, _thorough: bool, repair: bool, force: bool) -> Result<(), Error> {
    let config = Config::load(config)?;
    let dotfiles = Dotfiles::load(&config)?;

    fn force_behaviour(path: &PathBuf) -> Result<bool, Error> {
        info!("Deleting {:?}", path);
        Ok(false)
    }

    fn ask_behaviour(path: &PathBuf) -> Result<bool, Error> {
        print!("Delete {:?} [y/N]? ", path);
        io::stdout().flush()?;
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer)?;
        match buffer.as_str().trim() {
            "" | "N" => Ok(true),
            "y" => Ok(false),
            _ => {
                let msg = format!("Invalid answer: {}", buffer);
                Err(DotfilesError::new(msg))?
            }
        }
    }

    match dotfiles.check(&config) {
        Ok(()) => info!("Checking successful!"),
        Err(err) => if repair {
            warn!("Found problems during checking:");
            warn!("{}", err);
            info!("Attempting to repair problems");
            dotfiles.repair(
                &config,
                if force {
                    force_behaviour
                }
                else {
                    ask_behaviour
                }
            )?;
            info!("Rechecking");
            dotfiles.check(&config)?;
        }
        else {
            Err(err)?
        }
    }

    dotfiles.save(&config)?;
    Ok(())
}
