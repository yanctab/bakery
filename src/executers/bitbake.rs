use crate::cli::Cli;
use crate::data::{WsBitbakeData, WsTaskData};
use crate::error::BError;
use crate::executers::{Docker, DockerImage, Recipe, TaskExecuter};
use crate::fs::BitbakeConf;

use indexmap::IndexMap;
use std::collections::HashMap;

pub struct BBCleanExecuter<'a> {
    bb_data: &'a WsBitbakeData,
    task_data: &'a WsTaskData,
    cli: &'a Cli,
}

impl<'a> TaskExecuter for BBCleanExecuter<'a> {
    fn exec(
        &self,
        _args_env_variables: &HashMap<String, String>,
        _dry_run: bool,
        _interactive: bool,
    ) -> Result<(), BError> {
        let mut note: String = String::from("Please note that the sstate cache is not cleaned!\n");
        note.push_str(&format!(
            "The sstate cache is located at '{}'\n",
            self.bb_data.sstate_dir().display()
        ));
        note.push_str("The sstate cache might be used by multiple builds\n");
        note.push_str("removing the sstate cache will require a full build\n");
        note.push_str("and can potentially take hours\n");

        self.cli.info(format!(
            "execute bitbake clean task '{}'",
            self.task_data.name()
        ));

        /*
         * We are only allowed to remove the bitbake build dir. We make sure that is
         * the case by checking that it is a subdir of the task build directory which
         * should be set to the work directory when it is a bitbake task
         */
        if self
            .bb_data
            .build_dir()
            .starts_with(self.task_data.build_dir())
        {
            self.cli.info(format!(
                "Removing bitbake build dir '{}'",
                self.bb_data.build_dir().display()
            ));
            self.cli.rmdir_all(&self.bb_data.build_dir())?;
            self.cli.stdout(note);
        } else {
            self.cli.info(format!(
                "Bitbake build dir '{}' is outside of the task build dir '{}'",
                self.bb_data.build_dir().display(),
                self.task_data.build_dir().display()
            ))
        }

        Ok(())
    }
}

impl<'a> BBCleanExecuter<'a> {
    pub fn new(cli: &'a Cli, task_data: &'a WsTaskData, bb_data: &'a WsBitbakeData) -> Self {
        BBCleanExecuter {
            cli,
            bb_data,
            task_data,
        }
    }
}

pub struct BBBuildExecuter<'a> {
    bb_data: &'a WsBitbakeData,
    task_data: &'a WsTaskData,
    bb_variables: &'a Vec<String>,
    cli: &'a Cli,
}

