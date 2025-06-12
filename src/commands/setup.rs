use indexmap::{indexmap, IndexMap};

use crate::cli::Cli;
use crate::commands::{BBaseCommand, BCommand, BError};
use crate::data::{WsContextData, CTX_KEY_BRANCH};
use crate::workspace::Workspace;
use crate::workspace::WsCustomSubCmdHandler;

static BCOMMAND: &str = "setup";
static BCOMMAND_ABOUT: &str = "Set up the workspace, e.g., initialize git submodules.";
pub struct SetupCommand {
    cmd: BBaseCommand,
    // Your struct fields and methods here
}

impl BCommand for SetupCommand {
    fn get_config_name(&self, cli: &Cli) -> String {
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

    fn subcommand(&self) -> &clap::Command {
        &self.cmd.sub_cmd
    }

    fn is_docker_required(&self) -> bool {
        self.cmd.require_docker
    }

    fn execute(&self, cli: &Cli, workspace: &mut Workspace) -> Result<(), BError> {
        let config: String = self.get_arg_str(cli, "config", BCOMMAND)?;
        let branch: String = self.get_arg_str(cli, "branch", BCOMMAND)?;
        let ctx: Vec<String> = self.get_arg_many(cli, "ctx", BCOMMAND)?;
        let args_context: IndexMap<String, String> = self.setup_context(ctx);
        let mut context: WsContextData = WsContextData::new(&args_context)?;

        match cli.is_ws_empty(&workspace.settings().work_dir()) {
            Ok(is_empty) => {
                let ws_dir: String = workspace
                    .settings()
                    .work_dir()
                    .to_str()
                    .unwrap_or_default()
                    .to_string();
                if !is_empty {
                    return Err(BError::WorkspaceNotEmpty(ws_dir));
                } else {
                    cli.debug(format!("Workspace '{}' is empty", ws_dir));
                }
            }
            Err(e) => {
                return Err(BError::IOError(format!(
                    "Failed to check for empty workspace, {}",
                    e.to_string()
                )));
            }
        }

        if branch != String::from("NA") {
            context.update(&indexmap! {
                CTX_KEY_BRANCH.to_string() => branch,
            });
        }

        if !workspace.valid_config(config.as_str()) {
            return Err(BError::CliError(format!(
                "Unsupported build config '{}'",
                config
            )));
        }

        workspace.update_ctx(&context)?;

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
        clap::Arg::new("branch")
            .action(clap::ArgAction::Append)
            .short('b')
            .long("branch")
            .value_name("branch")
            .default_value("NA")
            .help("The branch to setup will be exposed as an context/environment variable $#[BKRY_BRANCH]"),
      )
      .arg(
        clap::Arg::new("verbose")
            .action(clap::ArgAction::SetTrue)
            .long("verbose")
            .help("Set verbose level."),
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
                require_docker: false,
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
    use crate::error::BError;
    use crate::workspace::{Workspace, WsBuildConfigHandler, WsSettingsHandler};

    #[test]
    fn test_cmd_setup() {
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
            }
        }"#;
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
            "setup": {
                "cmd": "$#[BKRY_SCRIPTS_DIR]/script.sh $#[ARG1] $#[ARG2] $#[ARG3]"
            }
        }
        "#;
        let mut mocked_system: MockSystem = MockSystem::new();
        mocked_system
            .expect_check_call()
            .with(mockall::predicate::eq(CallParams {
                cmd_line: vec![
                    &format!("{}/scripts/script.sh", work_dir.display()),
                    "arg1",
                    "arg2",
                    "arg3",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect(),
                env: HashMap::new(),
                shell: true,
            }))
            .once()
            .returning(|_x| Ok(()));
        mocked_system.expect_env().returning(|| HashMap::new());
        mocked_system
            .expect_is_directory_empty()
            .once()
            .returning(|_x| Ok(true));
        let settings: WsSettingsHandler = WsSettingsHandler::from_str(work_dir, json_ws_settings)
            .expect("Failed to parse settings");
        let config: WsBuildConfigHandler =
            WsBuildConfigHandler::from_str(json_build_config, &settings)
                .expect("Failed to parse build config");
        let mut workspace: Workspace =
            Workspace::new(Some(work_dir.to_owned()), Some(settings), Some(config))
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
    fn test_cmd_setup_ctx() {
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
            }
        }"#;
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
            "setup": {
                "cmd": "$#[BKRY_SCRIPTS_DIR]/script.sh $#[ARG1] $#[ARG2] $#[ARG3]"
            }
        }
        "#;
        let mut mocked_system: MockSystem = MockSystem::new();
        mocked_system
            .expect_check_call()
            .with(mockall::predicate::eq(CallParams {
                cmd_line: vec![
                    &format!("{}/scripts/script.sh", work_dir.display()),
                    "arg1",
                    "arg2",
                    "arg4",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect(),
                env: HashMap::new(),
                shell: true,
            }))
            .once()
            .returning(|_x| Ok(()));
        mocked_system.expect_env().returning(|| HashMap::new());
        mocked_system
            .expect_is_directory_empty()
            .once()
            .returning(|_x| Ok(true));
        let settings: WsSettingsHandler = WsSettingsHandler::from_str(work_dir, json_ws_settings)
            .expect("Failed to parse settings");
        let config: WsBuildConfigHandler =
            WsBuildConfigHandler::from_str(json_build_config, &settings)
                .expect("Failed to parse build config");
        let mut workspace: Workspace =
            Workspace::new(Some(work_dir.to_owned()), Some(settings), Some(config))
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
                "--context",
                "ARG3=arg4",
            ]),
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
        let mut mocked_system: MockSystem = MockSystem::new();
        mocked_system
            .expect_check_call()
            .with(mockall::predicate::eq(CallParams {
                cmd_line: vec![
                    &format!("{}/scripts/script.sh", work_dir.display()),
                    "test-branch",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect(),
                env: HashMap::new(),
                shell: true,
            }))
            .once()
            .returning(|_x| Ok(()));
        mocked_system.expect_env().returning(|| HashMap::new());
        mocked_system
            .expect_is_directory_empty()
            .once()
            .returning(|_x| Ok(true));
        let settings: WsSettingsHandler = WsSettingsHandler::from_str(work_dir, json_ws_settings)
            .expect("Failed to parse settings");
        let config: WsBuildConfigHandler =
            WsBuildConfigHandler::from_str(json_build_config, &settings)
                .expect("Failed to parse build config");
        let mut workspace: Workspace =
            Workspace::new(Some(work_dir.to_owned()), Some(settings), Some(config))
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
            ]),
        );
        let cmd: SetupCommand = SetupCommand::new();
        let _result: Result<(), BError> = cmd.execute(&cli, &mut workspace);
    }
}
