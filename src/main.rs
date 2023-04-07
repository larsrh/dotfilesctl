extern crate anyhow;
#[macro_use]
extern crate clap;
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

use clap::{App, Shell};
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

    let cli = App::from_yaml(yaml)
        .name(util::APP_NAME)
        .version(util::APP_VERSION);

    let matches = cli.clone().get_matches();
    let force = matches.is_present("force");

    let config = matches
        .value_of("config")
        .map(PathBuf::from)
        .ok_or(() /* dummy */)
        .or_else(|()| config::get_path())?;

    match matches.subcommand() {
        ("init", Some(matches)) => commands::init(
            &config,
            &PathBuf::from(matches.value_of("target").unwrap()),
            matches.value_of("home").map(PathBuf::from),
            force
        ),
        ("check", Some(matches)) => commands::check(
            &config,
            matches.is_present("repair"),
            force
        ),
        ("completions", Some(matches)) => {
            let shell = matches.value_of("shell").unwrap();
            cli.clone().gen_completions_to(
                "dotfilesctl",
                Shell::from_str(shell).unwrap(),
                &mut io::stdout()
            );
            Ok(())
        }
        ("list", Some(_)) => commands::list(&config),
        ("track", Some(matches)) => commands::track(
            &config,
            &PathBuf::from(matches.value_of("file").unwrap()),
            matches.is_present("skip_check"),
            force
        ),
        ("untrack", Some(matches)) => commands::untrack(
            &config,
            &PathBuf::from(matches.value_of("file").unwrap()),
            force
        ),
        _ => {
            println!("{}", matches.usage());
            Ok(())
        }
    }
}

fn main() {
    match exec() {
        Ok(()) => (),
        Err(err) => error!("{}", err)
    }
}
