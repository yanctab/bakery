use indexmap::{indexmap, IndexMap};
use once_cell::sync::OnceCell;
use std::collections::HashMap;

use crate::cli::Cli;
use crate::commands::{BBaseCommand, BCommand, BError};
use crate::data::{
    WsContextData, CTX_KEY_BRANCH, CTX_KEY_JOBS, CTX_KEY_PIPELINE_MODE, CTX_KEY_RESET_WS,
};
use crate::workspace::WsCustomSubCmdHandler;
use crate::workspace::{Mode, Workspace};

static BCOMMAND: &str = "sync";
static BCOMMAND_ABOUT: &str = "Synchronize and switch branches in a workspace, for example, by syncing or updating repositories using the repo tool.";
pub struct SyncCommand {
    cmd: BBaseCommand,
    // Your struct fields and methods here
}

impl BCommand for SyncCommand {
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
        let branch: String = self.get_arg_str(cli, "branch", BCOMMAND)?;
        let reset: bool = self.get_arg_flag(cli, "reset", BCOMMAND)?;
        let jobs: String = self.get_arg_str(cli, "jobs", BCOMMAND)?;
        let args_context: IndexMap<String, String> = self.setup_context(ctx);
        let mut context: WsContextData = WsContextData::new(&args_context)?;
        let interactive: bool = self.get_arg_bool(cli, "interactive", BCOMMAND)?;
        let env: Vec<String> = self.get_arg_many(cli, "env", BCOMMAND)?;
        let pipeline: bool = self.get_arg_flag(cli, "pipeline", BCOMMAND)?;

        /*
         * If Docker is enabled in the workspace settings, Bakery will be bootstrapped into
         * a Docker container where all baking operations are performed.
         * However, not all commands should run inside Docker, and if we're already inside
         * a container, we must avoid bootstrapping into another one.
         */
        if !workspace.settings().docker_disabled()
            && self.is_docker_required()
            && !cli.inside_docker()
        {
            return self.bootstrap(
                &self.get_cmd_line(cli, &config, None),
                cli,
                workspace,
                &vec![],
                interactive,
            );
        }

        if workspace.settings().mode() == Mode::SETUP {
            return Err(BError::ExecuteCmdInsideWorkspace(
                self.cmd.cmd_str.to_string(),
            ));
        }

        if branch != String::from("NA") {
            context.update(&indexmap! {
                CTX_KEY_BRANCH.to_string() => branch,
            });
        }

        context.update(&indexmap! {
            CTX_KEY_RESET_WS.to_string() => reset.to_string(),
            CTX_KEY_JOBS.to_string() => jobs,
            CTX_KEY_PIPELINE_MODE.to_string() => pipeline.to_string()
        });

        if !workspace.valid_config(config.as_str()) {
            return Err(BError::CliError(format!(
                "Unsupported build config '{}'",
                config
            )));
        }

        workspace.update_ctx(&context)?;

        let env_variables: HashMap<String, String> = self.setup_env(env);
        let sync: &WsCustomSubCmdHandler = workspace.config().sync();
        sync.run(cli, &env_variables, false, self.cmd.interactive)
    }
}

