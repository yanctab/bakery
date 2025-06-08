use crate::cli::{BLogger, Cli};
use crate::commands::BCommand;
use crate::configs::WsConfigFileHandler;
use crate::error::BError;
use crate::workspace::{Workspace, WsBuildConfigHandler, WsSettingsHandler};
use crate::executers::Docker;

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

    pub fn bkry_exit(
        &self,
        cmd: &String,
        err: &String,
        code: i32) -> ! {

        if !err.is_empty() {
            self.cli.error(err.clone());
        }

        if code != 0 {
            self.cli.error(format!("Failed to execute '{}'", cmd));
        }

        std::process::exit(code);
    }

    pub fn unwrap_or_exit<T>(
        &self,
        cmd: &str,
        result: Result<T, BError>,
    ) -> T {
        result.unwrap_or_else(|err| {
            self.bkry_exit(&cmd.to_string(), &err.to_string(), 1);
        })
    }

    pub fn bake(&self) {
        let work_dir: PathBuf = self.cli.get_curr_dir();
        let home_dir: PathBuf = self.cli.get_home_dir();
        let cfg_handler: WsConfigFileHandler = WsConfigFileHandler::new(&work_dir, &home_dir);
        let cmd_name: &str = self.cli.get_args().subcommand_name().unwrap();
        /*
         * Verify that a 'workspace.json' file can be found in one of the configuration directories:
         * the current directory (.), the user config directory (~/.bakery), or the system config directory (/etc/bakery).
         * If no 'workspace.json' is found in any of these locations, exit with an "invalid workspace" error.
         */
        let mut _res: () = self.unwrap_or_exit::<()>(cmd_name, cfg_handler.verify_ws());

        let settings: WsSettingsHandler =
            self.unwrap_or_exit::<WsSettingsHandler>(cmd_name, cfg_handler.ws_settings());
        let cmd_result: Result<&Box<dyn BCommand>, BError> = self.cli.get_command(cmd_name);

        match cmd_result {
            Ok(command) => {
                let config: WsBuildConfigHandler = self.unwrap_or_exit::<WsBuildConfigHandler>(cmd_name,
                    cfg_handler.build_config(&command.get_config_name(&self.cli), &settings),
                );

                self.cli.debug(format!("Current dir: {:?}", work_dir));
                self.cli.debug(format!("Home dir: {:?}", home_dir));
                self.cli.debug(format!("Workspace dir: {:?}", work_dir));
                self.cli.debug(format!("Configs dir: {:?}", settings.configs_dir()));
                self.cli.debug(format!("Includes dir: {:?}", settings.include_dir()));
                self.cli.debug(format!("Artifacts dir: {:?}", settings.artifacts_dir()));

                let mut workspace: Workspace = self.unwrap_or_exit::<Workspace>(cmd_name,Workspace::new(
                    Some(work_dir),
                    Some(settings),
                    Some(config),
                ));

                /*
                 * Verify that the directories defined in 'workspace.json' actually exist.
                 * These may include paths like 'configs', 'scripts', etc.
                 */
                _res = self.unwrap_or_exit::<()>(cmd_name, workspace.verify_ws());

                self.unwrap_or_exit::<()>(
                    &cmd_name.to_string(),
                    workspace.verify_ws()
                );
        
                self.cli.debug(format!("Executing '{}' cmd", cmd_name));
                self.unwrap_or_exit::<()>(
                    &cmd_name.to_string(),
                    command.execute(&self.cli, &mut workspace),
                );
            }
            Err(err) => {
                self.bkry_exit(&cmd_name.to_string(), &err.to_string(), 1);
            }
        }
        
        self.bkry_exit(
            &cmd_name.to_string(),
            &Default::default(),
            0,
        );
    }
}
