use indexmap::{indexmap, IndexMap};
use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::str::FromStr;

use crate::cli::Cli;
use crate::commands::{BError, BBaseCommand, BCommand, Variant};
use crate::data::context::{CTX_EYECANDY, CTX_KEY_BUILD_VARIANT};
use crate::data::WsContextData;
use crate::executers::{Docker, DockerImage};
use crate::workspace::{Mode, Workspace};

static BCOMMAND: &str = "shell";
static BCOMMAND_ABOUT: &str =
    "Initiate a shell within Docker or execute any command within the BitBake environment.";
pub struct ShellCommand {
    cmd: BBaseCommand,
    // Your struct fields and methods here
}

impl BCommand for ShellCommand {
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
        let variant: Variant = self.get_variant(cli, &workspace.settings().work_dir())?;
        let docker: String = self.get_arg_str(cli, "docker", BCOMMAND)?;
        let volumes: Vec<String> = self.get_arg_many(cli, "volume", BCOMMAND)?;
        let env: Vec<String> = self.get_arg_many(cli, "env", BCOMMAND)?;
        let cmd: String = self.get_arg_str(cli, "run", BCOMMAND)?;
        let docker_pull: bool = self.get_arg_flag(cli, "docker_pull", BCOMMAND)?;
        let eyecandy: bool = self.get_arg_flag(cli, "eyecandy", BCOMMAND)?;
        let interactive_str: String = self.get_arg_str(cli, "interactive", BCOMMAND)?;
        let mut res: Result<(), BError> = Ok(());

        /*
         * If docker is enabled in the workspace settings then bakery will be bootstraped into a docker container
         * with a bakery inside and all the baking will be done inside that docker container. Not all commands should
         * be run inside of docker and if we are already inside docker we should not try and bootstrap into a
         * second docker container.
         */
        if !workspace.settings().docker_disabled()
            && self.is_docker_required()
            && !cli.inside_docker()
        {
            let mut cmd_line: Vec<String> = vec![String::from("bakery"), String::from("shell")];

            if docker_pull {
                self.docker_pull(cli, workspace)?;
            }

            /*
             * We need to rebuild the command line because if the cmd is defined
             * we need to add "" around it to make sure it is not expanded and
             * not getting mixed up with the deej command
             */
            if !cmd.is_empty() {
                cmd_line.append(&mut vec![String::from("-c"), config]);

                cmd_line.append(&mut vec![String::from("-a"), variant.to_string()]);

                if !docker.is_empty() {
                    cmd_line.append(&mut vec![String::from("-d"), docker]);
                }

                if !volumes.is_empty() {
                    volumes.iter().for_each(|key_value| {
                        cmd_line.append(&mut vec![String::from("-v"), key_value.to_string()]);
                    })
                }

                if !env.is_empty() {
                    env.iter().for_each(|key_value| {
                        cmd_line.append(&mut vec![String::from("-e"), key_value.to_string()])
                    })
                }

                cmd_line.append(&mut vec![String::from("-r"), format!("\"{}\"", cmd)]);

                /*
                 * We ignore errors from the shell itself, assuming they originate from
                 * commands executed within the shell. There may be a smarter approach,
                 * but this is sufficient for now.
                 */
                res = self.bootstrap(&cmd_line, cli, workspace, &volumes, true);
                if let Err(err) = res {
                    cli.debug(format!("bootstrap error: {}", err.to_string()));
                }

                return Ok(());
            }

            /*
             * We ignore errors from the shell itself, assuming they originate from
             * commands executed within the shell. There may be a smarter approach,
             * but this is sufficient for now.
             */
            res = self.bootstrap(
                &self.get_cmd_line(cli, &config, Some(variant)),
                cli,
                workspace,
                &volumes,
                true,
            );
            if let Err(err) = res {
                cli.debug(format!("bootstrap error: {}", err.to_string()));
            }

            return Ok(());
        }

        if workspace.settings().mode() == Mode::SETUP {
            return Err(BError::ExecuteCmdInsideWorkspace(
                self.cmd.cmd_str.to_string(),
            ));
        }

        let mut args_ctx: IndexMap<String, String> = indexmap! {
            "BKRY_RELEASE_BUILD".to_string() => "0".to_string(),
            "BKRY_BUILD_VARIANT".to_string() => variant.to_string(),
            CTX_KEY_EYECANDY.to_string() => eyecandy.to_string(),
        };

        if variant.to_string() == "release" {
            /*
             * Build commands defined in the build config needs to
             * know if it is release build or not running by including
             * the BKRY_BUILD_VARIANT to the context we can expose this to
             * the build commands. We are keeping BKRY_RELEASE_BUILD for
             * backwards compatibility but should be replaced with BUILD_VARIANT
             */
            args_ctx.insert("BKRY_RELEASE_BUILD".to_string(), "1".to_string());
        }

