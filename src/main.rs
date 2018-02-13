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

use clap::{App, Arg, Shell, SubCommand};
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

    let init_command = SubCommand::with_name("init")
        .arg(
            Arg::with_name("force")
                .short("f")
                .long("force")
                .help("Overwrite existing configuration file")
        )
        .arg(
            Arg::with_name("home")
                .short("h")
                .long("home")
                .help("Specify home directory (default: auto-detected)")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("target")
                .value_name("DIR")
                .help("Directory to scan for dotfiles")
                .required(true)
        );

    let watch_command = SubCommand::with_name("watch");

    let check_command = SubCommand::with_name("check")
        .arg(
            Arg::with_name("thorough")
                .short("t")
                .long("throrough")
                .help("Throrough check: scan dotfiles in home directory for dangling symlinks")
        )
        .arg(
            Arg::with_name("repair")
                .short("r")
                .long("repair")
                .help("Repair broken files")
        );

    let completions_command = SubCommand::with_name("completions")
        .about("Generates completion scripts for your shell")
        .arg(
            Arg::with_name("shell")
                .value_name("SHELL")
                .required(true)
                .possible_values(&["bash", "fish", "zsh"])
                .help("The shell to generate the script for")
        );

    let xdg_dirs = xdg::BaseDirectories::with_prefix(APP_NAME).unwrap();

    let cli = App::new(APP_NAME)
        .version(APP_VERSION)
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file")
                .takes_value(true)
        )
        .subcommand(init_command)
        .subcommand(watch_command)
        .subcommand(check_command)
        .subcommand(completions_command);

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
