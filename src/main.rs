#[macro_use]
extern crate clap;
extern crate env_logger;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate log;
extern crate notify;
extern crate pretty_env_logger;
#[macro_use] #[cfg(test)]
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
use log::LevelFilter;
use std::io;
use std::path::PathBuf;
use std::str::FromStr;

static APP_VERSION: &'static str = crate_version!();
static APP_NAME: &'static str = crate_name!();

fn main() {
    let mut builder = pretty_env_logger::formatted_builder().unwrap();
    builder.filter(None, LevelFilter::Debug);
    builder.init();

    let xdg_dirs = xdg::BaseDirectories::with_prefix(APP_NAME).unwrap();

    let yaml = load_yaml!("../resources/cli.yml");

    let cli = App::from_yaml(yaml)
        .name(APP_NAME)
        .version(APP_VERSION);

    let matches = cli.clone().get_matches();

    let config = matches
        .value_of("config")
        .map(PathBuf::from)
        .unwrap_or_else(|| xdg_dirs.place_config_file("config.toml").unwrap());

    let result = match matches.subcommand() {
        ("init", Some(matches)) => commands::init(
            &config,
            &PathBuf::from(matches.value_of("dir").unwrap()),
            matches.value_of("home").map(PathBuf::from),
            matches.is_present("force")
        ),
        ("watch", Some(_)) => commands::watch(config),
        ("check", Some(matches)) => commands::check(
            config,
            matches.is_present("thorough"),
            matches.is_present("repair")
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
        _ => Ok(println!("{}", matches.usage()))
    };

    match result {
        Ok(()) => (),
        Err(err) => error!("{}", err)
    }
}