impl<'a> TaskExecuter for BBBuildExecuter<'a> {
    fn exec(
        &self,
        args_env_variables: &HashMap<String, String>,
        dry_run: bool,
        interactive: bool,
    ) -> Result<(), BError> {
        self.cli.info(format!(
            "execute bitbake build task '{}'",
            self.task_data.name()
        ));
        let force: bool = dry_run;
        let env: HashMap<String, String> = self.bb_build_env(args_env_variables)?;
        let exec_dir: std::path::PathBuf = self.bb_data.build_dir();
        let mut docker_str: &str = "";
        // if we are running a dry run we should always create the bb configs.
        // When not a dry run it will be determined if it is needed or not to
        // regenerate the bb configs based on the content of the existing configs
        // comparted to the new content
        let conf: BitbakeConf = BitbakeConf::new(self.bb_data, self.bb_variables, force);
        conf.create_bitbake_configs(self.cli)?;

        if dry_run {
            let mut cmd_line: Vec<String> = vec![];
            cmd_line.append(&mut vec![
                "devtool".to_string(),
                "create-workspace".to_string(),
            ]);
            if !docker_str.is_empty() {
                let image: DockerImage = DockerImage::new(docker_str)?;
                let docker: Docker = Docker::new(image, interactive);
                docker.run_cmd(
                    &mut cmd_line,
                    &self.cli,
                    &exec_dir,
                    &vec![],
                    &vec![],
                    args_env_variables,
                )?;
            } else {
                self.cli.check_call(&cmd_line, &env, true)?;
            }
            self.cli.info("Dry run. Skipping build!".to_string());
            return Ok(());
        }

        for r in self.task_data.recipes() {
            let mut cmd_line: Vec<String> = vec![];
            let recipe: Recipe = Recipe::new(r);
            let mut cmd: Vec<String> = recipe.bitbake_cmd();

            cmd_line.append(&mut vec![
                "cd".to_string(),
                exec_dir.to_string_lossy().to_string(),
                "&&".to_string(),
                "devtool".to_string(),
                "create-workspace".to_string(),
                "&&".to_string(),
            ]);
            cmd_line.append(&mut cmd);

            if !self.task_data.docker_image().is_empty() && self.task_data.docker_image() != "NA" {
                docker_str = self.task_data.docker_image();
            }

            /*
            Not sure that this is a real usecase for now we will just remove this but if we for some
            reason need to be able to use the docker image defined in the bb conf node of the build
            config for executing a task inside docker this can be enabled
            else if !self.bb_data.docker_image().is_empty() && self.bb_data.docker_image() != "NA" {
                // If docker image is set specifically for the task we use that if not we check and
                // see if there is a docker image set in the bb node for the build config which will
                // then be used for all the bitbake tasks. If that is not set then we skip execute
                // docker
                docker_str = self.bb_data.docker_image();
            }
            */

            if !docker_str.is_empty() {
                let image: DockerImage = DockerImage::new(docker_str)?;
                let docker: Docker = Docker::new(image, interactive);
                docker.run_cmd(
                    &mut cmd_line,
                    &self.cli,
                    &exec_dir,
                    &vec![],
                    &vec![],
                    args_env_variables,
                )?;
            } else {
                self.cli.check_call(&cmd_line, &env, true)?;
            }
        }
        Ok(())
    }
}

