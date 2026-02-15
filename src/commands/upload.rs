use indexmap::IndexMap;
use once_cell::sync::OnceCell;
use std::collections::HashMap;

use crate::cli::Cli;
use crate::commands::{BBaseCommand, BCommand, BError};
use crate::data::WsContextData;
use crate::workspace::WsCustomSubCmdHandler;
use crate::workspace::{Mode, Workspace};

static BCOMMAND: &str = "upload";
static BCOMMAND_ABOUT: &str = "Upload artifacts to any rtifactory server.";
pub struct UploadCommand {
    cmd: BBaseCommand,
    // Your struct fields and methods here
}

impl BCommand for UploadCommand {
    fn cmd_str(&self) -> &str {
        &self.cmd.cmd_str
    }

    fn cmd_type(&self) -> &str {
        &BCOMMAND
    }

    fn set_cfg_arg_specified(&self, value: bool) {
        self.cmd.cfg_arg_available.set(value);
    }

    fn was_cfg_arg_specified(&self) -> bool {
        self.cmd.cfg_arg_available.get().unwrap().clone()
    }

    fn subcommand(&self) -> &clap::Command {
        &self.cmd.sub_cmd
    }

    fn is_docker_required(&self) -> bool {
        self.cmd.require_docker
    }

    fn args_required(&self) -> bool {
        self.cmd.args_required
    }

    fn execute(&self, cli: &Cli, workspace: &mut Workspace) -> Result<(), BError> {
        let config: String = self.get_config(cli, &workspace.settings().work_dir())?;
        let ctx: Vec<String> = self.get_arg_many(cli, "ctx", BCOMMAND)?;
        let args_context: IndexMap<String, String> = self.setup_context(ctx);
        let context: WsContextData = WsContextData::new(&args_context)?;
        let volumes: Vec<String> = self.get_arg_many(cli, "volume", BCOMMAND)?;
        let interactive: bool = self.get_arg_bool(cli, "interactive", BCOMMAND)?;
        let env: Vec<String> = self.get_arg_many(cli, "env", BCOMMAND)?;

        /*
         * If docker is enabled in the workspace settings then bakery will be boottraped into a docker container
         * with a bakery inside and all the baking will be done inside that docker container. Not all commands should
         * be run inside of docker and if we are already inside docker we should not try and bootstrap into a
         * second docker container.
         */
        if !workspace.settings().docker_disabled()
            && self.is_docker_required()
            && !cli.inside_docker()
        {
            return self.bootstrap(
                &self.get_cmd_line(cli, &config, None),
                cli,
                workspace,
                &volumes,
                interactive,
            );
        }

        if workspace.settings().mode() == Mode::SETUP {
            return Err(BError::ExecuteCmdInsideWorkspace(
                self.cmd.cmd_str.to_string(),
            ));
        }

        if !workspace.valid_config(config.as_str()) {
            return Err(BError::CliError(format!(
                "Unsupported build config '{}'",
                config
            )));
        }

        workspace.update_ctx(&context)?;

        let env_variables: HashMap<String, String> = self.setup_env(env);
        let upload: &WsCustomSubCmdHandler = workspace.config().upload();
        upload.run(cli, &env_variables, false, self.cmd.interactive)
    }
}