        // Update the config context with the context from the args
        let context: WsContextData = WsContextData::new(&args_ctx)?;
        workspace.update_ctx(&context)?;

        if config == "NA" {
            return self.run_shell(cli, workspace, &docker, interactive);
        }

        if !workspace.valid_config(config.as_str()) {
            return Err(BError::CliError(format!(
                "Unsupported build config '{}'",
                config
            )));
        }

        workspace.expand_ctx()?;

        /*
         * We need to read variant from the ctx
         * after we have completed the processing and expansion of the
         * context to make sure that the env in the shell is matching the context
         */
        let ctx_variant: Variant = Variant::from_str(
            &workspace
                .config()
                .build_data()
                .context()
                .get_ctx_value(CTX_KEY_BUILD_VARIANT),
        )?;

        if cmd.is_empty() {
            /*
             * We ignore errors from the shell itself, assuming they originate from
             * commands executed within the shell. There may be a smarter approach,
             * but this is sufficient for now.
             */
            res = self.run_bitbake_shell(cli, workspace, &self.setup_env(env), &docker);
            if let Err(err) = res {
                cli.debug(format!("shell error: {}", err.to_string()));
            }

            return Ok(());
        }

        self.run_cmd(
            &cmd,
            cli,
            workspace,
            &self.setup_env(env),
            &docker,
            interactive,
        )
    }
}