impl<'a> BBBuildExecuter<'a> {
    fn bb_build_env(
        &self,
        args_env_variables: &HashMap<String, String>,
    ) -> Result<HashMap<String, String>, BError> {
        // Env variables priority are
        // 1. Cli env variables
        // 2. Build config env variables
        // 3. System env variables

        // Sourcing the init env file and returning all the env variables available including from the shell
        self.cli.info(format!(
            "source init env file {}",
            self.bb_data.init_env_file().display()
        ));
        let mut env: HashMap<String, String> = self
            .cli
            .source_init_env(&self.bb_data.init_env_file(), self.task_data.build_dir())?;
        // Reading out the env variables defined in the build config for the specific
        // task that will be executed
        let task_env: &IndexMap<String, String> = self.task_data.env();
        // Any variable that should be able to passthrough into bitbake needs to be defined as part of the bb passthrough variable
        // we define some defaults that should always be possible to passthrough
        let mut bb_env_passthrough_additions: String = String::from("SSTATE_DIR DL_DIR TMPDIR");

        // Process the task build config env variables
        task_env.iter().for_each(|(key, value)| {
            env.insert(key.clone(), value.clone());
            // Add any task build config variable to the list of passthrough variables
            bb_env_passthrough_additions.push_str(&String::from(" "));
            bb_env_passthrough_additions.push_str(&key.clone());
        });

        // Process the env variables from the cli
        args_env_variables.iter().for_each(|(key, value)| {
            env.insert(key.clone(), value.clone());
            // Any variable comming from the cli should not by default be added to the passthrough
            // list. The only way to get it through is if this variable is already defined as part
            // of the task build config env
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

    pub fn new(
        cli: &'a Cli,
        task_data: &'a WsTaskData,
        bb_data: &'a WsBitbakeData,
        bb_variables: &'a Vec<String>,
    ) -> Self {
        BBBuildExecuter {
            cli,
            bb_data,
            task_data,
            bb_variables,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use tempdir::TempDir;

    use crate::cli::*;
    use crate::data::{WsBuildData, WsTaskData};
    use crate::executers::{BBBuildExecuter, BBCleanExecuter, TaskExecuter};
    use crate::helper::Helper;

    #[test]
    fn test_bitbake_executer() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = temp_dir.into_path();
        let build_dir: PathBuf = work_dir.join("builds/default");
        let bb_variables: Vec<String> = vec![];
        let env_variables: HashMap<String, String> = HashMap::new();
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
            }
        }"#;
        let json_task_config: &str = r#"
        {
            "index": "0",
            "name": "task1-name",
            "recipes": [
                "test-image"
            ]
        }"#;
        let data: WsBuildData = Helper::setup_build_data(&work_dir, Some(json_build_config), None);
        let task_data: WsTaskData =
            WsTaskData::from_str(json_task_config, &data).expect("Failed to parse task config");
        let mut mocked_logger: MockLogger = MockLogger::new();
        mocked_logger
            .expect_info()
            .with(mockall::predicate::eq(format!(
                "Autogenerate {}",
                data.bitbake().local_conf_path().display()
            )))
            .once()
            .returning(|_x| ());
        mocked_logger
            .expect_info()
            .with(mockall::predicate::eq(format!(
                "Autogenerate {}",
                data.bitbake().bblayers_conf_path().display()
            )))
            .once()
            .returning(|_x| ());
        mocked_logger
            .expect_info()
            .with(mockall::predicate::eq(format!(
                "source init env file {}",
                data.bitbake().init_env_file().display()
            )))
            .once()
            .returning(|_x| ());
        mocked_logger
            .expect_info()
            .with(mockall::predicate::eq(format!(
                "execute bitbake build task '{}'",
                task_data.name()
            )))
            .once()
            .returning(|_x| ());
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
        let cli: Cli = Cli::new(
            Box::new(mocked_logger),
            Box::new(mocked_system),
            clap::Command::new("bakery"),
            Some(vec!["bakery"]),
        );
        let executer: BBBuildExecuter =
            BBBuildExecuter::new(&cli, &task_data, data.bitbake(), &bb_variables);
        executer
            .exec(&env_variables, false, true)
            .expect("Failed to execute task");
    }

    #[test]
    fn test_bitbake_executer_dry_run() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = temp_dir.into_path();
        let bb_variables: Vec<String> = vec![];
        let env_variables: HashMap<String, String> = HashMap::new();
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
            }
        }"#;
        let json_task_config: &str = r#"
        {
            "index": "0",
            "name": "task1-name",
            "recipes": [
                "test-image"
            ]
        }"#;
        let data: WsBuildData = Helper::setup_build_data(&work_dir, Some(json_build_config), None);
        let task_data: WsTaskData =
            WsTaskData::from_str(json_task_config, &data).expect("Failed to parse task config");
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
                data.bitbake().local_conf_path().display()
            )))
            .once()
            .returning(|_x| ());
        mocked_logger
            .expect_info()
            .with(mockall::predicate::eq(format!(
                "Autogenerate {}",
                data.bitbake().bblayers_conf_path().display()
            )))
            .once()
            .returning(|_x| ());
        mocked_logger
            .expect_info()
            .with(mockall::predicate::eq(format!(
                "source init env file {}",
                data.bitbake().init_env_file().display()
            )))
            .once()
            .returning(|_x| ());
        mocked_logger
            .expect_info()
            .with(mockall::predicate::eq(format!(
                "execute bitbake build task '{}'",
                task_data.name()
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
        let cli: Cli = Cli::new(
            Box::new(mocked_logger),
            Box::new(mocked_system),
            clap::Command::new("bakery"),
            Some(vec!["bakery"]),
        );
        let executer: BBBuildExecuter =
            BBBuildExecuter::new(&cli, &task_data, data.bitbake(), &bb_variables);
        executer
            .exec(&env_variables, true, true)
            .expect("Failed to execute task");
    }

    /*
    #[test]
    fn test_bitbake_executer_docker() {
        let temp_dir: TempDir = TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = temp_dir.into_path();
        let env_file: PathBuf = work_dir.clone().join("bakery-docker.env");
        let build_dir: PathBuf = work_dir.join("builds/default");
        let bb_variables: Vec<String> = vec![];
        let env_variables: HashMap<String, String> = HashMap::new();
        let interactive: bool = true;
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
            }
        }"#;
        let json_task_config: &str = r#"
        {
            "index": "0",
            "name": "task1-name",
            "docker": "test-registry/task-docker:0.1",
            "recipes": [
                "test-image"
            ]
        }"#;
        let docker_cmd_line: Vec<String> = Helper::docker_cmdline_string(
            interactive,
            &build_dir,
            &DockerImage::new("test-registry/task-docker:0.1"),
            &vec![
                String::from("cd"),
                build_dir.to_string_lossy().to_string(),
                String::from("&&"),
                String::from("bitbake"),
                String::from("test-image"),
            ],
            &env_file,
        );
        let mut cmd_line_str: String = String::new();
        docker_cmd_line.iter().for_each(|c|{
            cmd_line_str.push_str(c);
            cmd_line_str.push(' ');
        });
        let data: WsBuildData = Helper::setup_build_data(&work_dir, Some(json_build_config), None);
        let task_data: WsTaskData = WsTaskData::from_str(json_task_config, &data).expect("Failed to parse task config");
        let mut mocked_logger: MockLogger = MockLogger::new();
        mocked_logger
            .expect_info()
            .with(mockall::predicate::eq(format!("Autogenerate {}", data.bitbake().local_conf_path().display())))
            .once()
            .returning(|_x| ());
        mocked_logger
            .expect_info()
            .with(mockall::predicate::eq(format!("Autogenerate {}", data.bitbake().bblayers_conf_path().display())))
            .once()
            .returning(|_x| ());
        mocked_logger
            .expect_info()
            .with(mockall::predicate::eq(format!("source init env file {}", data.bitbake().init_env_file().display())))
            .once()
            .returning(|_x| ());
        mocked_logger
            .expect_info()
            .with(mockall::predicate::eq(format!("execute bitbake build task '{}'", task_data.name())))
            .once()
            .returning(|_x| ());
        mocked_logger
            .expect_info()
            .with(mockall::predicate::eq(cmd_line_str))
            .once()
            .returning(|_x| ());
        let mut mocked_system: MockSystem = MockSystem::new();
        mocked_system
            .expect_check_call()
            .with(mockall::predicate::eq(CallParams {
                cmd_line: docker_cmd_line,
                env: HashMap::new(),
                shell: true,
            }))
            .once()
            .returning(|_x| Ok(()));
        mocked_system
            .expect_init_env_file()
            .returning(|_x, _y| Ok(HashMap::new()));
        let cli: Cli = Cli::new(
            Box::new(mocked_logger),
            Box::new(mocked_system),
            clap::Command::new("bakery"),
            Some(vec!["bakery"]),
        );
        let executer: BBBuildExecuter = BBBuildExecuter::new(&cli, &task_data, data.bitbake(), &bb_variables);
        executer.exec(&env_variables, false, interactive).expect("Failed to execute task");
    }

    #[test]
    fn test_bitbake_executer_bb_docker() {
        let temp_dir: TempDir = TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = temp_dir.into_path();
        let build_dir: PathBuf = work_dir.join("builds/default");
        let bb_variables: Vec<String> = vec![];
        let env_variables: HashMap<String, String> = HashMap::new();
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
                "docker": "test-registry/bb-docker:0.1",
                "bblayersconf": [
                    "LCONF_VERSION=\"7\"",
                    "BBPATH=\"${TOPDIR}\""
                ],
                "localconf": [
                    "BB_NUMBER_THREADS ?= \"${@oe.utils.cpu_count()}\"",
                    "PARALLEL_MAKE ?= \"-j ${@oe.utils.cpu_count()}\""
                ]
            }
        }"#;
        let json_task_config: &str = r#"
        {
            "index": "0",
            "name": "task1-name",
            "recipes": [
                "test-image"
            ]
        }"#;
        let data: WsBuildData = Helper::setup_build_data(&work_dir, Some(json_build_config), None);
        let task_data: WsTaskData = WsTaskData::from_str(json_task_config, &data).expect("Failed to parse task config");
        let mut mocked_logger: MockLogger = MockLogger::new();
        mocked_logger
            .expect_info()
            .with(mockall::predicate::eq(format!("Autogenerate {}", data.bitbake().local_conf_path().display())))
            .once()
            .returning(|_x| ());
        mocked_logger
            .expect_info()
            .with(mockall::predicate::eq(format!("Autogenerate {}", data.bitbake().bblayers_conf_path().display())))
            .once()
            .returning(|_x| ());
        mocked_logger
            .expect_info()
            .with(mockall::predicate::eq(format!("source init env file {}", data.bitbake().init_env_file().display())))
            .once()
            .returning(|_x| ());
        mocked_logger
            .expect_info()
            .with(mockall::predicate::eq(format!("execute bitbake task '{}'", task_data.name())))
            .once()
            .returning(|_x| ());
        mocked_logger
            .expect_info()
            .with(mockall::predicate::eq(format!("docker run test-registry/bb-docker:0.1 cd {} && bitbake test-image", &build_dir.to_string_lossy().to_string())))
            .once()
            .returning(|_x| ());
        let mut mocked_system: MockSystem = MockSystem::new();
        mocked_system
            .expect_check_call()
            .with(mockall::predicate::eq(CallParams {
                cmd_line: vec!["docker", "run", "test-registry/bb-docker:0.1", "cd", &build_dir.to_string_lossy().to_string(), "&&", "bitbake", "test-image"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                env: HashMap::from([(String::from("BB_ENV_PASSTHROUGH_ADDITIONS"), String::from("SSTATE_DIR DL_DIR TMPDIR"))]),
                shell: true,
            }))
            .once()
            .returning(|_x| Ok(()));
        mocked_system
            .expect_init_env_file()
            .returning(|_x, _y| Ok(HashMap::new()));
        let cli: Cli = Cli::new(
            Box::new(mocked_logger),
            Box::new(mocked_system),
            clap::Command::new("bakery"),
            Some(vec!["bakery"]),
        );
        let executer: BitbakeExecuter = BitbakeExecuter::new(&cli, &task_data, data.bitbake(), &bb_variables);
        executer.exec(&env_variables, false, true).expect("Failed to execute task");
    }

    #[test]
    fn test_bitbake_executer_docker_order() {
        let temp_dir: TempDir = TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = temp_dir.into_path();
        let build_dir: PathBuf = work_dir.join("builds/default");
        let bb_variables: Vec<String> = vec![];
        let env_variables: HashMap<String, String> = HashMap::new();
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
                "docker": "test-registry/bb-docker:0.1",
                "bblayersconf": [
                    "LCONF_VERSION=\"7\"",
                    "BBPATH=\"${TOPDIR}\""
                ],
                "localconf": [
                    "BB_NUMBER_THREADS ?= \"${@oe.utils.cpu_count()}\"",
                    "PARALLEL_MAKE ?= \"-j ${@oe.utils.cpu_count()}\""
                ]
            }
        }"#;
        let json_task_config: &str = r#"
        {
            "index": "0",
            "name": "task1-name",
            "docker": "test-registry/task-docker:0.1",
            "recipes": [
                "test-image"
            ]
        }"#;
        let data: WsBuildData = Helper::setup_build_data(&work_dir, Some(json_build_config), None);
        let task_data: WsTaskData = WsTaskData::from_str(json_task_config, &data).expect("Failed to parse task config");
        let mut mocked_logger: MockLogger = MockLogger::new();
        mocked_logger
            .expect_info()
            .with(mockall::predicate::eq(format!("Autogenerate {}", data.bitbake().local_conf_path().display())))
            .once()
            .returning(|_x| ());
        mocked_logger
            .expect_info()
            .with(mockall::predicate::eq(format!("Autogenerate {}", data.bitbake().bblayers_conf_path().display())))
            .once()
            .returning(|_x| ());
        mocked_logger
            .expect_info()
            .with(mockall::predicate::eq(format!("source init env file {}", data.bitbake().init_env_file().display())))
            .once()
            .returning(|_x| ());
        mocked_logger
            .expect_info()
            .with(mockall::predicate::eq(format!("execute bitbake task '{}'", task_data.name())))
            .once()
            .returning(|_x| ());
        mocked_logger
            .expect_info()
            .with(mockall::predicate::eq(format!("docker run test-registry/task-docker:0.1 cd {} && bitbake test-image", &build_dir.to_string_lossy().to_string())))
            .once()
            .returning(|_x| ());
        let mut mocked_system: MockSystem = MockSystem::new();
        mocked_system
            .expect_check_call()
            .with(mockall::predicate::eq(CallParams {
                cmd_line: vec!["docker", "run", "test-registry/task-docker:0.1", "cd", &build_dir.to_string_lossy().to_string(), "&&", "bitbake", "test-image"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                env: HashMap::from([(String::from("BB_ENV_PASSTHROUGH_ADDITIONS"), String::from("SSTATE_DIR DL_DIR TMPDIR"))]),
                shell: true,
            }))
            .once()
            .returning(|_x| Ok(()));
        mocked_system
            .expect_init_env_file()
            .returning(|_x, _y| Ok(HashMap::new()));
        let cli: Cli = Cli::new(
            Box::new(mocked_logger),
            Box::new(mocked_system),
            clap::Command::new("bakery"),
            Some(vec!["bakery"]),
        );
        let executer: BitbakeExecuter = BitbakeExecuter::new(&cli, &task_data, data.bitbake(), &bb_variables);
        executer.exec(&env_variables, false, true).expect("Failed to execute task");
    }
    */

    #[test]
    fn test_bitbake_executer_env_empty() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = temp_dir.into_path();
        let bb_variables: Vec<String> = vec![];
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
                "docker": "test-registry/bb-docker:0.1",
                "bblayersconf": [
                    "LCONF_VERSION=\"7\"",
                    "BBPATH=\"${TOPDIR}\""
                ],
                "localconf": [
                    "BB_NUMBER_THREADS ?= \"${@oe.utils.cpu_count()}\"",
                    "PARALLEL_MAKE ?= \"-j ${@oe.utils.cpu_count()}\""
                ]
            }
        }"#;
        let json_task_config: &str = r#"
        {
            "index": "0",
            "name": "task1-name",
            "docker": "test-registry/task-docker:0.1",
            "recipes": [
                "test-image"
            ]
        }"#;
        let data: WsBuildData = Helper::setup_build_data(&work_dir, Some(json_build_config), None);
        let task_data: WsTaskData =
            WsTaskData::from_str(json_task_config, &data).expect("Failed to parse task config");
        let args_env_variables: HashMap<String, String> = HashMap::new();
        let init_env_variables: HashMap<String, String> = HashMap::new();
        let mut mocked_system: MockSystem = MockSystem::new();
        mocked_system
            .expect_init_env_file()
            .returning(move |_x, _y| Ok(init_env_variables.clone()));
        let cli: Cli = Cli::new(
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            clap::Command::new("bakery"),
            Some(vec!["bakery"]),
        );
        let executer: BBBuildExecuter =
            BBBuildExecuter::new(&cli, &task_data, data.bitbake(), &bb_variables);
        let env: HashMap<String, String> = executer
            .bb_build_env(&args_env_variables)
            .expect("Failed to process bb build env");
        Helper::assert_hashmap(
            &env,
            &HashMap::from([(
                String::from("BB_ENV_PASSTHROUGH_ADDITIONS"),
                String::from("SSTATE_DIR DL_DIR TMPDIR"),
            )]),
        )
    }

    #[test]
    fn test_bitbake_executer_env() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = temp_dir.into_path();
        let bb_variables: Vec<String> = vec![];
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
                "docker": "test-registry/bb-docker:0.1",
                "bblayersconf": [
                    "LCONF_VERSION=\"7\"",
                    "BBPATH=\"${TOPDIR}\""
                ],
                "localconf": [
                    "BB_NUMBER_THREADS ?= \"${@oe.utils.cpu_count()}\"",
                    "PARALLEL_MAKE ?= \"-j ${@oe.utils.cpu_count()}\""
                ]
            }
        }"#;
        let json_task_config: &str = r#"
        {
            "index": "0",
            "name": "task1-name",
            "docker": "test-registry/task-docker:0.1",
            "env": [
                "BUILD_CONFIG_ENV1=BC_VALUE1",
                "BUILD_CONFIG_ENV2=BC_VALUE2"
            ],
            "recipes": [
                "test-image"
            ]
        }"#;
        let data: WsBuildData = Helper::setup_build_data(&work_dir, Some(json_build_config), None);
        let task_data: WsTaskData =
            WsTaskData::from_str(json_task_config, &data).expect("Failed to parse task config");
        let args_env_variables: HashMap<String, String> = HashMap::from([
            (String::from("CLI_ARG_ENV1"), String::from("CLI_VALUE1")),
            (String::from("CLI_ARG_ENV2"), String::from("CLI_VALUE2")),
            (
                String::from("BUILD_CONFIG_ENV2"),
                String::from("CLI_VALUE3"),
            ),
        ]);
        let init_env_variables: HashMap<String, String> = HashMap::new();
        let mut mocked_system: MockSystem = MockSystem::new();
        mocked_system
            .expect_init_env_file()
            .returning(move |_x, _y| Ok(init_env_variables.clone()));
        let cli: Cli = Cli::new(
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            clap::Command::new("bakery"),
            Some(vec!["bakery"]),
        );
        let executer: BBBuildExecuter =
            BBBuildExecuter::new(&cli, &task_data, data.bitbake(), &bb_variables);
        let env: HashMap<String, String> = executer
            .bb_build_env(&args_env_variables)
            .expect("Failed to process bb build env");
        Helper::assert_hashmap(
            &env,
            &HashMap::from([
                (
                    String::from("BB_ENV_PASSTHROUGH_ADDITIONS"),
                    String::from("SSTATE_DIR DL_DIR TMPDIR BUILD_CONFIG_ENV1 BUILD_CONFIG_ENV2"),
                ),
                (String::from("CLI_ARG_ENV1"), String::from("CLI_VALUE1")),
                (String::from("CLI_ARG_ENV2"), String::from("CLI_VALUE2")),
                (String::from("BUILD_CONFIG_ENV1"), String::from("BC_VALUE1")),
                (
                    String::from("BUILD_CONFIG_ENV2"),
                    String::from("CLI_VALUE3"),
                ),
            ]),
        );
    }

    #[test]
    fn test_bitbake_executer_env_prio() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = temp_dir.into_path();
        let bb_variables: Vec<String> = vec![];
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
                "docker": "test-registry/bb-docker:0.1",
                "bblayersconf": [
                    "LCONF_VERSION=\"7\"",
                    "BBPATH=\"${TOPDIR}\""
                ],
                "localconf": [
                    "BB_NUMBER_THREADS ?= \"${@oe.utils.cpu_count()}\"",
                    "PARALLEL_MAKE ?= \"-j ${@oe.utils.cpu_count()}\""
                ]
            }
        }"#;
        let json_task_config: &str = r#"
        {
            "index": "0",
            "name": "task1-name",
            "docker": "test-registry/task-docker:0.1",
            "env": [
                "BUILD_CONFIG_ENV1=BC_VALUE1",
                "BUILD_CONFIG_ENV2=BC_VALUE2"
            ],
            "recipes": [
                "test-image"
            ]
        }"#;
        let data: WsBuildData = Helper::setup_build_data(&work_dir, Some(json_build_config), None);
        let task_data: WsTaskData =
            WsTaskData::from_str(json_task_config, &data).expect("Failed to parse task config");
        // The env cli args should overwrite any env variable defined in the build config
        let args_env_variables: HashMap<String, String> = HashMap::from([
            (String::from("CLI_ARG_ENV1"), String::from("CLI_VALUE1")),
            (String::from("CLI_ARG_ENV2"), String::from("CLI_VALUE2")),
            (
                String::from("BUILD_CONFIG_ENV2"),
                String::from("CLI_VALUE3"),
            ),
        ]);
        // Any variable defined in the system env should always be used
        // we might change this where the cli env have highest priority
        let init_env_variables: HashMap<String, String> = HashMap::from([
            (
                String::from("CLI_ARG_ENV1"),
                String::from("INIT_ENV_VALUE1"),
            ),
            (
                String::from("CLI_ARG_ENV2"),
                String::from("INIT_ENV_VALUE2"),
            ),
            (
                String::from("BUILD_CONFIG_ENV1"),
                String::from("INIT_ENV_VALUE3"),
            ),
            (
                String::from("BUILD_CONFIG_ENV2"),
                String::from("INIT_ENV_VALUE4"),
            ),
            (String::from("SYSTEM_ENV2"), String::from("INIT_ENV_VALUE5")),
        ]);
        let mut mocked_system: MockSystem = MockSystem::new();
        mocked_system
            .expect_init_env_file()
            .returning(move |_x, _y| Ok(init_env_variables.clone()));
        let cli: Cli = Cli::new(
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            clap::Command::new("bakery"),
            Some(vec!["bakery"]),
        );
        let executer: BBBuildExecuter =
            BBBuildExecuter::new(&cli, &task_data, data.bitbake(), &bb_variables);
        let env: HashMap<String, String> = executer
            .bb_build_env(&args_env_variables)
            .expect("Failed to process bb build env");
        Helper::assert_hashmap(
            &env,
            &HashMap::from([
                (
                    String::from("BB_ENV_PASSTHROUGH_ADDITIONS"),
                    String::from("SSTATE_DIR DL_DIR TMPDIR BUILD_CONFIG_ENV1 BUILD_CONFIG_ENV2"),
                ),
                (String::from("CLI_ARG_ENV1"), String::from("CLI_VALUE1")),
                (String::from("CLI_ARG_ENV2"), String::from("CLI_VALUE2")),
                (String::from("BUILD_CONFIG_ENV1"), String::from("BC_VALUE1")),
                (
                    String::from("BUILD_CONFIG_ENV2"),
                    String::from("CLI_VALUE3"),
                ),
                (String::from("SYSTEM_ENV2"), String::from("INIT_ENV_VALUE5")),
            ]),
        );
    }

    #[test]
    fn test_bitbake_clean_executer() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = temp_dir.into_path();
        let env_variables: HashMap<String, String> = HashMap::new();
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
            }
        }"#;
        let json_task_config: &str = r#"
        {
            "index": "0",
            "name": "task1-name",
            "recipes": [
                "test-image"
            ]
        }"#;
        let data: WsBuildData = Helper::setup_build_data(&work_dir, Some(json_build_config), None);
        let mut note: String = String::from("Please note that the sstate cache is not cleaned!\n");
        note.push_str(&format!(
            "The sstate cache is located at '{}'\n",
            data.bitbake().sstate_dir().display()
        ));
        note.push_str("The sstate cache might be used by multiple builds\n");
        note.push_str("removing the sstate cache will require a full build\n");
        note.push_str("and can potentially take hours\n");
        let task_data: WsTaskData =
            WsTaskData::from_str(json_task_config, &data).expect("Failed to parse task config");
        let mut mocked_system: MockSystem = MockSystem::new();
        mocked_system
            .expect_rmdir_all()
            .with(mockall::predicate::eq(data.bitbake().build_dir()))
            .once()
            .returning(|_x| Ok(()));
        let mut mocked_logger: MockLogger = MockLogger::new();
        mocked_logger
            .expect_info()
            .with(mockall::predicate::eq(format!(
                "execute bitbake clean task '{}'",
                task_data.name()
            )))
            .once()
            .returning(|_x| ());
        mocked_logger
            .expect_info()
            .with(mockall::predicate::eq(format!(
                "Removing bitbake build dir '{}'",
                data.bitbake().build_dir().display()
            )))
            .once()
            .returning(|_x| ());
        mocked_logger
            .expect_stdout()
            .with(mockall::predicate::eq(note))
            .once()
            .returning(|_x| ());
        let cli: Cli = Cli::new(
            Box::new(mocked_logger),
            Box::new(mocked_system),
            clap::Command::new("bakery"),
            Some(vec!["bakery"]),
        );
        let executer: BBCleanExecuter = BBCleanExecuter::new(&cli, &task_data, data.bitbake());
        executer
            .exec(&env_variables, true, true)
            .expect("Failed to execute task");
    }
}