impl UploadCommand {
    pub fn new() -> Self {
        let subcmd: clap::Command = clap::Command::new(BCOMMAND)
      .about(BCOMMAND_ABOUT)
      .arg(
        clap::Arg::new("config")
            .short('c')
            .long("config")
            .help("The build config defining deploy task")
            .value_name("name")
            .required(true),
      )
      .arg(
        clap::Arg::new("volume")
            .action(clap::ArgAction::Append)
            .short('v')
            .long("docker-volume")
            .value_name("path:path")
            .help("Docker volume to mount bind when boot strapping into docker."),
      )
      .arg(
        clap::Arg::new("verbose")
            .action(clap::ArgAction::SetTrue)
            .long("verbose")
            .help("Set verbose level."),
      )
      .arg(
        clap::Arg::new("interactive")
            .short('i')
            .long("interactive")
            .value_name("interactive")
            .default_value("true")
            .value_parser(["true", "false"])
            .help("Determines whether a build inside Docker should be interactive. This can be useful to set to false when running in CI environments."),
       )
       .arg(
        clap::Arg::new("env")
            .action(clap::ArgAction::Append)
            .short('e')
            .long("env")
            .value_name("KEY=VALUE")
            .help("Extra variables to add to build env."),
      )
      .arg(
        clap::Arg::new("ctx")
            .action(clap::ArgAction::Append)
            .short('x')
            .long("context")
            .value_name("KEY=VALUE")
            .help("Adding variable to the context. Any KEY that already exists in the context will be overwriten."),
      );
        // Initialize and return a new DeployCommand instance
        UploadCommand {
            // Initialize fields if any
            cmd: BBaseCommand {
                cmd_str: String::from(BCOMMAND),
                sub_cmd: subcmd,
                interactive: true,
                require_docker: true,
                cfg_arg_available: OnceCell::new(),
                args_required: true,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use tempdir::TempDir;

    use crate::cli::*;
    use crate::commands::{BCommand, UploadCommand};
    use crate::error::BError;
    use crate::helper::Helper;
    use crate::workspace::{
        Workspace, WsBuildConfigHandler, WsBuildMetadataHandler, WsId, WsSettingsHandler,
    };

    fn helper_test_upload_subcommand(
        json_build_config: &str,
        work_dir: &PathBuf,
        logger: Box<dyn Logger>,
        system: Box<dyn System>,
        cmd_line: Vec<&str>,
    ) -> Result<(), BError> {
        let json_ws_settings: &str = r#"
        {
            "version": "6",
            "builds": {
                "supported": [
                    "default"
                ]
            },
            "workspace": {
                "configsdir": "configs",
                "includedir": "configs/include",
                "scriptsdir": "scripts"
            },
            "docker": {
                "disabled": "true"
            }
        }"#;
        let settings: WsSettingsHandler =
            WsSettingsHandler::from_str(work_dir, json_ws_settings, None)?;
        let config: WsBuildConfigHandler =
            WsBuildConfigHandler::from_str(json_build_config, &settings)?;
        let metadata: WsBuildMetadataHandler =
            WsBuildMetadataHandler::new(work_dir, &work_dir.join(PathBuf::from(".bkry")), None);
        let mut workspace: Workspace = Workspace::new(
            Some(work_dir.to_owned()),
            Some(settings),
            Some(config),
            Some(metadata),
        )?;
        let cli: Cli = Cli::new(logger, system, clap::Command::new("bakery"), Some(cmd_line));
        let cmd: UploadCommand = UploadCommand::new();
        cmd.execute(&cli, &mut workspace)
    }

    #[test]
    fn test_cmd_upload() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: &PathBuf = &temp_dir.into_path();
        let json_build_config: &str = r#"
        {
            "version": "6",
            "name": "default",
            "description": "Test Description",
            "arch": "test-arch",
            "bb": {},
            "context": [
                "ARG1=arg1",
                "ARG2=arg2",
                "ARG3=arg3"
            ],
            "upload": {
                "cmd": "$#[BKRY_SCRIPTS_DIR]/script.sh $#[ARG1] $#[ARG2] $#[ARG3]"
            }
        }
        "#;
        let mut mocked_system: MockSystem = MockSystem::new();
        mocked_system
            .expect_check_call()
            .with(mockall::predicate::eq(CallParams {
                cmd_line: Helper::cmd_line_string(&format!(
                    "{}/scripts/script.sh arg1 arg2 arg3",
                    work_dir.display()
                )),
                env: HashMap::from([(String::from("BKRY_WORKSPACE_ID"), WsId::get())]),
                shell: true,
            }))
            .once()
            .returning(|_x| Ok(()));
        mocked_system.expect_env().returning(|| HashMap::new());
        let _result: Result<(), BError> = helper_test_upload_subcommand(
            json_build_config,
            &work_dir,
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            vec!["bakery", "upload", "--config", "default"],
        );
    }

    #[test]
    fn test_cmd_upload_ctx() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: &PathBuf = &temp_dir.into_path();
        let json_build_config: &str = r#"
        {
            "version": "6",
            "name": "default",
            "description": "Test Description",
            "arch": "test-arch",
            "bb": {},
            "context": [
                "ARG1=arg1",
                "ARG2=arg2",
                "ARG3=arg3"
            ],
            "upload": {
                "cmd": "$#[BKRY_SCRIPTS_DIR]/script.sh $#[ARG1] $#[ARG2] $#[ARG3]"
            }
        }
        "#;
        let mut mocked_system: MockSystem = MockSystem::new();
        mocked_system
            .expect_check_call()
            .with(mockall::predicate::eq(CallParams {
                cmd_line: Helper::cmd_line_string(&format!(
                    "{}/scripts/script.sh arg1 arg2 arg4",
                    work_dir.display()
                )),
                env: HashMap::from([(String::from("BKRY_WORKSPACE_ID"), WsId::get())]),
                shell: true,
            }))
            .once()
            .returning(|_x| Ok(()));
        mocked_system.expect_env().returning(|| HashMap::new());
        let _result: Result<(), BError> = helper_test_upload_subcommand(
            json_build_config,
            &work_dir,
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            vec![
                "bakery",
                "upload",
                "--config",
                "default",
                "--context",
                "ARG3=arg4",
            ],
        );
    }
}
