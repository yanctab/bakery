pub mod build;
pub mod clean;
pub mod deploy;
pub mod handler;
pub mod list;
pub mod setup;
pub mod shell;
pub mod sync;
pub mod upload;

use indexmap::IndexMap;
use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use crate::cli::Cli;
use crate::constants::BkryConstants;
use crate::error::BError;
use crate::executers::docker::Docker;
use crate::workspace::{Workspace, WsBuildMetadataHandler, WsId};

#[derive(Clone, PartialEq, Debug)]
pub enum Variant {
    RELEASE,
    DEV,
    TEST,
}

// Implement Display for to-string conversion (lowercase output)
impl fmt::Display for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let variant_str = match self {
            Variant::RELEASE => "release",
            Variant::DEV => "dev",
            Variant::TEST => "test",
        };
        write!(f, "{}", variant_str)
    }
}

// Implement FromStr for from-string conversion (lowercase input)
impl FromStr for Variant {
    type Err = BError;

    fn from_str(s: &str) -> Result<Self, BError> {
        match s {
            "release" => Ok(Variant::RELEASE),
            "dev" => Ok(Variant::DEV),
            "test" => Ok(Variant::TEST),
            _ => Err(BError::ValueError(format!("Invalid Build Variant: {}", s))),
        }
    }
}

// Bakery SubCommand
pub trait BCommand {
    fn setup_context(&self, ctx: Vec<String>) -> IndexMap<String, String> {
        let context: IndexMap<String, String> = ctx
            .iter()
            .map(|c| {
                let v: Vec<&str> = c.split('=').collect();
                (v[0].to_string(), v[1].to_string())
            })
            .collect();
        context
    }

    fn setup_env(&self, env: Vec<String>) -> HashMap<String, String> {
        let mut variables: HashMap<String, String> = env
            .iter()
            .map(|e| {
                let v: Vec<&str> = e.split('=').collect();
                (v[0].to_string(), v[1].to_string())
            })
            .collect();
        variables.insert(String::from("BKRY_WORKSPACE_ID"), WsId::get());
        variables
    }

    fn execute(&self, cli: &Cli, _workspace: &mut Workspace) -> Result<(), BError> {
        cli.info(format!("Execute command {}", self.cmd_str()));
        Ok(())
    }

    fn is_docker_required(&self) -> bool {
        false
    }

    fn docker_pull(&self, cli: &Cli, workspace: &Workspace) -> Result<(), BError> {
        let docker: Docker = Docker::new(workspace.settings().docker_image(), false);
        return docker.pull(cli);
    }

    fn bootstrap(
        &self,
        cmd_line: &Vec<String>,
        cli: &Cli,
        workspace: &Workspace,
        volumes: &Vec<String>,
        interactive: bool,
    ) -> Result<(), BError> {
        let docker: Docker = Docker::new(workspace.settings().docker_image(), interactive);

        if self.docker_pull(cli, workspace).is_err() {
            cli.debug(format!(
                "Failed to pull the Docker image '{}'",
                workspace.settings().docker_image()
            ));
        }

        /*
         * When we bootstrap bakery into docker we should make sure that we pull
         * in the entire env from the parent
         */
        let env: HashMap<String, String> = cli.env();

        cli.info(format!("Bootstrap bakery into '{}'", docker.image()));
        cli.debug(format!("env: {:?}", env));

        if !PathBuf::from("/usr/bin/docker").exists() {
            return Err(BError::DockerError());
        }
        return docker.bootstrap_bkry(
            cmd_line,
            cli,
            &workspace.settings().docker_top_dir(),
            &workspace.settings().work_dir(),
            workspace.settings().docker_args(),
            volumes,
            &env,
        );
    }