impl SyncCommand {
    pub fn new() -> Self {
        let subcmd: clap::Command = clap::Command::new(BCOMMAND)
      .about(BCOMMAND_ABOUT)
      .arg(
        clap::Arg::new("config")
            .short('c')
            .long("config")
            .help("The build config defining deploy task")
            .value_name("name")
            .required(false),
      )
      .arg(
        clap::Arg::new("verbose")
            .action(clap::ArgAction::SetTrue)
            .long("verbose")
            .help("Set verbose level."),
      )
      .arg(
        clap::Arg::new("jobs")
            .action(clap::ArgAction::Append)
            .short('j')
            .long("jobs")
            .value_name("jobs")
            .default_value("16")
            .help("Specify the number of jobs used when setting up, it will be exposed as context/environment variable $#[BKRY_JOBS]"),
      )
      .arg(
        clap::Arg::new("reset")
            .action(clap::ArgAction::SetTrue)
            .long("reset")
            .help("Reset workspace, all changes will be lost, it will be exposed as context/environment variable $#[BKRY_RESET_WS]"),
      )
      .arg(
        clap::Arg::new("branch")
            .action(clap::ArgAction::Append)
            .short('b')
            .long("branch")
            .value_name("branch")
            .default_value("NA")
            .help("The branch to setup will be exposed as an context/environment variable $#[BKRY_BRANCH]"),
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
        clap::Arg::new("pipeline")
            .action(clap::ArgAction::SetTrue)
            .long("pipeline")
            .help("Run sync in pipeline mode, can be used to limit functionality that can be in conflict with the pipeline")
      )
      .arg(
        clap::Arg::new("ctx")
            .action(clap::ArgAction::Append)
            .short('x')
            .long("context")
            .value_name("KEY=VALUE")
            .help("Adding variable to the context. Any KEY that already exists in the context will be overwriten."),
      );
        // Initialize and return a new SetupCommand instance
        SyncCommand {
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
    use crate::commands::{BCommand, SyncCommand};
    use crate::constants::BkryConstants;
    use crate::error::BError;
    use crate::executers::DockerImage;
    use crate::helper::Helper;
    use crate::workspace::{
        Workspace, WsBuildConfigHandler, WsBuildMetadataHandler, WsId, WsSettingsHandler,
    };

    fn helper_test_sync_subcommand_custom_ws(
        json_ws_settings: &str,
        json_build_config: &str,
        work_dir: &PathBuf,
        logger: Box<dyn Logger>,
        system: Box<dyn System>,
        cmd_line: Vec<&str>,
    ) -> Result<(), BError> {
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
        let cmd: SyncCommand = SyncCommand::new();
        cmd.execute(&cli, &mut workspace)
    }

    fn helper_test_sync_subcommand(
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
        let cmd: SyncCommand = SyncCommand::new();
        cmd.execute(&cli, &mut workspace)
    }

    #[test]
    fn test_cmd_sync() {
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
            "sync": {
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
        let _result: Result<(), BError> = helper_test_sync_subcommand(
            json_build_config,
            &work_dir,
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            vec!["bakery", "sync", "--config", "default"],
        );
    }

    #[test]
    fn test_cmd_sync_ctx() {
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
            "sync": {
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
        let _result: Result<(), BError> = helper_test_sync_subcommand(
            json_build_config,
            &work_dir,
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            vec![
                "bakery",
                "sync",
                "--config",
                "default",
                "--context",
                "ARG3=arg4",
            ],
        );
    }

    #[test]
    fn test_cmd_sync_default_branch() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: &PathBuf = &temp_dir.into_path();
        let json_build_config: &str = r#"
        {
            "version": "6",
            "name": "default",
            "description": "Test Description",
            "arch": "test-arch",
            "context": [
                "BKRY_BRANCH=new-default-branch"
            ],
            "sync": {
                "cmd": "$#[BKRY_SCRIPTS_DIR]/script.sh $#[BKRY_BRANCH]"
            }
        }
        "#;
        let mut mocked_system: MockSystem = MockSystem::new();
        mocked_system
            .expect_check_call()
            .with(mockall::predicate::eq(CallParams {
                cmd_line: Helper::cmd_line_string(&format!(
                    "{}/scripts/script.sh new-default-branch",
                    work_dir.display()
                )),
                env: HashMap::from([(String::from("BKRY_WORKSPACE_ID"), WsId::get())]),
                shell: true,
            }))
            .once()
            .returning(|_x| Ok(()));
        mocked_system.expect_env().returning(|| HashMap::new());
        let _result: Result<(), BError> = helper_test_sync_subcommand(
            json_build_config,
            &work_dir,
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            vec!["bakery", "sync", "--config", "default"],
        );
    }

    #[test]
    fn test_cmd_sync_branch() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: &PathBuf = &temp_dir.into_path();
        let json_build_config: &str = r#"
        {
            "version": "6",
            "name": "default",
            "description": "Test Description",
            "arch": "test-arch",
            "sync": {
                "cmd": "$#[BKRY_SCRIPTS_DIR]/script.sh $#[BKRY_BRANCH]"
            }
        }
        "#;
        let mut mocked_system: MockSystem = MockSystem::new();
        mocked_system
            .expect_check_call()
            .with(mockall::predicate::eq(CallParams {
                cmd_line: Helper::cmd_line_string(&format!(
                    "{}/scripts/script.sh test-branch",
                    work_dir.display()
                )),
                env: HashMap::from([(String::from("BKRY_WORKSPACE_ID"), WsId::get())]),
                shell: true,
            }))
            .once()
            .returning(|_x| Ok(()));
        mocked_system.expect_env().returning(|| HashMap::new());
        let _result: Result<(), BError> = helper_test_sync_subcommand(
            json_build_config,
            &work_dir,
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            vec!["bakery", "sync", "-c", "default", "-b", "test-branch"],
        );
    }

    #[test]
    fn test_cmd_sync_reset() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: &PathBuf = &temp_dir.into_path();
        let json_build_config: &str = r#"
        {
            "version": "6",
            "name": "default",
            "description": "Test Description",
            "arch": "test-arch",
            "sync": {
                "cmd": "$#[BKRY_SCRIPTS_DIR]/script.sh $#[BKRY_BRANCH] $#[BKRY_RESET_WS]"
            }
        }
        "#;
        let mut mocked_system: MockSystem = MockSystem::new();
        mocked_system
            .expect_check_call()
            .with(mockall::predicate::eq(CallParams {
                cmd_line: Helper::cmd_line_string(&format!(
                    "{}/scripts/script.sh test-branch true",
                    work_dir.display()
                )),
                env: HashMap::from([(String::from("BKRY_WORKSPACE_ID"), WsId::get())]),
                shell: true,
            }))
            .once()
            .returning(|_x| Ok(()));
        mocked_system.expect_env().returning(|| HashMap::new());
        let _result: Result<(), BError> = helper_test_sync_subcommand(
            json_build_config,
            &work_dir,
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            vec![
                "bakery",
                "sync",
                "-c",
                "default",
                "-b",
                "test-branch",
                "--reset",
            ],
        );
    }

    #[test]
    fn test_cmd_sync_jobs() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: &PathBuf = &temp_dir.into_path();
        let json_build_config: &str = r#"
        {
            "version": "6",
            "name": "default",
            "description": "Test Description",
            "arch": "test-arch",
            "sync": {
                "cmd": "$#[BKRY_SCRIPTS_DIR]/script.sh $#[BKRY_BRANCH] $#[BKRY_JOBS]"
            }
        }
        "#;
        let mut mocked_system: MockSystem = MockSystem::new();
        mocked_system
            .expect_check_call()
            .with(mockall::predicate::eq(CallParams {
                cmd_line: Helper::cmd_line_string(&format!(
                    "{}/scripts/script.sh test-branch 18",
                    work_dir.display()
                )),
                env: HashMap::from([(String::from("BKRY_WORKSPACE_ID"), WsId::get())]),
                shell: true,
            }))
            .once()
            .returning(|_x| Ok(()));
        mocked_system.expect_env().returning(|| HashMap::new());
        let _result: Result<(), BError> = helper_test_sync_subcommand(
            json_build_config,
            &work_dir,
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            vec![
                "bakery",
                "sync",
                "-c",
                "default",
                "-b",
                "test-branch",
                "--jobs",
                "18",
            ],
        );
    }

