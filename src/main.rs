#[macro_use] extern crate clap;
#[macro_use] extern crate failure;
#[macro_use] extern crate proptest;
#[macro_use] extern crate serde_derive;
extern crate toml;
extern crate xdg;

mod commands;
mod paths;

use clap::{Arg, App, SubCommand};
use std::path::PathBuf;

static APP_VERSION: &'static str = crate_version!();
static APP_NAME: &'static str = crate_name!();

fn main() {
    let init_command =
        SubCommand::with_name("init")
            .arg(Arg::with_name("force")
                .short("f")
                .long("force")
                .help("Overwrite existing configuration file"))
            .arg(Arg::with_name("dir")
                .value_name("DIR")
                .help("Directory to scan for dotfiles")
                .required(true));

    let watch_command =
        SubCommand::with_name("watch");

    let xdg_dirs = xdg::BaseDirectories::with_prefix(APP_NAME).unwrap();

    let matches =
        App::new(APP_NAME)
            .version(APP_VERSION)
            .arg(
                Arg::with_name("config")
                    .short("c")
                    .long("config")
                    .value_name("FILE")
                    .help("Sets a custom config file")
                    .takes_value(true))
            .subcommand(init_command)
            .subcommand(watch_command)
            .get_matches();

    let config = matches.value_of("config").map(PathBuf::from).unwrap_or_else(|| {
        xdg_dirs.place_config_file("config.toml").unwrap()
    });

    let result = match matches.subcommand() {
        ("init", Some(matches)) =>
            commands::init(
                config,
                PathBuf::from(matches.value_of("dir").unwrap()),
                matches.is_present("force")),
        ("watch", Some(_)) =>
            commands::watch(config),
        _ =>
            Ok(println!("{}", matches.usage()))
    };

    result.unwrap();
}