    fn get_config_name(&self, cli: &Cli, workspace_dir: &PathBuf) -> String {
        let config: String = self
            .get_arg_str(cli, "config", self.cmd_type())
            .unwrap_or(String::from(""));

        if !config.is_empty() {
            return config;
        }

        /*
         * If no build config is specified then we will check if there is any build metadata
         * available for this workspace and then we use that as input this will make it
         * possible to lock a workspace for a specific build metadata define once when
         * setting up the workspace
         */
        let meta: WsBuildMetadataHandler = WsBuildMetadataHandler::new(
            workspace_dir,
            &cli.get_home_dir().join(PathBuf::from(".bakery")),
            None,
        );

        match meta.config() {
            Ok(config) => {
                return config;
            }
            Err(_err) => {}
        }

        return String::from("");
    }

    fn get_cmd_line(&self, cli: &Cli, config: &String, variant: Option<Variant>) -> Vec<String> {
        /*
         * If there is no --config <build config> available as part of the cli
         * then the config might have been set by bakery through the metadata
         * under ~/.bkry/workspaces as a way to lock a workspace to
         * specific set of build parameters. This means that we are loosing
         * track of how bakery is called this tries to make sure
         * that we can still keep track of it by appending the build config
         * to the command line again if there was none specified by the user.
         * For more information please see the bakery documentation about build
         * metadata.
         */
        if !self.was_cfg_arg_specified() {
            let mut cmd_line: Vec<String> = cli.get_cmd_line();
            let mut variant_found: bool = false;

            for arg in &cmd_line {
                match arg.as_str() {
                    "-a" | "--variant" => {
                        variant_found = true; // Flag that the variant argument is found
                    }
                    _ => {}
                }
            }

            cmd_line.append(&mut vec![format!("-c {}", config)]);
            if !variant_found && variant.is_some() {
                cmd_line.append(&mut vec![format!(
                    "-a {}",
                    variant.unwrap_or(BkryConstants::BKRY_DEFAULT_VARIANT)
                )]);
            }

            return cmd_line;
        }

        cli.get_cmd_line()
    }

    fn get_config(&self, cli: &Cli, workspace_dir: &PathBuf) -> Result<String, BError> {
        let mut config: String = String::new();

        match self.get_arg_str(cli, "config", self.cmd_type()) {
            Ok(cfg) => {
                cli.debug(String::from("Config specified as an argument"));
                self.set_cfg_arg_specified(true);
                config = cfg;
            }
            Err(_err) => {
                if !self.args_required() {
                    cli.debug(String::from("No args return NA"));
                    return Ok("NA".to_string());
                }

                self.set_cfg_arg_specified(false);
                cli.debug(String::from("No config specified as an argument, falling back to metadata for build parameters"));
                config = self.get_config_name(cli, workspace_dir);
            }
        }

        if config.is_empty() && self.args_required() {
            return Err(BError::NoBuildConfigError());
        }

        return Ok(config);
    }

    fn get_variant(&self, cli: &Cli, workspace_dir: &PathBuf) -> Result<Variant, BError> {
        if self.was_cfg_arg_specified() {
            return self.get_arg_variant(cli, "variant", self.cmd_type());
        }

        /*
         * If no build config is specified then we will check if there is any build metadata
         * available for this workspace and then we use that as input this will make it
         * possible to lock a workspace for a specific build metadata define once when
         * setting up the workspace
         */
        let meta: WsBuildMetadataHandler = WsBuildMetadataHandler::new(
            workspace_dir,
            &cli.get_home_dir().join(PathBuf::from(".bakery")),
            None,
        );

        return meta.variant();
    }

    fn get_arg_str(&self, cli: &Cli, id: &str, cmd: &str) -> Result<String, BError> {
        if let Some(sub_matches) = cli.get_args().subcommand_matches(cmd) {
            if sub_matches.contains_id(id) {
                if let Some(value) = sub_matches.get_one::<String>(id) {
                    return Ok(value.clone());
                }
            }
        }
        return Err(BError::CliError(format!("Failed to read arg {}", id)));
    }