    #[test]
    fn test_cmd_sync_interactive() {
        let json_ws_settings: &str = r#"
        {
            "version": "6",
            "builds": {
                "supported": [
                    "default"
                ]
            }
        }"#;
        let json_build_config: &str = r#"
        {
            "version": "6",
            "name": "default",
            "description": "Test Description",
            "arch": "test-arch",
            "context": [
                "ARG1=arg1",
                "ARG2=arg2",
                "ARG3=arg3"
            ],
            "sync": {
                "cmd": "$#[BKRY_SCRIPTS_DIR]/script.sh $#[ARG1] $#[ARG2] $#[ARG3]"
            }
        }
        "#;
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = temp_dir.into_path();
        let docker_image: DockerImage = DockerImage::new(&format!(
            "{}/{}:{}",
            BkryConstants::DOCKER_REGISTRY,
            BkryConstants::DOCKER_IMAGE,
            BkryConstants::DOCKER_TAG
        ))
        .expect("Invalid docker image format");
        let mut mocked_system: MockSystem = MockSystem::new();
        mocked_system.expect_inside_docker().returning(|| false);
        mocked_system
            .expect_check_call()
            .with(mockall::predicate::eq(CallParams {
                cmd_line: Helper::docker_pull_string(&docker_image),
                env: HashMap::new(),
                shell: true,
            }))
            .once()
            .returning(|_x| Ok(()));
        mocked_system
            .expect_exists()
            .with(mockall::predicate::always())
            .times(1..10)
            .returning(|_x| true);
        mocked_system
            .expect_check_call()
            .with(mockall::predicate::eq(CallParams {
                cmd_line: Helper::docker_bootstrap_string(
                    false,
                    &vec![],
                    &vec![],
                    &work_dir.clone(),
                    &work_dir,
                    &docker_image,
                    &vec![
                        String::from("bakery"),
                        String::from("sync"),
                        String::from("--config"),
                        String::from("default"),
                        String::from("--interactive=false"),
                    ],
                ),
                env: HashMap::new(),
                shell: true,
            }))
            .once()
            .returning(|_x| Ok(()));
        mocked_system.expect_env().returning(|| HashMap::new());
        let _result: Result<(), BError> = helper_test_sync_subcommand_custom_ws(
            json_ws_settings,
            json_build_config,
            &work_dir,
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            vec![
                "bakery",
                "sync",
                "--config",
                "default",
                "--interactive=false",
            ],
        );
    }

