use indexmap::{indexmap, IndexMap};
use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::cli::Cli;
use crate::commands::{BBaseCommand, BCommand, BError, Variant};
use crate::constants::BkryConstants;
use crate::data::{WsContextData, CTX_KEY_BRANCH, CTX_KEY_JOBS, CTX_KEY_PIPELINE_MODE};
use crate::workspace::Workspace;
use crate::workspace::WsCustomSubCmdHandler;

static BCOMMAND: &str = "setup";
static BCOMMAND_ABOUT: &str = "Set up the workspace, e.g., initialize git submodules.";
pub struct SetupCommand {
    cmd: BBaseCommand,
    // Your struct fields and methods here
}

impl BCommand for SetupCommand {
    fn get_config_name(&self, cli: &Cli, _workspace_dir: &PathBuf) -> String {
        if let Some(sub_matches) = cli.get_args().subcommand_matches(BCOMMAND) {
            if sub_matches.contains_id("config") {
                if let Some(value) = sub_matches.get_one::<String>("config") {
                    return value.clone();
                }
            }
        }

        return String::from("default");
    }

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
        let config: String = self.get_arg_str(cli, "config", BCOMMAND)?;
        let branch: String = self.get_arg_str(cli, "branch", BCOMMAND)?;
        let ctx: Vec<String> = self.get_arg_many(cli, "ctx", BCOMMAND)?;
        let interactive: bool = self.get_arg_bool(cli, "interactive", BCOMMAND)?;
        let force: bool = self.get_arg_flag(cli, "force", BCOMMAND)?;
        let args_context: IndexMap<String, String> = self.setup_context(ctx);
        let mut context: WsContextData = WsContextData::new(&args_context)?;
        let jobs: String = self.get_arg_str(cli, "jobs", BCOMMAND)?;
        let env: Vec<String> = self.get_arg_many(cli, "env", BCOMMAND)?;
        let variant: Variant = self.get_arg_variant(cli, "variant", BCOMMAND)?;
        let mut metadata: bool = self.get_arg_bool(cli, "metadata", BCOMMAND)?;
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
            return self.bootstrap(&cli.get_cmd_line(), cli, workspace, &vec![], interactive);
        }

        if !force {
            match cli.is_ws_empty(&workspace.settings().work_dir()) {
                Ok(is_empty) => {
                    let ws_dir: PathBuf = workspace.settings().work_dir();
                    if ws_dir.join(BkryConstants::WS_SETTINGS).exists() {
                        cli.debug(format!(
                        "Workspace '{}' is not empty and contains a '{}'. You can use --force to enforce the setup.",
                        ws_dir.to_str().unwrap_or_default().to_string(),
                        BkryConstants::WS_SETTINGS
                    ));
                    } else if !is_empty {
                        return Err(BError::WorkspaceNotEmpty(
                            ws_dir.to_str().unwrap_or_default().to_string(),
                        ));
                    } else {
                        cli.debug(format!(
                            "Workspace '{}' is empty",
                            ws_dir.to_str().unwrap_or_default().to_string()
                        ));
                    }
                }
                Err(e) => {
                    return Err(BError::IOError(format!(
                        "Failed to check for empty workspace, {}",
                        e.to_string()
                    )));
                }
            }
        }

        if branch != String::from("NA") {
            context.update(&indexmap! {
                CTX_KEY_BRANCH.to_string() => branch,
            });
        }

        context.update(&indexmap! {
            CTX_KEY_PIPELINE_MODE.to_string() => pipeline.to_string(),
            CTX_KEY_JOBS.to_string() => jobs,
        });

        if !workspace.valid_config(config.as_str()) {
            return Err(BError::CliError(format!(
                "Unsupported build config '{}'",
                config
            )));
        }

        workspace.update_ctx(&context)?;

        if pipeline {
            /*
             * When in pipeline mode we should not use the build metadata to lock
             * the workspace.
             */
            metadata = false;
            cli.info(format!("Pipeline mode: {}", pipeline));
        }

        if metadata {
            workspace.metadata().write(
                config.as_str(),
                workspace.config().build_data().bitbake().machine(),
                workspace.config().build_data().bitbake().distro(),
                &variant,
            )?;
        }

        /*
        cli.info(format!("Build Config: {}", config));
        cli.info(format!(
            "Build Machine: {}",
            workspace.config().build_data().bitbake().machine()
        ));
        cli.info(format!(
            "Build Distro: {}",
            workspace.config().build_data().bitbake().distro()
        ));
        cli.info(format!("Build Variant: {}", variant));
        */

        let env_variables: HashMap<String, String> = self.setup_env(env);

        let setup: &WsCustomSubCmdHandler = workspace.config().setup();
        setup.run(cli, &cli.env(), false, self.cmd.interactive)
    }
}

