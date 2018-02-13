use config::*;
use dotfiles::*;
use failure::Error;
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::time::Duration;
use paths::*;

pub use config::init;

pub fn watch(config: PathBuf) -> Result<(), Error> {
    let config = Config::load(&config)?;
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
            }
            _ => {}
        }
    }
}

// TODO implement thorough checking
pub fn check(config: PathBuf, _thorough: bool, repair: bool) -> Result<(), Error> {
    let config = Config::load(&config)?;
    let dotfiles = Dotfiles::load(&config)?;

    match dotfiles.check(&config) {
        Ok(()) => info!("Checking successful!"),
        Err(err) => if repair {
            warn!("Found problems during checking:");
            warn!("{}", err);
            info!("Attempting to repair problems");
            dotfiles.repair(&config)?;
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
