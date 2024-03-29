use crate::config::*;
use crate::dotfiles::*;
use crate::util::*;
use anyhow::Result;
use std::io;
use std::io::Write;
use std::path::{Component, PathBuf};

pub use crate::config::init;

pub fn list(config: &PathBuf) -> Result<()> {
    let config = Config::load(config)?;
    let dotfiles = Dotfiles::load(&config)?;
    for file in dotfiles.get_files() {
        println!(
            "{}",
            result_from_option(
                file.to_str(),
                format!("{:?} is not a valid UTF-8 path", file)
            )?
        )
    }
    Ok(())
}

pub fn check(config: &PathBuf, repair: bool, force: bool) -> Result<()> {
    let config = Config::load(config)?;
    let dotfiles = Dotfiles::load(&config)?;

    fn force_behaviour(_: &PathBuf) -> Result<RepairAction> {
        Ok(RepairAction::Delete)
    }

    fn ask_behaviour(path: &PathBuf) -> Result<RepairAction> {
        print!("Delete {:?} [y/N]? ", path);
        io::stdout().flush()?;
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer)?;
        match buffer.as_str().trim() {
            "" | "N" => Ok(RepairAction::Skip),
            "y" => Ok(RepairAction::Delete),
            _ => Err(anyhow!("Invalid answer: {}", buffer))?,
        }
    }

    match dotfiles.check(&config) {
        Ok(()) => info!("Checking successful!"),
        Err(err) => {
            if repair {
                warn!("Found problems during checking:");
                warn!("{}", err);
                info!("Attempting to repair problems");
                let result = dotfiles.repair(
                    &config,
                    if force {
                        force_behaviour
                    } else {
                        ask_behaviour
                    },
                )?;
                match result {
                    RepairResult::Successful => {
                        info!("Rechecking");
                        dotfiles.check(&config)?
                    }
                    RepairResult::Skipped => {
                        warn!("Skipped some files, problems remain")
                    }
                }
            } else {
                Err(err)?
            }
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
    } else {
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
                    format!("{:?} is not a valid UTF-8 path", path),
                )?;
                if !str.starts_with('.') {
                    Err(anyhow!(
                        "Only dotfiles can be tracked, {:?} does not start with a dot",
                        path
                    ))?
                }
            }
            _ => Err(anyhow!(
                "Only dotfiles can be tracked, {:?} does not start with a normal path component",
                path
            ))?,
        };
        Ok(())
    }

    dotfiles
        .track(
            &config,
            file,
            if force {
                force_behaviour
            } else {
                check_behaviour
            },
        )?
        .save(&config)?;
    Ok(())
}

pub fn untrack(config: &PathBuf, file: &PathBuf, force: bool) -> Result<()> {
    let config = Config::load(config)?;
    let dotfiles = Dotfiles::load(&config)?;
    dotfiles.check(&config)?;

    fn force_behaviour(_: &PathBuf) -> Result<()> {
        Ok(())
    }

    fn ask_behaviour(path: &PathBuf) -> Result<()> {
        print!("Delete {:?} [y/N]? ", path);
        io::stdout().flush()?;
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer)?;
        match buffer.as_str().trim() {
            "y" => Ok(()),
            _ => Err(anyhow!("Not deleting"))?,
        }
    }

    dotfiles
        .untrack(
            &config,
            file,
            if force {
                force_behaviour
            } else {
                ask_behaviour
            },
        )?
        .save(&config)?;
    Ok(())
}

pub fn set_executable(config: &PathBuf, file: &PathBuf, mode: Executable) -> Result<()> {
    let config = Config::load(config)?;
    let dotfiles = Dotfiles::load(&config)?;
    dotfiles.check(&config)?;
    dotfiles
        .set_executable(&config, file, mode)?
        .save(&config)?;
    Ok(())
}
