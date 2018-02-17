#[macro_use]
extern crate clap;
extern crate env_logger;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate log;
extern crate notify;
extern crate pretty_env_logger;
#[macro_use]
#[cfg(test)]
extern crate proptest;
#[macro_use]
extern crate serde_derive;
extern crate tempdir;
extern crate toml;
extern crate xdg;

mod config;
mod commands;
mod dotfiles;
mod paths;
mod util;

use clap::{App, Shell};
use failure::Error;
use log::LevelFilter;
use std::io;
use std::path::PathBuf;
use std::str::FromStr;

fn exec() -> Result<(), Error> {
    let mut builder = pretty_env_logger::formatted_builder()?;
    builder.filter(None, LevelFilter::Debug);
    builder.init();

    let yaml = load_yaml!("../resources/cli.yml");

    let cli = App::from_yaml(yaml)
        .name(util::APP_NAME)
        .version(util::APP_VERSION);

    let matches = cli.clone().get_matches();

    let config = matches
        .value_of("config")
        .map(PathBuf::from)
        .ok_or(() /* dummy */)
        .or_else(|()| config::get_path())?;

    match matches.subcommand() {
        ("init", Some(matches)) => commands::init(
            &config,
            &PathBuf::from(matches.value_of("dir").unwrap()),
            matches.value_of("home").map(PathBuf::from),
            matches.is_present("force")
        ),
        ("watch", Some(_)) => commands::watch(&config),
        ("check", Some(matches)) => commands::check(
            &config,
            matches.is_present("thorough"),
            matches.is_present("repair"),
            matches.is_present("force")
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