impl SetupCommand {
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
                clap::Arg::new("pipeline")
                    .action(clap::ArgAction::SetTrue)
                    .long("pipeline")
                    .help("Run setup in pipeline mode, can be used to limit functionality that can get be conflict with the pipeline")
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
                clap::Arg::new("verbose")
                    .action(clap::ArgAction::SetTrue)
                    .long("verbose")
                    .help("Set verbose level."),
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
                clap::Arg::new("metadata")
                    .short('m')
                    .long("meta-data")
                    .value_name("metadata")
                    .default_value("true")
                    .value_parser(["true", "false"])
                    .help("Determines whether workspace build metadata, should be tracked under ~/.bkry."),
            )
            .arg(
                clap::Arg::new("force")
                    .action(clap::ArgAction::SetTrue)
                    .long("force")
                    .help("Run the setup regardless of whether the workspace is empty or not. The default is false, which will result in an error if setup is executed in a non-empty workspace.")
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
                clap::Arg::new("variant")
                    .short('a')
                    .long("variant")
                    .value_name("variant")
                    .default_value("dev")
                    .value_parser(["dev", "test", "release"])
                    .default_value("dev")
                    .help("Specify the variant of the build it can be one of release, dev or test. Will be available as a context variable BKRY_BUILD_VARIANT"),
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
        SetupCommand {
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
    use crate::commands::{BCommand, SetupCommand};
    use crate::constants::BkryConstants;
    use crate::error::BError;
    use crate::executers::DockerImage;
    use crate::helper::Helper;
    use crate::workspace::{
        Workspace, WsBuildConfigHandler, WsBuildMetadataHandler, WsId, WsSettingsHandler,
    };

    fn helper_test_setup_subcommand_custom_ws(
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
        let cmd: SetupCommand = SetupCommand::new();
        cmd.execute(&cli, &mut workspace)
    }

    fn helper_test_setup_subcommand(
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
        let cmd: SetupCommand = SetupCommand::new();
        cmd.execute(&cli, &mut workspace)
    }

    #[test]
    fn test_cmd_setup() {
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
                "ARG1=arg1",
                "ARG2=arg2",
                "ARG3=arg3"
            ],
            "setup": {
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
        mocked_system
            .expect_is_directory_empty()
            .once()
            .returning(|_x| Ok(true));
        let _result: Result<(), BError> = helper_test_setup_subcommand(
            json_build_config,
            &work_dir,
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            vec![
                "bakery",
                "setup",
                "--config",
                "default",
                "--meta-data=false",
            ],
        );
    }

    #[test]
    fn test_cmd_setup_ctx() {
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
                "ARG1=arg1",
                "ARG2=arg2",
                "ARG3=arg3"
            ],
            "setup": {
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
        mocked_system
            .expect_is_directory_empty()
            .once()
            .returning(|_x| Ok(true));
        let _result: Result<(), BError> = helper_test_setup_subcommand(
            json_build_config,
            &work_dir,
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            vec!["bakery", "setup", "-c", "default", "--context", "ARG3=arg4"],
        );
    }

    #[test]
    fn test_cmd_setup_default_branch() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: &PathBuf = &temp_dir.into_path();
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
        let json_build_config: &str = r#"
        {
            "version": "6",
            "name": "default",
            "description": "Test Description",
            "arch": "test-arch",
            "context": [
                "BKRY_BRANCH=new-default-branch"
            ],
            "setup": {
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
        mocked_system
            .expect_is_directory_empty()
            .once()
            .returning(|_x| Ok(true));
        let settings: WsSettingsHandler =
            WsSettingsHandler::from_str(work_dir, json_ws_settings, None)
                .expect("Failed to parse settings");
        let config: WsBuildConfigHandler =
            WsBuildConfigHandler::from_str(json_build_config, &settings)
                .expect("Failed to parse build config");
        let metadata: WsBuildMetadataHandler =
            WsBuildMetadataHandler::new(work_dir, &work_dir.join(PathBuf::from(".bkry")), None);
        let mut workspace: Workspace = Workspace::new(
            Some(work_dir.to_owned()),
            Some(settings),
            Some(config),
            Some(metadata),
        )
        .expect("Failed to setup workspace");
        let cli: Cli = Cli::new(
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            clap::Command::new("bakery"),
            Some(vec!["bakery", "setup", "-c", "default"]),
        );
        let cmd: SetupCommand = SetupCommand::new();
        let _result: Result<(), BError> = cmd.execute(&cli, &mut workspace);
    }

    #[test]
    fn test_cmd_setup_branch() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: &PathBuf = &temp_dir.into_path();
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
        mocked_system
            .expect_is_directory_empty()
            .once()
            .returning(|_x| Ok(true));
        let settings: WsSettingsHandler =
            WsSettingsHandler::from_str(work_dir, json_ws_settings, None)
                .expect("Failed to parse settings");
        let config: WsBuildConfigHandler =
            WsBuildConfigHandler::from_str(json_build_config, &settings)
                .expect("Failed to parse build config");
        let metadata: WsBuildMetadataHandler =
            WsBuildMetadataHandler::new(work_dir, &work_dir.join(PathBuf::from(".bkry")), None);
        let mut workspace: Workspace = Workspace::new(
            Some(work_dir.to_owned()),
            Some(settings),
            Some(config),
            Some(metadata),
        )
        .expect("Failed to setup workspace");
        let cli: Cli = Cli::new(
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            clap::Command::new("bakery"),
            Some(vec![
                "bakery",
                "setup",
                "-c",
                "default",
                "-b",
                "test-branch",
                "--jobs",
                "18",
            ]),
        );
        let cmd: SetupCommand = SetupCommand::new();
        let _result: Result<(), BError> = cmd.execute(&cli, &mut workspace);
    }

    #[test]
    fn test_cmd_setup_interactive() {
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
                        String::from("setup"),
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
        let _result: Result<(), BError> = helper_test_setup_subcommand_custom_ws(
            json_ws_settings,
            json_build_config,
            &work_dir,
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            vec![
                "bakery",
                "setup",
                "--config",
                "default",
                "--interactive=false",
            ],
        );
    }

    #[test]
    fn test_cmd_setup_docker_volumes() {
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
                        String::from("setup"),
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
        let _result: Result<(), BError> = helper_test_setup_subcommand_custom_ws(
            json_ws_settings,
            json_build_config,
            &work_dir,
            Box::new(mocked_logger),
            Box::new(mocked_system),
            vec!["bakery", "setup", "--config", "default"],
        );
    }

    #[test]
    fn test_cmd_setup_pipeline() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: &PathBuf = &temp_dir.into_path();
        let json_build_config: &str = r#"
        {
            "version": "6",
            "name": "default",
            "description": "Test Description",
            "arch": "test-arch",
            "setup": {
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
        mocked_system
            .expect_is_directory_empty()
            .once()
            .returning(|_x| Ok(true));
        let _result: Result<(), BError> = helper_test_setup_subcommand(
            json_build_config,
            &work_dir,
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            vec!["bakery", "setup", "-c", "default", "--pipeline"],
        );
    }
}