    /*
     * In this test we are adding volumes as docker args to the workspace.json.
     * One is valid and one is invalid and then we are making sure we print
     * the warning about the invalid volume and that we are calling the expected
     * docker call.
     */
    #[test]
    fn test_cmd_sync_docker_volumes() {
        let json_ws_settings: &str = r#"
        {
            "version": "6",
            "builds": {
                "supported": [
                    "default"
                ]
            },
            "docker": {
                "args": [
                    "-v /test/testdir2:/test/testdir2",
                    "-v :"
                ]
            }
        }"#;
        let json_build_config: &str = r#"
        {
            "version": "6",
            "name": "default",
            "description": "Test Description",
            "arch": "test-arch",
            "context": [
                "BKRY_BRANCH=test"
            ],
            "setup": {
                "cmd": "$#[BKRY_SCRIPTS_DIR]/script.sh $#[BKRY_BRANCH]"
            }
        }
        "#;
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = temp_dir.into_path();
        let docker_image: DockerImage = DockerImage::new(&format!(
            "{}/{}:{}",
            BkryConstants::DOCKER_REGISTRY,
            BkryConstants::DOCKER_IMAGE,
            BkryConstants::DOCKER_TAG
        ))
        .expect("Invalid docker image format");
        let mut mocked_system: MockSystem = MockSystem::new();
        let mut mocked_logger: MockLogger = MockLogger::new();
        mocked_logger
            .expect_info()
            .with(mockall::predicate::always())
            .times(1..10)
            .returning(|_x| ());
        mocked_logger
            .expect_warn()
            .with(mockall::predicate::eq(
                "invalid docker volume '-v :'".to_string(),
            ))
            .once()
            .returning(|_x| ());
        mocked_system.expect_inside_docker().returning(|| false);
        mocked_system
            .expect_check_call()
            .with(mockall::predicate::eq(CallParams {
                cmd_line: Helper::docker_pull_string(&docker_image),
                env: HashMap::new(),
                shell: true,
            }))
            .once()
            .returning(|_x| Ok(()));
        mocked_system
            .expect_exists()
            .with(mockall::predicate::always())
            .times(1..11)
            .returning(|_x| true);
        mocked_system
            .expect_check_call()
            .with(mockall::predicate::eq(CallParams {
                cmd_line: Helper::docker_bootstrap_string(
                    true,
                    &vec![String::from("-v /test/testdir2:/test/testdir2")],
                    &vec![],
                    &work_dir.clone(),
                    &work_dir,
                    &docker_image,
                    &vec![
                        String::from("bakery"),
                        String::from("sync"),
                        String::from("--config"),
                        String::from("default"),
                    ],
                ),
                env: HashMap::new(),
                shell: true,
            }))
            .once()
            .returning(|_x| Ok(()));
        mocked_system
            .expect_init_env_file()
            .returning(|_x, _y| Ok(HashMap::new()));
        mocked_system.expect_env().returning(|| HashMap::new());
        let _result: Result<(), BError> = helper_test_sync_subcommand_custom_ws(
            json_ws_settings,
            json_build_config,
            &work_dir,
            Box::new(mocked_logger),
            Box::new(mocked_system),
            vec!["bakery", "sync", "--config", "default"],
        );
    }

    #[test]
    fn test_cmd_sync_pipeline() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: &PathBuf = &temp_dir.into_path();
        let json_build_config: &str = r#"
        {
            "version": "6",
            "name": "default",
            "description": "Test Description",
            "arch": "test-arch",
            "sync": {
                "cmd": "$#[BKRY_SCRIPTS_DIR]/script.sh $#[BKRY_PIPELINE_MODE]"
            }
        }
        "#;
        let mut mocked_system: MockSystem = MockSystem::new();
        mocked_system
            .expect_check_call()
            .with(mockall::predicate::eq(CallParams {
                cmd_line: Helper::cmd_line_string(&format!(
                    "{}/scripts/script.sh true",
                    work_dir.display()
                )),
                env: HashMap::from([(String::from("BKRY_WORKSPACE_ID"), WsId::get())]),
                shell: true,
            }))
            .once()
            .returning(|_x| Ok(()));
        mocked_system.expect_env().returning(|| HashMap::new());
        let _result: Result<(), BError> = helper_test_sync_subcommand(
            json_build_config,
            &work_dir,
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            vec!["bakery", "sync", "-c", "default", "--pipeline"],
        );
    }
}
