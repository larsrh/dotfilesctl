#[macro_use]
extern crate clap;
extern crate xdg;

use clap::{Arg, App, SubCommand};
use std::path::PathBuf;

static app_version: &'static str = crate_version!();
static app_name: &'static str = crate_name!();

fn main() {
    let init =
        SubCommand::with_name("init");

    let xdg_dirs = xdg::BaseDirectories::with_prefix(app_name).unwrap();

    let matches =
        App::new(app_name)
            .version(app_version)
            .arg(
                Arg::with_name("config")
                    .short("c")
                    .long("config")
                    .value_name("FILE")
                    .help("Sets a custom config file")
                    .takes_value(true))
            .subcommand(init)
            .get_matches();

    let config = matches.value_of("config").map(PathBuf::from).unwrap_or_else(|| {
        xdg_dirs.place_config_file("config.toml").unwrap()
    });

    let (subcommand, matches) = matches.subcommand();

    println!("{}: {}", config.to_string_lossy(), config.exists());
}
