use crate::CliError;
use clap::{crate_description, crate_name, crate_version, App, Arg};

pub struct Config {
    pub command: String,
}

impl Config {
    pub fn new() -> Result<Self, CliError> {
        let matches = App::new(crate_name!())
            .version(crate_version!())
            .about(crate_description!())
            .arg(
                Arg::new("command")
                    .default_value("login")
                    // .hide_default_value(true)
                    .index(1)
                    .help("Command to run"),
            )
            .get_matches();

        let command = matches.value_of("command").unwrap();

        Ok(Config {
            command: command.to_string(),
        })
    }
}