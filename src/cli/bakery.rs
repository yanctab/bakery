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
        cmd_require_docker: bool,
        err: &String,
        code: i32) -> ! {

        /*
         * Avoid log for the same command twice. 
         * - If we're inside Docker, we log.
         * - If we're outside Docker and the command does not require Docker, we also log.
         */
        let inside_docker: bool = Docker::inside_docker();
        if inside_docker || (!inside_docker && !cmd_require_docker) {
            if !err.is_empty() {
                self.cli.error(err.to_string());
            }

            if code != 0 {
                self.cli.error(format!("Failed to execute '{}'", cmd));
            }
        }

        std::process::exit(code);
    }

    pub fn unwrap_or_exit<T>(
        &self,
        cmd: &str,
        cmd_require_docker: bool,
        result: Result<T, BError>,
    ) -> T {
        result.unwrap_or_else(|err| {
            self.bkry_exit(&cmd.to_string(), cmd_require_docker, &err.to_string(), 1);
        })
    }

    pub fn bake(&self) {
        let work_dir: PathBuf = self.cli.get_curr_dir();
        let home_dir: PathBuf = self.cli.get_home_dir();
        /*
         * Since we cannot reliably determine whether the command requires Docker,
         * we will assume that it doesn't 
         */
        let mut cmd_require_docker: bool = false;
        
        let cfg_handler: WsConfigFileHandler = WsConfigFileHandler::new(&work_dir, &home_dir);
        let cmd_name: &str = self.cli.get_args().subcommand_name().unwrap();
        /*
         * Verify that a 'workspace.json' file can be found in one of the configuration directories:
         * the current directory (.), the user config directory (~/.bakery), or the system config directory (/etc/bakery).
         * If no 'workspace.json' is found in any of these locations, exit with an "invalid workspace" error.
         */
        self.cli.debug("Verify that we have a workspace.json available".to_string());
        self.unwrap_or_exit::<()>(cmd_name, cmd_require_docker, cfg_handler.verify_ws());

        let settings: WsSettingsHandler =
            self.unwrap_or_exit::<WsSettingsHandler>(cmd_name, cmd_require_docker, cfg_handler.ws_settings());
        let cmd_result: Result<&Box<dyn BCommand>, BError> = self.cli.get_command(cmd_name);

        match cmd_result {
            Ok(command) => {
                cmd_require_docker = command.is_docker_required();
                let config: WsBuildConfigHandler = self.unwrap_or_exit::<WsBuildConfigHandler>(
                    cmd_name,
                    cmd_require_docker,
                    cfg_handler.build_config(&command.get_config_name(&self.cli),
                    &settings),
                );

                self.cli.debug(format!("Current dir: {:?}", work_dir));
                self.cli.debug(format!("Home dir: {:?}", home_dir));
                self.cli.debug(format!("Workspace dir: {:?}", work_dir));

                /*
                 * Create the workspace configuration, which consists of the workspace settings and a
                 * selected build configuration. The workspace settings are defined in 'workspace.json',
                 * while the build configuration is defined in one of the available build JSON files.
                 */
                let mut workspace: Workspace = self.unwrap_or_exit::<Workspace>(
                    cmd_name,
                    cmd_require_docker,
                    Workspace::new(
                    Some(work_dir),
                    Some(settings),
                    Some(config),
                ));

                /*
                 * We expand any context variables defined in the workspace configuration to
                 * make sure that we get the true paths
                 */
                self.unwrap_or_exit::<()>(
                    cmd_name,
                    cmd_require_docker,
                     workspace.expand_ctx());
                self.cli.debug(format!("Configs dir: {:?}", workspace.settings().configs_dir()));
                self.cli.debug(format!("Includes dir: {:?}", workspace.settings().include_dir()));
                self.cli.debug(format!("Artifacts dir: {:?}", workspace.settings().artifacts_dir()));

                /*
                 * Verify that the directories defined in 'workspace.json' actually exist.
                 * These may include paths like 'configs', 'scripts', etc.
                 */
                self.unwrap_or_exit::<()>(
                    &cmd_name.to_string(),
                    cmd_require_docker,
                    workspace.verify_ws()
                );
        
                self.cli.debug(format!("Executing '{}' cmd", cmd_name));
                self.unwrap_or_exit::<()>(
                    &cmd_name.to_string(),
                    cmd_require_docker,
                    command.execute(&self.cli, &mut workspace),
                );
            }
            Err(err) => {
                self.bkry_exit(
                    &cmd_name.to_string(),
                    cmd_require_docker,
                     &err.to_string(), 1);
            }
        }
        
        self.bkry_exit(
            &cmd_name.to_string(),
            cmd_require_docker,
            &Default::default(),
            0,
        );
    }
}
