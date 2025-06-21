use indexmap::{indexmap, IndexMap};
use std::collections::HashMap;

use crate::cli::Cli;
use crate::commands::{BBaseCommand, BCommand};
use crate::data::context::{
    CTX_KEY_BUILD_ID, CTX_KEY_BUILD_SHA, CTX_KEY_BUILD_VARIANT, CTX_KEY_PLATFORM_RELEASE,
    CTX_KEY_PLATFORM_VERSION, CTX_KEY_RELEASE_BUILD,
};
use crate::data::WsContextData;
use crate::error::BError;
use crate::executers::Docker;
use crate::workspace::{Mode, Workspace, WsTaskHandler};

static BCOMMAND: &str = "build";
static BCOMMAND_ABOUT: &str =
    "Execute a build, either a full build or a task of one of the builds.";

pub struct BuildCommand {
    cmd: BBaseCommand,
    // Your struct fields and methods here
}

impl BCommand for BuildCommand {
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
        true
    }

    fn execute(&self, cli: &Cli, workspace: &mut Workspace) -> Result<(), BError> {
        let config: String = self.get_arg_str(cli, "config", BCOMMAND)?;
        let version: String = self.get_arg_str(cli, "platform_version", BCOMMAND)?;
        let build_id: String = self.get_arg_str(cli, "build_id", BCOMMAND)?;
        let sha: String = self.get_arg_str(cli, "build_sha", BCOMMAND)?;
        let build_history: bool = self.get_arg_flag(cli, "build_history", BCOMMAND)?;
        let archiver: bool = self.get_arg_flag(cli, "archiver", BCOMMAND)?;
        let debug_symbols: bool = self.get_arg_flag(cli, "debug_symbols", BCOMMAND)?;
        let tar_balls: bool = self.get_arg_flag(cli, "tar_balls", BCOMMAND)?;
        let dry_run: bool = self.get_arg_flag(cli, "dry_run", BCOMMAND)?;
        let interactive_str: String = self.get_arg_str(cli, "interactive", BCOMMAND)?;
        let ctx: Vec<String> = self.get_arg_many(cli, "ctx", BCOMMAND)?;
        let env: Vec<String> = self.get_arg_many(cli, "env", BCOMMAND)?;
        let volumes: Vec<String> = self.get_arg_many(cli, "volume", BCOMMAND)?;
        let tasks: Vec<String> = self.get_arg_many(cli, "tasks", BCOMMAND)?;
        let variant: String = self.get_arg_str(cli, "variant", BCOMMAND)?;
        let verbose: bool = self.get_arg_flag(cli, "verbose", BCOMMAND)?;
        let mut bb_variables: Vec<String> = Vec::new();
        let mut interactive: bool = false;

        if workspace.settings().mode() == Mode::SETUP {
            return Err(BError::CmdInsideWorkspace(self.cmd.cmd_str.to_string()));
        }

        if interactive_str == "true" {
            interactive = true;
        }

        if !workspace.valid_config(config.as_str()) {
            return Err(BError::CliError(format!(
                "Unsupported build config '{}'",
                config
            )));
        }

        /*
         * If docker is enabled in the workspace settings then bakery will be boottraped into a docker container
         * with a bakery inside and all the baking will be done inside that docker container. Not all commands should
         * be run inside of docker and if we are already inside docker we should not try and bootstrap into a
         * second docker container.
         */
        if !workspace.settings().docker_disabled()
            && self.is_docker_required()
            && !Docker::inside_docker()
        {
            return self.bootstrap(&cli.get_cmd_line(), cli, workspace, &volumes, interactive);
        }

        /*
        if !workspace.config().enabled() {
            return Err(BError::CliError(format!("Build config '{}' is currently not enabled", config)));
        }
        */

        if tar_balls {
            bb_variables.push("BB_GENERATE_MIRROR_TARBALLS = \"1\"".to_string());
        }

        if build_history {
            bb_variables.push("INHERIT += \"buildhistory\"".to_string());
            bb_variables.push("BUILDHISTORY_COMMIT = \"1\"".to_string());
        }

        let env_variables: HashMap<String, String> = self.setup_env(env);
        let mut args_context: IndexMap<String, String> = self.setup_context(ctx);

        let mut extra_ctx: IndexMap<String, String> = indexmap! {
            String::from(CTX_KEY_PLATFORM_VERSION) => version.clone(),
            String::from(CTX_KEY_BUILD_ID) => build_id.clone(),
            String::from(CTX_KEY_BUILD_SHA) => sha,
            String::from(CTX_KEY_RELEASE_BUILD) => "0".to_string(),
            String::from(CTX_KEY_BUILD_VARIANT) => variant.clone(),
            String::from(CTX_KEY_PLATFORM_RELEASE) => format!("{}-{}", version, build_id),
        };

        if archiver {
            bb_variables.push("INHERIT += \"archiver\"".to_string());
            bb_variables.push("ARCHIVER_MODE[src] = \"original\"".to_string());
            args_context.insert("BKRY_ARCHIVER".to_string(), "1".to_string());
        }

        if debug_symbols {
            bb_variables.push("IMAGE_GEN_DEBUGFS = \"1\"".to_string());
            bb_variables.push("IMAGE_FSTYPES_DEBUGFS = \"tar.bz2\"".to_string());
            args_context.insert("BKRY_DEBUG_SYMBOLS".to_string(), "1".to_string());
        }

        if variant == "release" {
            /*
             * Build commands defined in the build config needs to
             * know if it is release build or not running by including
             * the BKRY_BUILD_VARIANT to the context we can expose this to
             * the build commands. We are keeping BKRY_RELEASE_BUILD for
             * backwards compatibility but should be replaced with BKRY_BUILD_VARIANT
             */
            extra_ctx.insert(String::from(CTX_KEY_BUILD_VARIANT), "release".to_string());
            extra_ctx.insert(String::from(CTX_KEY_RELEASE_BUILD), "1".to_string());
        }

        // We need to add the extra context variables to the list of bitbake variables
        // so they can be added to the bitbake local.conf file if used as env variables
        // they can also be injected as context variables
        for (key, value) in extra_ctx.clone() {
            let bb_key: &str = key.strip_prefix("BKRY_").unwrap();
            bb_variables.push(format!("{} ?= \"{}\"", bb_key, value));
        }

        // Update the config context with the context from the args
        let mut context: WsContextData = WsContextData::new(&args_context)?;
        context.update(&extra_ctx);
        workspace.update_ctx(&context)?;

        if verbose {
            let variables: IndexMap<String, String> = workspace.context()?;
            cli.debug(format!("Context varibles for build config '{}':", config));
            variables.iter().for_each(|(key, value)| {
                cli.stdout(format!("{}={}", key.to_ascii_uppercase(), value));
            });
        }

        if tasks.len() > 1 {
            // More then one task was specified on the command line
            for t_name in tasks {
                let task: &WsTaskHandler = workspace.config().task(&t_name)?;
                task.build(
                    cli,
                    &workspace.config().build_data(),
                    &bb_variables,
                    &env_variables,
                    dry_run,
                    interactive,
                    true,
                )?;
            }
        } else {
            // One task was specified on the command line or default was used
            let task: &String = tasks.get(0).unwrap();
            if task == "all" {
                // The alias "all" was specified on the command line or it none was specified and "all" was used
                for (_t_name, task) in workspace.config().tasks() {
                    task.build(
                        cli,
                        &workspace.config().build_data(),
                        &bb_variables,
                        &env_variables,
                        dry_run,
                        interactive,
                        false,
                    )?;
                }
            } else {
                // One task was specified on the command line
                let task: &WsTaskHandler = workspace.config().task(tasks.get(0).unwrap())?;
                task.build(
                    cli,
                    &workspace.config().build_data(),
                    &bb_variables,
                    &env_variables,
                    dry_run,
                    interactive,
                    true,
                )?;
            }
        }
        Ok(())
    }
}

