use indexmap::IndexMap;
use once_cell::sync::OnceCell;
use std::collections::HashMap;

use crate::cli::Cli;
use crate::commands::{BBaseCommand, BCommand};
use crate::data::WsContextData;
use crate::error::BError;
use crate::workspace::{Mode, Workspace, WsTaskHandler};

static BCOMMAND: &str = "clean";
static BCOMMAND_ABOUT: &str = "Clean one or all tasks defined in a build config.";
pub struct CleanCommand {
    cmd: BBaseCommand,
    // Your struct fields and methods here
}

impl BCommand for CleanCommand {
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
        let tasks: Vec<String> = self.get_arg_many(cli, "tasks", BCOMMAND)?;
        let args_context: IndexMap<String, String> = self.setup_context(ctx);
        let context: WsContextData = WsContextData::new(&args_context)?;
        let interactive: bool = self.get_arg_bool(cli, "interactive", BCOMMAND)?;
        let env: Vec<String> = self.get_arg_many(cli, "env", BCOMMAND)?;

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

        if !workspace.valid_config(config.as_str()) {
            return Err(BError::CliError(format!(
                "Unsupported build config '{}'",
                config
            )));
        }

        /*
         * We will update the context with the variables from the cli
         * and then expand the context variables in the config
         */
        workspace.update_ctx(&context)?;

        let env_variables: HashMap<String, String> = self.setup_env(env);

        if tasks.len() > 1 {
            // More then one task was specified on the command line
            for t_name in tasks {
                let task: &WsTaskHandler = workspace.config().task(&t_name)?;
                task.clean(cli, &workspace.config().build_data(), &env_variables, true)?;
            }
        } else {
            // One task was specified on the command line or default was used
            let task: &String = tasks.get(0).unwrap();
            if task == "all" {
                // The alias "all" was specified on the command line or it none was specified and "all" was used
                for (_t_name, task) in workspace.config().tasks() {
                    task.clean(cli, &workspace.config().build_data(), &env_variables, false)?;
                }
            } else {
                // One task was specified on the command line
                let task: &WsTaskHandler = workspace.config().task(tasks.get(0).unwrap())?;
                task.clean(cli, &workspace.config().build_data(), &env_variables, true)?;
            }
        }
        Ok(())
    }
}

impl CleanCommand {
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
            clap::Arg::new("tasks")
                .short('t')
                .long("tasks")
                .value_name("tasks")
                .default_value("all")
                .value_delimiter(',')
                .help("The task(s) to clean."),
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
            clap::Arg::new("interactive")
                .short('i')
                .long("interactive")
                .value_name("interactive")
                .default_value("true")
                .value_parser(["true", "false"])
                .help("Determines whether a build inside Docker should be interactive. This can be useful to set to false when running in CI environments."),
        )
          .arg(
            clap::Arg::new("ctx")
                .action(clap::ArgAction::Append)
                .short('x')
                .long("context")
                .value_name("KEY=VALUE")
                .help("Adding variable to the context. Any KEY that already exists in the context will be overwriten."),
          );
        // Initialize and return a new BuildCommand instance
        CleanCommand {
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
    use crate::commands::{BCommand, CleanCommand};
    use crate::constants::BkryConstants;
    use crate::error::BError;
    use crate::executers::DockerImage;
    use crate::helper::Helper;
    use crate::workspace::{
        Workspace, WsBuildConfigHandler, WsBuildMetadataHandler, WsId, WsSettingsHandler,
    };

    fn helper_test_clean_subcommand(
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
        let cmd: CleanCommand = CleanCommand::new();
        cmd.execute(&cli, &mut workspace)
    }

    #[test]
    fn test_cmd_clean_nonbitbake() {
        let json_ws_settings: &str = r#"
        {
            "version": "6",
            "builds": {
                "supported": [
                    "default"
                ]
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
            "bb": {},
            "tasks": {
                "task-name": {
                    "index": "1",
                    "name": "task-name",
                    "type": "non-bitbake",
                    "builddir": "test-dir",
                    "build": "test.sh",
                    "clean": "rm -rf dir-to-delete"
                }
            }
        }
        "#;
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = temp_dir.into_path();
        let build_dir: PathBuf = work_dir.join("test-dir");
        let mut mocked_system: MockSystem = MockSystem::new();
        mocked_system
            .expect_check_call()
            .with(mockall::predicate::eq(CallParams {
                cmd_line: vec![
                    "cd",
                    &build_dir.to_string_lossy().to_string(),
                    "&&",
                    "rm",
                    "-rf",
                    "dir-to-delete",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect(),
                env: HashMap::from([(String::from("BKRY_WORKSPACE_ID"), WsId::get())]),
                shell: true,
            }))
            .once()
            .returning(|_x| Ok(()));
        let _result: Result<(), BError> = helper_test_clean_subcommand(
            json_ws_settings,
            json_build_config,
            &work_dir,
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            vec!["bakery", "clean", "--config", "default"],
        );
    }

    #[test]
    fn test_cmd_clean_interactive() {
        let json_ws_settings: &str = r#"
        {
            "version": "5",
            "builds": {
                "supported": [
                    "default"
                ]
            }
        }"#;
        let json_build_config: &str = r#"
        {
            "version": "5",
            "name": "default",
            "description": "Test Description",
            "arch": "test-arch",
            "tasks": {
                "task-name": {
                    "index": "1",
                    "type": "non-bitbake",
                    "name": "task-name",
                    "builddir": "test-dir",
                    "build": "test.sh",
                    "clean": "rm -rf dir-to-delete"
                }
            }
        }
        "#;
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = temp_dir.into_path();
        let mut mocked_system: MockSystem = MockSystem::new();
        let docker_image: DockerImage = DockerImage::new(&format!(
            "{}/{}:{}",
            BkryConstants::DOCKER_REGISTRY,
            BkryConstants::DOCKER_IMAGE,
            BkryConstants::DOCKER_TAG
        ))
        .expect("Invalid docker image format");
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
                        String::from("clean"),
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
        let _result: Result<(), BError> = helper_test_clean_subcommand(
            json_ws_settings,
            json_build_config,
            &work_dir,
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            vec![
                "bakery",
                "clean",
                "--config",
                "default",
                "--interactive=false",
            ],
        );
    }
}
