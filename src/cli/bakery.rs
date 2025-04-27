use crate::cli::{BLogger, Cli};
use crate::commands::BCommand;
use crate::configs::WsConfigFileHandler;
use crate::error::BError;
use crate::workspace::{Workspace, WsBuildConfigHandler, WsSettingsHandler};

use clap::Command;
use std::path::PathBuf;

use super::BSystem;

pub struct Bakery {
    cli: Cli,
}

impl Bakery {
    pub fn new() -> Self {
        /*
            TODO: We should try and use command! macro in clap so
            the about, author and version can be read out from the
            Cargo.toml
        */
        let cli: Cli = Cli::new(
            Box::new(BLogger::new()),
            Box::new(BSystem::new()),
            Command::new("bakery")
                .version(env!("CARGO_PKG_VERSION"))
                .subcommand_required(true)
                .arg_required_else_help(true)
                .about("Build engine for the Yocto/OE using docker")
                .author("bakery by yanctab(yanctab.com)"),
            None,
        );

        Bakery { cli: cli }
    }

    pub fn unwrap_or_exit<T>(&self, result: Result<T, BError>) -> T {
        result.unwrap_or_else(|err| {
            self.cli.error(err.to_string());
            std::process::exit(1);
        })
    }

    pub fn bake(&self) {
        let work_dir: PathBuf = self.cli.get_curr_dir();
        let home_dir: PathBuf = self.cli.get_home_dir();
        let cfg_handler: WsConfigFileHandler = WsConfigFileHandler::new(&work_dir, &home_dir);
        let settings: WsSettingsHandler = self.unwrap_or_exit(cfg_handler.ws_settings());
        let cmd_name: &str = self.cli.get_args().subcommand_name().unwrap();
        let cmd_result: Result<&Box<dyn BCommand>, BError> = self.cli.get_command(cmd_name);
        let mut _res: () = self.unwrap_or_exit(settings.verify_ws());

        match cmd_result {
            Ok(command) => {
                let config: WsBuildConfigHandler = self.unwrap_or_exit(
                    cfg_handler.build_config(&command.get_config_name(&self.cli), &settings),
                );
                let mut workspace: Workspace = self.unwrap_or_exit(Workspace::new(
                    Some(work_dir),
                    Some(settings),
                    Some(config),
                ));
                let res: Result<(), BError> = command.execute(&self.cli, &mut workspace);
                res.unwrap_or_else(|err| {
                    self.cli.error(format!(
                        "Failed to execute '{}', with error '{}'",
                        cmd_name,
                        err.to_string()
                    ));
                    std::process::exit(1);
                });
            }
            Err(err) => {
                self.cli.error(format!("{}", err.to_string()));
                std::process::exit(1);
            }
        }
        std::process::exit(0);
    }
}