impl BuildCommand {
    fn setup_env(&self, env: Vec<String>) -> HashMap<String, String> {
        let variables: HashMap<String, String> = env
            .iter()
            .map(|e| {
                let v: Vec<&str> = e.split('=').collect();
                (v[0].to_string(), v[1].to_string())
            })
            .collect();
        variables
    }

    pub fn new() -> Self {
        let subcmd: clap::Command = clap::Command::new(BCOMMAND)
            .about(BCOMMAND_ABOUT)
            .arg_required_else_help(true)
            .arg(
                clap::Arg::new("config")
                    .short('c')
                    .long("config")
                    .help("The build config defining all the components for the full build")
                    .value_name("name")
                    .required(true),
            )
            .arg(
                clap::Arg::new("verbose")
                    .action(clap::ArgAction::SetTrue)
                    .long("verbose")
                    .help("Set verbose level."),
            )
            .arg(
                clap::Arg::new("tasks")
                    .short('t')
                    .long("tasks")
                    .value_name("tasks")
                    .default_value("all")
                    .value_delimiter(',')
                    .help("The task(s) to execute."),
            )
            .arg(
                clap::Arg::new("env")
                    .action(clap::ArgAction::Append)
                    .short('e')
                    .long("env")
                    .value_name("KEY=VALUE")
                    .help("Extra variables to add to build env for bitbake."),
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
                clap::Arg::new("build_history")
                    .action(clap::ArgAction::SetTrue)
                    .long("build-history")
                    .help("Records information about each package and image and commits that information to a local Git repository where you can examine the information."),
            )
            .arg(
                clap::Arg::new("archiver")
                    .action(clap::ArgAction::SetTrue)
                    .long("archiver")
                    .help("Setting context variable BKRY_ARCHIVER to 1 which will result in adding the archiver class to the local.conf. For more information see https://www.yoctoproject.org/docs/latest/mega-manual/mega-manual.html#ref-classes-archiver."),
            )
            .arg(
                clap::Arg::new("debug_symbols")
                    .action(clap::ArgAction::SetTrue)
                    .long("debug-symbols")
                    .help("Setting context variable BKRY_DEBUG_SYMBOLS to 1 which will result in adding IMAGE_GEN_DEBUGFS=1 to the local.conf. For more information see https://www.yoctoproject.org/docs/latest/mega-manual/mega-manual.html#platdev-gdb-remotedebug."),
            )
            .arg(
                clap::Arg::new("dry_run")
                    .action(clap::ArgAction::SetTrue)
                    .long("dry-run")
                    .help("Only generates local.conf. To manually start the build run source ./layers/poky/oe-init-env-build <build-dir> followed by any bitbake command."),
            )
            .arg(
                clap::Arg::new("tar_balls")
                    .action(clap::ArgAction::SetTrue)
                    .long("tar-balls")
                    .help("This will add BB_GENERATE_MIRROR_TARBALLS=1 to the local.conf. For more information see https://www.yoctoproject.org/docs/latest/mega-manual/mega-manual.html#var-BB_GENERATE_MIRROR_TARBALLS."),
            )
            .arg(
                clap::Arg::new("platform_version")
                    .short('r')
                    .long("platform-version")
                    .value_name("x.y.z")
                    .default_value("0.0.0")
                    .help("Platform version number for the build. Will be available as context variable BKRY_PLATFORM_VERSION"),
            )
            .arg(
                clap::Arg::new("build_sha")
                    .short('s')
                    .long("build-sha")
                    .value_name("sha")
                    .default_value("dev")
                    .help("Sha for the current build. Will be available as a context variable BKRY_BUILD_SHA"),
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
                clap::Arg::new("interactive")
                    .short('i')
                    .long("interactive")
                    .value_name("interactive")
                    .default_value("true")
                    .value_parser(["true", "false"])
                    .help("Determines if a build inside docker should be interactive or not can be useful to set to false when running in the CI"),
            )
            .arg(
                clap::Arg::new("build_id")
                    .short('n')
                    .long("build-id")
                    .value_name("nbr")
                    .default_value("0")
                    .help("Build id number can be used if x.y.z is not enough for some reason and will be part of BKRY_PLATFORM_RELEASE x.y.z-w"),
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
        BuildCommand {
            // Initialize fields if any
            cmd: BBaseCommand {
                cmd_str: String::from(BCOMMAND),
                sub_cmd: subcmd,
                interactive: true,
                require_docker: true,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::Read;
    use std::path::PathBuf;
    use tempdir::TempDir;

    use crate::cli::*;
    use crate::commands::{BCommand, BuildCommand};
    use crate::error::BError;
    use crate::executers::DockerImage;
    use crate::helper::Helper;
    use crate::workspace::{Workspace, WsBuildConfigHandler, WsSettingsHandler};

    fn helper_test_build_subcommand(
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
        let mut workspace: Workspace =
            Workspace::new(Some(work_dir.to_owned()), Some(settings), Some(config))?;
        let cli: Cli = Cli::new(logger, system, clap::Command::new("bakery"), Some(cmd_line));
        let cmd: BuildCommand = BuildCommand::new();
        cmd.execute(&cli, &mut workspace)
    }

    fn helper_verify_bitbake_conf(
        local_conf_path: &PathBuf,
        local_conf_content: &str,
        bblayers_conf_path: &PathBuf,
        bblayers_conf_content: &str,
    ) {
        assert!(local_conf_path.exists());
        assert!(bblayers_conf_path.exists());
        let mut file: File = File::open(local_conf_path).expect("Failed to open local.conf file!");
        let mut contents: String = String::new();
        file.read_to_string(&mut contents)
            .expect("Failed to read local.conf file!");
        let mut validate_local_conf: String = String::from("# AUTO GENERATED\n");
        validate_local_conf.push_str(local_conf_content);
        assert_eq!(validate_local_conf, contents);

        let mut file: File =
            File::open(bblayers_conf_path).expect("Failed to open bblayers.conf file!");
        let mut contents: String = String::new();
        file.read_to_string(&mut contents)
            .expect("Failed to read bblayers.conf file!");
        let mut validate_bblayers_conf: String = String::from("# AUTO GENERATED\n");
        validate_bblayers_conf.push_str(bblayers_conf_content);
        assert_eq!(validate_bblayers_conf, contents);
    }

    fn helper_test_local_conf_args(
        args: &mut Vec<&str>,
        lines: Option<&str>,
        bb_variables: Option<&str>,
    ) {
        let mut cmd_line: Vec<&str> = vec![
            "bakery",
            "build",
            "--config",
            "default",
            "--tasks",
            "image",
            "--dry-run",
        ];
        cmd_line.append(args);
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
            "bb": {
                "machine": "raspberrypi3",
                "variant": "release",
                "distro": "strix",
                "bblayersconf": [
                    "LCONF_VERSION=\"7\"",
                    "BBPATH=\"${TOPDIR}\""
                ],
                "localconf": [
                    "BB_NUMBER_THREADS ?= \"${@oe.utils.cpu_count()}\"",
                    "PARALLEL_MAKE ?= \"-j ${@oe.utils.cpu_count()}\""
                ]
            },
            "tasks": {
                "image": {
                    "index": "1",
                    "name": "image",
                    "recipes": [
                        "image"
                    ]
                }
            }
        }
        "#;
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = temp_dir.into_path();
        let build_dir: PathBuf = work_dir.join("builds/default");
        let local_conf_path: PathBuf = build_dir.clone().join("conf/local.conf");
        let bblayers_conf_path: PathBuf = build_dir.clone().join("conf/bblayers.conf");
        let settings: WsSettingsHandler =
            WsSettingsHandler::from_str(&work_dir, json_ws_settings, None)
                .expect("Failed to setup settings handler");
        let config: WsBuildConfigHandler =
            WsBuildConfigHandler::from_str(json_build_config, &settings)
                .expect("Failed to setup build config handler");
        let mut workspace: Workspace =
            Workspace::new(Some(work_dir.to_owned()), Some(settings), Some(config))
                .expect("Failed to setup workspace handler");
        let mut mocked_system: MockSystem = MockSystem::new();
        mocked_system
            .expect_init_env_file()
            .returning(|_x, _y| Ok(HashMap::new()));
        mocked_system
            .expect_check_call()
            .with(mockall::predicate::eq(CallParams {
                cmd_line: vec!["devtool", "create-workspace"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                env: HashMap::from([(
                    String::from("BB_ENV_PASSTHROUGH_ADDITIONS"),
                    String::from("SSTATE_DIR DL_DIR TMPDIR"),
                )]),
                shell: true,
            }))
            .once()
            .returning(|_x| Ok(()));
        let mut mocked_logger: MockLogger = MockLogger::new();
        mocked_logger
            .expect_info()
            .with(mockall::predicate::eq(format!(
                "Autogenerate {}",
                local_conf_path.display()
            )))
            .once()
            .returning(|_x| ());
        mocked_logger
            .expect_info()
            .with(mockall::predicate::eq(format!(
                "Autogenerate {}",
                bblayers_conf_path.display()
            )))
            .once()
            .returning(|_x| ());
        mocked_logger
            .expect_info()
            .with(mockall::predicate::eq(format!(
                "source init env file {}",
                workspace
                    .config()
                    .build_data()
                    .bitbake()
                    .init_env_file()
                    .display()
            )))
            .once()
            .returning(|_x| ());
        for (name, _task) in workspace.config().tasks() {
            mocked_logger
                .expect_info()
                .with(mockall::predicate::eq(format!(
                    "execute bitbake build task '{}'",
                    name
                )))
                .once()
                .returning(|_x| ());
            mocked_logger
                .expect_info()
                .with(mockall::predicate::eq(
                    "Dry run. Skipping build!".to_string(),
                ))
                .once()
                .returning(|_x| ());
        }
        let cli: Cli = Cli::new(
            Box::new(mocked_logger),
            Box::new(mocked_system),
            clap::Command::new("bakery"),
            Some(cmd_line),
        );
        let cmd: BuildCommand = BuildCommand::new();
        let _result: Result<(), BError> = cmd.execute(&cli, &mut workspace);
        let mut bblayers_conf_content: String = String::from("");
        bblayers_conf_content.push_str("LCONF_VERSION=\"7\"\n");
        bblayers_conf_content.push_str("BBPATH=\"${TOPDIR}\"\n");
        let mut local_conf_content: String = String::from("");
        local_conf_content.push_str("BB_NUMBER_THREADS ?= \"${@oe.utils.cpu_count()}\"\n");
        local_conf_content.push_str("PARALLEL_MAKE ?= \"-j ${@oe.utils.cpu_count()}\"\n");
        local_conf_content.push_str("MACHINE ?= \"raspberrypi3\"\n");
        local_conf_content.push_str("PRODUCT_NAME ?= \"default\"\n");
        local_conf_content.push_str("DISTRO ?= \"strix\"\n");
        local_conf_content.push_str(&format!(
            "SSTATE_DIR ?= \"{}/.cache/test-arch/sstate-cache\"\n",
            work_dir.to_string_lossy().to_string()
        ));
        local_conf_content.push_str(&format!(
            "DL_DIR ?= \"{}/.cache/download\"\n",
            work_dir.to_string_lossy().to_string()
        ));
        local_conf_content.push_str(lines.unwrap_or(""));
        let mut default_bb_variables: String = String::from("");
        default_bb_variables.push_str("PLATFORM_VERSION ?= \"0.0.0\"\n");
        default_bb_variables.push_str("BUILD_ID ?= \"0\"\n");
        default_bb_variables.push_str("BUILD_SHA ?= \"dev\"\n");
        default_bb_variables.push_str("RELEASE_BUILD ?= \"0\"\n");
        default_bb_variables.push_str("BUILD_VARIANT ?= \"dev\"\n");
        default_bb_variables.push_str("PLATFORM_RELEASE ?= \"0.0.0-0\"\n");
        local_conf_content.push_str(bb_variables.unwrap_or(&default_bb_variables));
        helper_verify_bitbake_conf(
            &local_conf_path,
            &local_conf_content,
            &bblayers_conf_path,
            &bblayers_conf_content,
        );
    }

    #[test]
    fn test_cmd_build_bitbake() {
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
                    "recipes": [
                        "test-image"
                    ]
                }
            }
        }
        "#;
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = temp_dir.into_path();
        let build_dir: PathBuf = work_dir.join("builds/default");
        let mut mocked_system: MockSystem = MockSystem::new();
        mocked_system
            .expect_check_call()
            .with(mockall::predicate::eq(CallParams {
                cmd_line: vec![
                    "cd",
                    &build_dir.to_string_lossy().to_string(),
                    "&&",
                    "devtool",
                    "create-workspace",
                    "&&",
                    "bitbake",
                    "test-image",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect(),
                env: HashMap::from([(
                    String::from("BB_ENV_PASSTHROUGH_ADDITIONS"),
                    String::from("SSTATE_DIR DL_DIR TMPDIR"),
                )]),
                shell: true,
            }))
            .once()
            .returning(|_x| Ok(()));
        mocked_system
            .expect_init_env_file()
            .returning(|_x, _y| Ok(HashMap::new()));
        let _result: Result<(), BError> = helper_test_build_subcommand(
            json_ws_settings,
            json_build_config,
            &work_dir,
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            vec!["bakery", "build", "--config", "default"],
        );
    }

    #[test]
    fn test_cmd_build_non_bitbake() {
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
                    "clean": "rm -rf test-dir"
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
                    "test.sh",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect(),
                env: HashMap::new(),
                shell: true,
            }))
            .once()
            .returning(|_x| Ok(()));
        mocked_system
            .expect_init_env_file()
            .returning(|_x, _y| Ok(HashMap::new()));
        mocked_system.expect_env().returning(|| HashMap::new());
        let _result: Result<(), BError> = helper_test_build_subcommand(
            json_ws_settings,
            json_build_config,
            &work_dir,
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            vec!["bakery", "build", "--config", "default"],
        );
    }

    #[test]
    fn test_cmd_build_docker_bitbake() {
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
            "tasks": {
                "task-name": {
                    "index": "1",
                    "name": "task-name",
                    "type": "bitbake",
                    "recipes": [
                        "test-image"
                    ]
                }
            }
        }
        "#;
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = temp_dir.into_path();
        let mut mocked_system: MockSystem = MockSystem::new();
        let docker_image: DockerImage = DockerImage::new(&format!(
            "ghcr.io/yanctab/bakery/bakery-workspace:{}",
            env!("CARGO_PKG_VERSION")
        ))
        .expect("Invalid docker image format");
        mocked_system
            .expect_check_call()
            .with(mockall::predicate::eq(CallParams {
                cmd_line: Helper::docker_bootstrap_string(
                    true,
                    &vec![],
                    &vec![],
                    &work_dir.clone(),
                    &work_dir,
                    &docker_image,
                    &vec![
                        String::from("bakery"),
                        String::from("build"),
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
        let _result: Result<(), BError> = helper_test_build_subcommand(
            json_ws_settings,
            json_build_config,
            &work_dir,
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            vec!["bakery", "build", "--config", "default"],
        );
    }

    #[test]
    fn test_cmd_build_docker_volumes() {
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
            "tasks": {
                "task-name": {
                    "index": "1",
                    "name": "task-name",
                    "type": "bitbake",
                    "recipes": [
                        "test-image"
                    ]
                }
            }
        }
        "#;
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = temp_dir.into_path();
        let docker_image: DockerImage = DockerImage::new(&format!(
            "ghcr.io/yanctab/bakery/bakery-workspace:{}",
            env!("CARGO_PKG_VERSION")
        ))
        .expect("Invalid docker image format");
        let mut mocked_system: MockSystem = MockSystem::new();
        mocked_system
            .expect_check_call()
            .with(mockall::predicate::eq(CallParams {
                cmd_line: Helper::docker_bootstrap_string(
                    true,
                    &vec![],
                    &vec![String::from("/test/testdir:/test/testdir")],
                    &work_dir.clone(),
                    &work_dir,
                    &docker_image,
                    &vec![
                        String::from("bakery"),
                        String::from("build"),
                        String::from("--config"),
                        String::from("default"),
                        String::from("-v"),
                        String::from("/test/testdir:/test/testdir"),
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
        let _result: Result<(), BError> = helper_test_build_subcommand(
            json_ws_settings,
            json_build_config,
            &work_dir,
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            vec![
                "bakery",
                "build",
                "--config",
                "default",
                "-v",
                "/test/testdir:/test/testdir",
            ],
        );
    }

    #[test]
    fn test_cmd_build_docker_interactive() {
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
            "tasks": {
                "task-name": {
                    "index": "1",
                    "name": "task-name",
                    "type": "bitbake",
                    "recipes": [
                        "test-image"
                    ]
                }
            }
        }
        "#;
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = temp_dir.into_path();
        let docker_image: DockerImage = DockerImage::new(&format!(
            "ghcr.io/yanctab/bakery/bakery-workspace:{}",
            env!("CARGO_PKG_VERSION")
        ))
        .expect("Invalid docker image format");
        let mut mocked_system: MockSystem = MockSystem::new();
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
                        String::from("build"),
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
        mocked_system
            .expect_init_env_file()
            .returning(|_x, _y| Ok(HashMap::new()));
        mocked_system.expect_env().returning(|| HashMap::new());
        let _result: Result<(), BError> = helper_test_build_subcommand(
            json_ws_settings,
            json_build_config,
            &work_dir,
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            vec![
                "bakery",
                "build",
                "--config",
                "default",
                "--interactive=false",
            ],
        );
    }

    #[test]
    fn test_cmd_build_docker_args() {
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
                    "--test=test"
                ]
            }
        }"#;
        let json_build_config: &str = r#"
        {
            "version": "6",
            "name": "default",
            "description": "Test Description",
            "arch": "test-arch",
            "tasks": {
                "task-name": {
                    "index": "1",
                    "name": "task-name",
                    "type": "bitbake",
                    "recipes": [
                        "test-image"
                    ]
                }
            }
        }
        "#;
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = temp_dir.into_path();
        let docker_image: DockerImage = DockerImage::new(&format!(
            "ghcr.io/yanctab/bakery/bakery-workspace:{}",
            env!("CARGO_PKG_VERSION")
        ))
        .expect("Invalid docker image format");
        let mut mocked_system: MockSystem = MockSystem::new();
        mocked_system
            .expect_check_call()
            .with(mockall::predicate::eq(CallParams {
                cmd_line: Helper::docker_bootstrap_string(
                    true,
                    &vec![String::from("--test=test")],
                    &vec![],
                    &work_dir.clone(),
                    &work_dir,
                    &docker_image,
                    &vec![
                        String::from("bakery"),
                        String::from("build"),
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
        let _result: Result<(), BError> = helper_test_build_subcommand(
            json_ws_settings,
            json_build_config,
            &work_dir,
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            vec!["bakery", "build", "--config", "default"],
        );
    }

    #[test]
    fn test_cmd_build_task() {
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
                    "type": "bitbake",
                    "recipes": [
                        "test-image"
                    ]
                }
            }
        }
        "#;
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = temp_dir.into_path();
        let build_dir: PathBuf = work_dir.join("builds/default");
        let mut mocked_system: MockSystem = MockSystem::new();
        mocked_system
            .expect_check_call()
            .with(mockall::predicate::eq(CallParams {
                cmd_line: vec![
                    "cd",
                    &build_dir.to_string_lossy().to_string(),
                    "&&",
                    "devtool",
                    "create-workspace",
                    "&&",
                    "bitbake",
                    "test-image",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect(),
                env: HashMap::from([(
                    String::from("BB_ENV_PASSTHROUGH_ADDITIONS"),
                    String::from("SSTATE_DIR DL_DIR TMPDIR"),
                )]),
                shell: true,
            }))
            .once()
            .returning(|_x| Ok(()));
        mocked_system
            .expect_init_env_file()
            .returning(|_x, _y| Ok(HashMap::new()));
        let _result: Result<(), BError> = helper_test_build_subcommand(
            json_ws_settings,
            json_build_config,
            &work_dir,
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            vec![
                "bakery",
                "build",
                "--config",
                "default",
                "--tasks",
                "task-name",
            ],
        );
    }

    #[test]
    fn test_cmd_build_tasks() {
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
                "image": {
                    "index": "1",
                    "name": "image",
                    "recipes": [
                        "image"
                    ]
                },
                "test": {
                    "index": "2",
                    "name": "test",
                    "recipes": [
                        "test-image"
                    ]
                },
                "sdk": {
                    "index": "3",
                    "name": "sdk",
                    "recipes": [
                        "image:sdk"
                    ]
                }
            }
        }
        "#;
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = temp_dir.into_path();
        let build_dir: PathBuf = work_dir.join("builds/default");
        let mut mocked_system: MockSystem = MockSystem::new();
        mocked_system
            .expect_check_call()
            .with(mockall::predicate::eq(CallParams {
                cmd_line: vec![
                    "cd",
                    &build_dir.to_string_lossy().to_string(),
                    "&&",
                    "devtool",
                    "create-workspace",
                    "&&",
                    "bitbake",
                    "image",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect(),
                env: HashMap::from([(
                    String::from("BB_ENV_PASSTHROUGH_ADDITIONS"),
                    String::from("SSTATE_DIR DL_DIR TMPDIR"),
                )]),
                shell: true,
            }))
            .once()
            .returning(|_x| Ok(()));
        mocked_system
            .expect_check_call()
            .with(mockall::predicate::eq(CallParams {
                cmd_line: vec![
                    "cd",
                    &build_dir.to_string_lossy().to_string(),
                    "&&",
                    "devtool",
                    "create-workspace",
                    "&&",
                    "bitbake",
                    "image",
                    "-c",
                    "do_populate_sdk",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect(),
                env: HashMap::from([(
                    String::from("BB_ENV_PASSTHROUGH_ADDITIONS"),
                    String::from("SSTATE_DIR DL_DIR TMPDIR"),
                )]),
                shell: true,
            }))
            .once()
            .returning(|_x| Ok(()));
        mocked_system
            .expect_init_env_file()
            .returning(|_x, _y| Ok(HashMap::new()));
        let _result: Result<(), BError> = helper_test_build_subcommand(
            json_ws_settings,
            json_build_config,
            &work_dir,
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            vec![
                "bakery",
                "build",
                "--config",
                "default",
                "--tasks",
                "image,sdk",
            ],
        );
    }

    #[test]
    fn test_cmd_build_arg_build_history() {
        let mut local_conf_lines: String = String::from("");
        local_conf_lines.push_str("INHERIT += \"buildhistory\"\n");
        local_conf_lines.push_str("BUILDHISTORY_COMMIT = \"1\"\n");
        helper_test_local_conf_args(&mut vec!["--build-history"], Some(&local_conf_lines), None);
    }

    #[test]
    fn test_cmd_build_arg_tar_balls() {
        let mut local_conf_lines: String = String::from("");
        local_conf_lines.push_str("BB_GENERATE_MIRROR_TARBALLS = \"1\"\n");
        helper_test_local_conf_args(&mut vec!["--tar-balls"], Some(&local_conf_lines), None);
    }

    #[test]
    fn test_cmd_build_arg_debug_symbols() {
        let mut local_conf_lines: String = String::from("");
        local_conf_lines.push_str("IMAGE_GEN_DEBUGFS = \"1\"\n");
        local_conf_lines.push_str("IMAGE_FSTYPES_DEBUGFS = \"tar.bz2\"\n");
        helper_test_local_conf_args(&mut vec!["--debug-symbols"], Some(&local_conf_lines), None);
    }

    #[test]
    fn test_cmd_build_arg_archiver() {
        let mut local_conf_lines: String = String::from("");
        local_conf_lines.push_str("INHERIT += \"archiver\"\n");
        local_conf_lines.push_str("ARCHIVER_MODE[src] = \"original\"\n");
        helper_test_local_conf_args(&mut vec!["--archiver"], Some(&local_conf_lines), None);
    }

    #[test]
    fn test_cmd_build_arg_bitbake_variables() {
        let mut bb_variables: String = String::from("");
        bb_variables.push_str("PLATFORM_VERSION ?= \"1.2.3\"\n");
        bb_variables.push_str("BUILD_ID ?= \"4\"\n");
        bb_variables.push_str("BUILD_SHA ?= \"abcdefgh\"\n");
        bb_variables.push_str("RELEASE_BUILD ?= \"1\"\n");
        bb_variables.push_str("BUILD_VARIANT ?= \"release\"\n");
        bb_variables.push_str("PLATFORM_RELEASE ?= \"1.2.3-4\"\n");
        helper_test_local_conf_args(
            &mut vec![
                "--platform-version=1.2.3",
                "--build-id=4",
                "--build-sha=abcdefgh",
                "--variant=release",
            ],
            None,
            Some(&bb_variables),
        );
    }

    #[test]
    fn test_cmd_build_arg_variant_test() {
        let mut bb_variables: String = String::from("");
        bb_variables.push_str("PLATFORM_VERSION ?= \"0.0.0\"\n");
        bb_variables.push_str("BUILD_ID ?= \"0\"\n");
        bb_variables.push_str("BUILD_SHA ?= \"dev\"\n");
        bb_variables.push_str("RELEASE_BUILD ?= \"0\"\n");
        bb_variables.push_str("BUILD_VARIANT ?= \"test\"\n");
        bb_variables.push_str("PLATFORM_RELEASE ?= \"0.0.0-0\"\n");
        helper_test_local_conf_args(&mut vec!["--variant=test"], None, Some(&bb_variables));
    }

    #[test]
    fn test_cmd_build_context() {
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
            "context": [
                "DIR1=dir1",
                "DIR2=$#[DIR1]/dir2",
                "PROJECT=all"
            ],
            "tasks": {
                "task-name": {
                    "index": "1",
                    "name": "task-name",
                    "type": "non-bitbake",
                    "builddir": "build/$#[DIR2]",
                    "build": "test.sh build $#[PROJECT]",
                    "clean": "test.sh clean $#[PROJECT]"
                }
            }
        }"#;
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = temp_dir.into_path();
        let build_dir: PathBuf = work_dir.join("build/dir3/dir2");
        let mut mocked_system: MockSystem = MockSystem::new();
        mocked_system
            .expect_check_call()
            .with(mockall::predicate::eq(CallParams {
                cmd_line: vec![
                    "cd",
                    &build_dir.to_string_lossy().to_string(),
                    "&&",
                    "test.sh",
                    "build",
                    "test",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect(),
                env: HashMap::new(),
                shell: true,
            }))
            .once()
            .returning(|_x| Ok(()));
        mocked_system
            .expect_init_env_file()
            .returning(|_x, _y| Ok(HashMap::new()));
        let _result: Result<(), BError> = helper_test_build_subcommand(
            json_ws_settings,
            json_build_config,
            &work_dir,
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            vec![
                "bakery",
                "build",
                "--config",
                "default",
                "--tasks",
                "task-name",
                "--context",
                "DIR1=dir3",
                "--context",
                "PROJECT=test",
            ],
        );
    }

    /*
    #[test]
    fn test_cmd_build_env() {
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
            "tasks": {
                "task-name": {
                    "index": "1",
                    "name": "task-name",
                    "type": "non-bitbake",
                    "env": [
                        "ENV_VAR1=VALUE1"
                    ],
                    "builddir": "build",
                    "build": "test.sh build",
                    "clean": "test.sh clean"
                }
            }
        }"#;
        let temp_dir: TempDir = TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = temp_dir.into_path();
        let build_dir: PathBuf = work_dir.join("build/");
        let mut mocked_system: MockSystem = MockSystem::new();
        mocked_system
            .expect_check_call()
            .with(mockall::predicate::eq(CallParams {
                cmd_line: vec!["cd", &build_dir.to_string_lossy().to_string(), "&&", "test.sh", "build"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                env: HashMap::new(),
                shell: true,
            }))
            .once()
            .returning(|_x| Ok(()));
        let _result: Result<(), BError> = helper_test_build_subcommand(
            json_ws_settings,
            json_build_config,
            &work_dir,
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            vec!["bakery", "build", "--config", "default", "--tasks", "task-name", "--env", "ENV_VAR1=CLI_VALUE1"],
        );
    }
    */
}
