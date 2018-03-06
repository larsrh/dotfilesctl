use config::*;
use dotfiles::*;
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use paths::*;
use std::io;
use std::io::Write;
use std::path::{Component, PathBuf};
use std::sync::mpsc::channel;
use std::time::Duration;
use util::*;

pub use config::init;

pub fn watch(config: &PathBuf) -> Result<()> {
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
pub fn check(config: &PathBuf, _thorough: bool, repair: bool, force: bool) -> Result<()> {
    let config = Config::load(config)?;
    let dotfiles = Dotfiles::load(&config)?;

    fn force_behaviour(path: &PathBuf) -> Result<bool> {
        info!("Deleting {:?}", path);
        Ok(false)
    }

    fn ask_behaviour(path: &PathBuf) -> Result<bool> {
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

pub fn track(config: &PathBuf, file: &PathBuf, skip_check: bool, force: bool) -> Result<()> {
    let config = Config::load(config)?;
    let dotfiles = Dotfiles::load(&config)?;
    if skip_check {
        warn!("Skipping check, this is potentially dangerous")
    }
    else {
        dotfiles.check(&config)?;
    }

    fn force_behaviour(_: &PathBuf) -> Result<()> {
        Ok(())
    }

    fn check_behaviour(path: &PathBuf) -> Result<()> {
        match path.components().next().unwrap() {
            Component::Normal(str) => {
                let str = result_from_option(
                    str.to_str(),
                    format!("{:?} is not a valid UTF-8 path", path)
                )?;
                if !str.starts_with('.') {
                    let msg = format!(
                        "Only dotfiles can be tracked, {:?} does not start with a dot",
                        path
                    );
                    Err(DotfilesError::new(msg))?
                }
            }
            _ => {
                let msg = format!(
                    "Only dotfiles can be tracked, {:?} does not start with a normal path component",
                    path
                );
                Err(DotfilesError::new(msg))?
            }
        };
        Ok(())
    }

    dotfiles
        .track(
            &config,
            file,
            if force {
                force_behaviour
            }
            else {
                check_behaviour
            }
        )?
        .save(&config)?;
    Ok(())
}
