extern crate anyhow;
#[macro_use]
extern crate clap;
extern crate clap_complete;
extern crate dirs;
extern crate env_logger;
extern crate fs_extra;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;
#[macro_use]
#[cfg(test)]
extern crate proptest;
#[macro_use]
extern crate serde_derive;
#[cfg(test)]
extern crate tempfile;
#[macro_use]
extern crate thiserror;
extern crate toml;
extern crate xdg;

mod config;
mod commands;
mod dotfiles;
mod paths;
mod util;

use anyhow::Error;
use clap::App;
use clap_complete::{generate, Shell};
use log::LevelFilter;
use std::io;
use std::path::PathBuf;
use std::str::FromStr;
use util::*;

fn exec() -> Result<()> {
    let mut builder = pretty_env_logger::formatted_builder();
    builder.filter(None, LevelFilter::Debug);
    builder.init();

    let yaml = load_yaml!("../resources/cli.yml");

    let mut cli = App::from_yaml(yaml)
        .name(util::APP_NAME)
        .version(util::APP_VERSION);

    let matches = cli.clone().get_matches();
    let force = matches.is_present("force");

    let config = matches
        .value_of("config")
        .map(PathBuf::from)
        .ok_or(() /* dummy */)
        .or_else(|()| config::get_path())?;

    if let Some((cmd, matches)) = matches.subcommand() {
        match cmd {
            "init" => commands::init(
                &config,
                &PathBuf::from(matches.value_of("target").unwrap()),
                matches.value_of("home").map(PathBuf::from),
                force
            ),
            "check" => commands::check(
                &config,
                matches.is_present("repair"),
                force
            ),
            "completions" => {
                let shell = matches.value_of("shell").unwrap();
                generate(
                    Shell::from_str(shell).map_err(Error::msg)?,
                    &mut cli,
                    "dotfilesctl",
                    &mut io::stdout()
                );
                Ok(())
            },
            "list" => commands::list(&config),
            "track" => commands::track(
                &config,
                &PathBuf::from(matches.value_of("file").unwrap()),
                matches.is_present("skip_check"),
                force
            ),
            "untrack" => commands::untrack(
                &config,
                &PathBuf::from(matches.value_of("file").unwrap()),
                force
            ),
            _ => {
                cli.print_help()?;
                Ok(())
            }
        }
    }
    else {
        cli.print_help()?;
        Ok(())
    }
}

fn main() {
    match exec() {
        Ok(()) => (),
        Err(err) => error!("{}", err)
    }
}