    fn get_arg_variant(&self, cli: &Cli, id: &str, cmd: &str) -> Result<Variant, BError> {
        if let Some(sub_matches) = cli.get_args().subcommand_matches(cmd) {
            if sub_matches.contains_id(id) {
                if let Some(value) = sub_matches.get_one::<String>(id) {
                    match value.parse::<Variant>() {
                        Ok(variant) => return Ok(variant.clone()),
                        Err(_e) => {
                            return Err(BError::ParseTasksError(format!(
                                "Invalid variant '{}'",
                                value
                            )))
                        }
                    }
                }
            }
        }
        return Err(BError::CliError(format!("Failed to read arg {}", id)));
    }

    fn get_arg_flag(&self, cli: &Cli, id: &str, cmd: &str) -> Result<bool, BError> {
        if let Some(sub_matches) = cli.get_args().subcommand_matches(cmd) {
            if sub_matches.contains_id(id) {
                let flag: bool = sub_matches.get_flag(id);
                return Ok(flag);
            }
        }
        return Err(BError::CliError(format!("Failed to read arg {}", id)));
    }

    fn get_arg_bool(&self, cli: &Cli, id: &str, cmd: &str) -> Result<bool, BError> {
        if let Some(sub_matches) = cli.get_args().subcommand_matches(cmd) {
            if sub_matches.contains_id(id) {
                if let Some(value) = sub_matches.get_one::<String>(id) {
                    return Ok(value == "true");
                }
            }
        }
        return Err(BError::CliError(format!("Failed to read arg {}", id)));
    }

    fn get_arg_many<'a>(
        &'a self,
        cli: &'a Cli,
        id: &str,
        cmd: &str,
    ) -> Result<Vec<String>, BError> {
        if let Some(sub_matches) = cli.get_args().subcommand_matches(cmd) {
            if sub_matches.contains_id(id) {
                let many: Vec<String> = sub_matches
                    .get_many::<String>(id)
                    .unwrap_or_default()
                    .collect::<Vec<_>>()
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
                return Ok(many);
            }
            return Ok(Vec::new());
        }
        return Err(BError::CliError(format!("Failed to read arg {}", id)));
    }

    // Return a clap sub-command containing the args
    // for the bakery command
    fn subcommand(&self) -> &clap::Command;

    fn cmd_str(&self) -> &str;

    fn set_cfg_arg_specified(&self, value: bool);

    fn was_cfg_arg_specified(&self) -> bool;

    fn args_required(&self) -> bool;

    fn cmd_type(&self) -> &str;
}

pub struct BBaseCommand {
    cmd_str: String,
    sub_cmd: clap::Command,
    interactive: bool,
    require_docker: bool,
    cfg_arg_available: OnceCell<bool>,
    args_required: bool,
}

pub fn get_supported_cmds() -> HashMap<&'static str, Box<dyn BCommand>> {
    let mut supported_cmds: HashMap<&'static str, Box<dyn BCommand>> = HashMap::new();

    // Add supported commands to the HashMap
    supported_cmds.insert("build", Box::new(BuildCommand::new()));
    supported_cmds.insert("clean", Box::new(CleanCommand::new()));
    supported_cmds.insert("list", Box::new(ListCommand::new()));
    supported_cmds.insert("shell", Box::new(ShellCommand::new()));
    supported_cmds.insert("deploy", Box::new(DeployCommand::new()));
    supported_cmds.insert("upload", Box::new(UploadCommand::new()));
    supported_cmds.insert("setup", Box::new(SetupCommand::new()));
    supported_cmds.insert("sync", Box::new(SyncCommand::new()));

    // Add more commands as needed

    supported_cmds
}

pub use build::BuildCommand;
pub use clean::CleanCommand;
pub use deploy::DeployCommand;
pub use handler::CmdHandler;
pub use list::ListCommand;
pub use setup::SetupCommand;
pub use shell::ShellCommand;
pub use sync::SyncCommand;
pub use upload::UploadCommand;