impl ShellCommand {
    pub fn new() -> Self {
        let subcmd: clap::Command = clap::Command::new(BCOMMAND)
        .about(BCOMMAND_ABOUT)
        .arg(
            clap::Arg::new("config")
                .short('c')
                .long("config")
                .help("Setup bitbake build environment if no task specified drop into shell.")
                .value_name("name"),
        )
        .arg(
            clap::Arg::new("verbose")
                .action(clap::ArgAction::SetTrue)
                .long("verbose")
                .help("Set verbose level."),
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
            clap::Arg::new("env")
                .action(clap::ArgAction::Append)
                .short('e')
                .long("env")
                .value_name("KEY=VALUE")
                .help("Extra variables to add to the build environment. This can be used to “lock” the shell to specific environment variables."),
        )
        .arg(
            clap::Arg::new("docker")
                .short('d')
                .long("docker")
                .value_name("registry/image:tag")
                .default_value("")
                .help("Use a custome docker image when creating a shell."),
        )
        .arg(
            clap::Arg::new("docker_pull")
                .action(clap::ArgAction::SetTrue)
                .long("docker-pull")
                .help("Force the bakery shell to pull down the latest docker image from registry."),
        )
        .arg(
            clap::Arg::new("eyecandy")
                .action(clap::ArgAction::SetTrue)
                .long("eyecandy")
                .help("Enable starship https://starship.rs/ if available inside the docker container."),
        )
        .arg(
            clap::Arg::new("run")
                .short('r')
                .long("run-cmd")
                .value_name("cmd")
                .default_value("")
                .help("Run a command inside the docker workspace container"),
        );
        // Initialize and return a new BuildCommand instance
        ShellCommand {
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

    fn bb_build_env(
        &self,
        cli: &Cli,
        workspace: &Workspace,
        args_env_variables: &HashMap<String, String>,
        variant: &Variant,
    ) -> Result<HashMap<String, String>, BError> {
        let init_env: PathBuf = workspace.config().build_data().bitbake().init_env_file();

        /*
         * Env variables priority are
         * 1. Cli env variables
         * 2. System env variables
         */

        /* Sourcing the init env file and returning all the env variables available including from the shell */
        cli.info(format!("source init env file {}", init_env.display()));
        let mut env: HashMap<String, String> = cli.source_init_env(
            &init_env,
            &workspace.config().build_data().bitbake().build_dir(),
        )?;

        /*
         * Set the BKRY_BUILD_CONFIG and BKRY_WORKSPACE env variable used by the aliases in
         * /etc/bkry/bkry.bashrc which is sourced by /etc/bash.bashrc when running an interactive
         * bash shell. This will make it possible to run build, clean, deploy, upload aliases from any location
         * in the shell without having to specify the build config or change directory since it is selected
         * when starting the shell
         */

        env.insert(
            String::from("BKRY_BUILD_CONFIG"),
            workspace.config().build_data().product().name().to_string(),
        );

        env.insert(String::from("BKRY_BUILD_VARIANT"), variant.to_string());

        env.insert(
            String::from("BKRY_WORK_DIR"),
            workspace
                .config()
                .build_data()
                .settings()
                .docker_work_dir()
                .to_string_lossy()
                .to_string(),
        );

        /*
         * Any variable that should be able to passthrough into bitbake needs to be defined as part of the bb passthrough variable
         * we define some defaults that should always be possible to passthrough
         */
        let mut bb_env_passthrough_additions: String = String::from("SSTATE_DIR DL_DIR TMPDIR");

        /* Process the env variables from the cli */
        args_env_variables.iter().for_each(|(key, value)| {
            env.insert(key.clone(), value.clone());
            /*
             * Any variable comming from the cli should not by default be added to the passthrough
             * list. The only way to get it through is if this variable is already defined as part
             * of the task build config env
             */
        });

        if env.contains_key("BB_ENV_PASSTHROUGH_ADDITIONS") {
            bb_env_passthrough_additions.push_str(
                env.get("BB_ENV_PASSTHROUGH_ADDITIONS")
                    .unwrap_or(&String::from("")),
            );
        }

        env.insert(
            String::from("BB_ENV_PASSTHROUGH_ADDITIONS"),
            bb_env_passthrough_additions,
        );
        Ok(env)
    }

    pub fn run_bitbake_shell(
        &self,
        cli: &Cli,
        workspace: &Workspace,
        args_env_variables: &HashMap<String, String>,
        docker: &String,
        Variant: &Variant,
    ) -> Result<(), BError> {
        let cmd_line: Vec<String> = vec![String::from("/bin/bash"), String::from("-i")];

        let mut env: HashMap<String, String> =
            self.bb_build_env(cli, workspace, args_env_variables, variant)?;

        /*
         * Set the BKRY_BUILD_CONFIG and BKRY_WORK_DIR env variable used by the aliases in
         * /etc/bakery/bakery.bashrc which is sourced by /etc/bash.bashrc when running an interactive
         * bash shell. This will make it possible to run build, clean, deploy, upload aliases from any location
         * in the shell without having to specify the build config or change directory since it is selected
         * when starting the shell
         */

        let ctx: IndexMap<String, String> = workspace.context()?;

        cli.debug(format!("ctx: {:?}", ctx));

        let bkry_env: HashMap<String, String> = ctx
            .iter()
            .filter(|(k, _)| k.starts_with("bkry_"))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        cli.debug(format!("bkry-env: {:?}", bkry_env));

        bkry_env.iter().for_each(|(key, value)| {
            //println!(format!("{}={}", key.to_ascii_uppercase(), value));
            env.insert(key.to_ascii_uppercase(), value.clone());
        });

        env.extend(bkry_env);

        cli.info(String::from("Start shell setting up bitbake build env"));
        if !docker.is_empty() {
            let image: DockerImage = DockerImage::new(&docker)?;
            let executer: Docker = Docker::new(image, true);
            return executer.run_cmd(
                &cmd_line,
                cli,
                &workspace.settings().work_dir(),
                &vec![],
                &vec![],
                &env,
            );
        }

        cli.check_call(&cmd_line, &env, true)
    }

    pub fn run_cmd(
        &self,
        cmd: &String,
        cli: &Cli,
        workspace: &Workspace,
        args_env_variables: &HashMap<String, String>,
        docker: &String,
        variant: &Variant,
    ) -> Result<(), BError> {
        let cmd_line: Vec<String> = vec![
            String::from("cd"),
            format!(
                "{}",
                workspace
                    .config()
                    .build_data()
                    .settings()
                    .docker_work_dir()
                    .to_string_lossy()
            ),
            String::from("&&"),
            String::from("/bin/bash"),
            String::from("-i"),
            String::from("-c"),
            format!("\"{}\"", cmd),
        ];

        /*
         * The command don't have to be a bitbake command but we will setup the bb env anyway
         */
        let env: HashMap<String, String> = self.bb_build_env(cli, workspace, args_env_variables, variant)?;
        cli.info(format!("Running command '{}'", cmd));
        if !docker.is_empty() {
            let image: DockerImage = DockerImage::new(&docker)?;
            let executer: Docker = Docker::new(image, true);
            return executer.run_cmd(
                &cmd_line,
                cli,
                &workspace.settings().work_dir(),
                &vec![],
                &vec![],
                &env,
            );
        }

        cli.check_call(&cmd_line, &env, true)
    }

    pub fn run_shell(
        &self,
        cli: &Cli,
        workspace: &Workspace,
        docker: &String,
    ) -> Result<(), BError> {
        let cmd_line: Vec<String> = vec![String::from("/bin/bash"), String::from("-i")];

        cli.info(String::from("Starting shell"));
        if !docker.is_empty() {
            let image: DockerImage = DockerImage::new(&docker)?;
            let executer: Docker = Docker::new(image, true);
            return executer.run_cmd(
                &cmd_line,
                &HashMap::new(),
                &workspace.settings().work_dir(),
                cli,
            );
        }

        cli.check_call(&cmd_line, &HashMap::new(), true)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use tempdir::TempDir;

    use crate::cli::*;
    use crate::commands::{DCommand, ShellCommand};
    use crate::constants::DeejConstants;
    use crate::error::BError;
    use crate::executers::DockerImage;
    use crate::helper::Helper;
    use crate::workspace::{
        Workspace, WsBuildConfigHandler, WsBuildMetadataHandler, WsSettingsHandler,
    };

    fn helper_test_shell_subcommand_custom_ws(
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
            WsBuildMetadataHandler::new(work_dir, &work_dir.join(PathBuf::from(".deej")), None);
        let mut workspace: Workspace = Workspace::new(
            Some(work_dir.to_owned()),
            Some(settings),
            Some(config),
            Some(metadata),
        )?;
        let cli: Cli = Cli::new(logger, system, clap::Command::new("deej"), Some(cmd_line));
        let cmd: ShellCommand = ShellCommand::new();
        cmd.execute(&cli, &mut workspace)
    }

    fn helper_test_shell_subcommand(
        json_build_config: &str,
        work_dir: &PathBuf,
        logger: Box<dyn Logger>,
        system: Box<dyn System>,
        cmd_line: Vec<&str>,
    ) -> Result<(), BError> {
        let json_ws_settings: &str = r#"
        {
            "version": "5",
            "builds": {
                "supported": [
                    "default"
                ]
            },
            "workspace": {
                "configsdir": "configs",
                "includedir": "configs/include",
                "scriptsdir": "scripts"
            }
        }"#;
        let settings: WsSettingsHandler =
            WsSettingsHandler::from_str(work_dir, json_ws_settings, None)?;
        let config: WsBuildConfigHandler =
            WsBuildConfigHandler::from_str(json_build_config, &settings)?;
        let metadata: WsBuildMetadataHandler =
            WsBuildMetadataHandler::new(work_dir, &work_dir.join(PathBuf::from(".deej")), None);
        let mut workspace: Workspace = Workspace::new(
            Some(work_dir.to_owned()),
            Some(settings),
            Some(config),
            Some(metadata),
        )?;
        let cli: Cli = Cli::new(logger, system, clap::Command::new("deej"), Some(cmd_line));
        let cmd: ShellCommand = ShellCommand::new();
        cmd.execute(&cli, &mut workspace)
    }

    /*
     * In this test we are using a default workspace.json and then calling
     * deej shell and making sure that we are calling the expected docker call.
     */
    #[test]
    fn test_cmd_shell() {
        let json_build_config: &str = r#"
        {
            "version": "5",
            "name": "default",
            "description": "Test Description",
            "arch": "test-arch"
        }
        "#;
        let temp_dir: TempDir =
            TempDir::new("deej-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = temp_dir.into_path();
        let docker_image: DockerImage = DockerImage::new(&format!(
            "{}/{}:{}",
            DeejConstants::DOCKER_REGISTRY,
            DeejConstants::DOCKER_IMAGE,
            DeejConstants::DOCKER_TAG
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
            .times(1..11)
            .returning(|_x| true);
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
                        String::from("deej"),
                        String::from("shell"),
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
        let _result: Result<(), BError> = helper_test_shell_subcommand(
            json_build_config,
            &work_dir,
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            vec!["deej", "shell", "--config", "default"],
        );
    }

    /*
     * In this test we are adding volumes as docker args to the workspace.json.
     * One is valid and one is invalid and then we are making sure we print
     * the warning about the invalid volume and that we are calling the expected
     * docker call.
     */
    #[test]
    fn test_cmd_shell_volumes() {
        let json_ws_settings: &str = r#"
        {
            "version": "5",
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
            "version": "5",
            "name": "default",
            "description": "Test Description",
            "arch": "test-arch"
        }
        "#;
        let temp_dir: TempDir =
            TempDir::new("deej-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = temp_dir.into_path();
        let docker_image: DockerImage = DockerImage::new(&format!(
            "{}/{}:{}",
            DeejConstants::DOCKER_REGISTRY,
            DeejConstants::DOCKER_IMAGE,
            DeejConstants::DOCKER_TAG
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
                    &vec!["-v /test/testdir2:/test/testdir2".to_string()],
                    &vec![],
                    &work_dir.clone(),
                    &work_dir,
                    &docker_image,
                    &vec![
                        String::from("deej"),
                        String::from("shell"),
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
        let _result: Result<(), BError> = helper_test_shell_subcommand_custom_ws(
            json_ws_settings,
            json_build_config,
            &work_dir,
            Box::new(mocked_logger),
            Box::new(mocked_system),
            vec!["deej", "shell", "--config", "default"],
        );
    }
}
