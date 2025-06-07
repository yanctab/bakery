use crate::cli::Cli;
use crate::collector::{Collected, Collector, CollectorFactory};
use crate::configs::Context;
use crate::data::{TType, WsBuildData, WsTaskData};
use crate::error::BError;
use crate::executers::{
    BBBuildExecuter, BBCleanExecuter, NonBBBuildExecuter, NonBBCleanExecuter, TaskExecuter,
};
use crate::fs::ConfigFileReader;
use crate::workspace::WsArtifactsHandler;

use serde_json::Value;
use std::collections::HashMap;

pub struct WsTaskHandler {
    data: WsTaskData,
    artifacts: Vec<WsArtifactsHandler>,
}

impl WsTaskHandler {
    pub fn from_str(json_config: &str, build_data: &WsBuildData) -> Result<Self, BError> {
        let data: Value = ConfigFileReader::parse(json_config)?;
        Self::new(&data, build_data)
    }

    pub fn new(data: &Value, build_data: &WsBuildData) -> Result<Self, BError> {
        let task_data: WsTaskData = WsTaskData::from_value(data, build_data)?;
        let artifacts: Vec<WsArtifactsHandler> =
            build_data.get_artifacts(data, task_data.build_dir())?;

        Ok(WsTaskHandler {
            data: task_data,
            artifacts,
        })
    }

    pub fn build<'a>(
        &self,
        cli: &'a Cli,
        build_data: &WsBuildData,
        bb_variables: &Vec<String>,
        env_variables: &HashMap<String, String>,
        dry_run: bool,
        interactive: bool,
        force: bool,
    ) -> Result<(), BError> {
        let executer: Box<dyn TaskExecuter>;

        if !force && self.data.disabled() {
            cli.info(format!("Task '{}' disabled, skipping", self.data.name()));
            return Ok(());
        }

        if !force && !self.data.condition() {
            cli.info(format!(
                "Task condition for '{}' is not met, skipping",
                self.data.name()
            ));
            return Ok(());
        }

        match self.data.ttype() {
            TType::Bitbake => {
                executer = Box::new(BBBuildExecuter::new(
                    cli,
                    &self.data,
                    build_data.bitbake(),
                    bb_variables,
                ));
            }
            TType::NonBitbake => {
                executer = Box::new(NonBBBuildExecuter::new(cli, &self.data));
            }
        }

        executer.exec(env_variables, dry_run, interactive)?;

        if !dry_run {
            self.collect(cli, build_data)?;
        }

        Ok(())
    }

    pub fn clean<'a>(
        &self,
        cli: &'a Cli,
        build_data: &WsBuildData,
        env_variables: &HashMap<String, String>,
        force: bool,
    ) -> Result<(), BError> {
        let executer: Box<dyn TaskExecuter>;

        if self.data.disabled() {
            cli.info(format!(
                "Task '{}' is disabled in build config, execution is skipped",
                self.data.name()
            ));
            return Ok(());
        }

        if !force && !self.data.condition() {
            cli.info(format!(
                "Task condition for task '{}' is not met, execution is skipped",
                self.data.name()
            ));
            return Ok(());
        }

        match self.data.ttype() {
            TType::Bitbake => {
                executer = Box::new(BBCleanExecuter::new(cli, &self.data, build_data.bitbake()));
            }
            TType::NonBitbake => {
                executer = Box::new(NonBBCleanExecuter::new(cli, &self.data));
            }
        }

        executer.exec(env_variables, false, false)?;

        Ok(())
    }

    pub fn collect(&self, cli: &Cli, build_data: &WsBuildData) -> Result<Vec<Collected>, BError> {
        let mut collected: Vec<Collected> = vec![];

        if !self.artifacts.is_empty() {
            cli.info(format!(
                "Collecting artifacts for task '{}'",
                self.data.name()
            ));
            for artifact in self.artifacts.iter() {
                let collector: Box<dyn Collector> = CollectorFactory::create(artifact, Some(cli))?;
                let mut c: Vec<Collected> = collector.collect(
                    self.data.build_dir(),
                    &build_data.settings().artifacts_dir(),
                )?;
                collected.append(&mut c);
            }

            cli.info(format!(
                "All artifacts for task '{}' have been collected to '{}'",
                self.data.name(),
                build_data
                    .settings()
                    .artifacts_dir()
                    .to_string_lossy()
                    .to_string()
            ));
        } else {
            cli.info(format!(
                "No artifacts to collect for task '{}'",
                self.data.name()
            ));
        }

        Ok(collected)
    }

    pub fn expand_ctx(&mut self, ctx: &Context) -> Result<(), BError> {
        self.data.expand_ctx(ctx)?;
        for a in self.artifacts.iter_mut() {
            a.expand_ctx(ctx)?;
        }
        Ok(())
    }

    pub fn data(&self) -> &WsTaskData {
        &self.data
    }

    pub fn artifacts(&self) -> &Vec<WsArtifactsHandler> {
        &self.artifacts
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::Read;
    use std::path::{Path, PathBuf};
    use tempdir::TempDir;

    use crate::cli::{BLogger, CallParams, Cli, MockLogger, MockSystem};
    use crate::data::{AType, TType, WsBuildData};
    use crate::helper::Helper;
    use crate::workspace::{WsArtifactsHandler, WsTaskHandler};

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

    #[test]
    fn test_ws_task_nonbitbake() {
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let json_task_str: &str = r#"
        {
            "index": "0",
            "name": "task-name",
            "type": "non-bitbake",
            "disabled": "false",
            "condition": "true",
            "builddir": "task/dir",
            "build": "build-cmd",
            "clean": "clean-cmd",
            "artifacts": []
        }"#;
        let build_data: WsBuildData = Helper::setup_build_data(&work_dir, None, None);
        let task: WsTaskHandler = WsTaskHandler::from_str(json_task_str, &build_data)
            .expect("Failed to parse Task config");
        assert_eq!(
            task.data().build_dir(),
            &PathBuf::from("/workspace/task/dir")
        );
        assert!(task.data().condition());
        assert_eq!(task.data().name(), "task-name");
        assert_eq!(task.data().build_cmd(), "build-cmd");
        assert_eq!(task.data().clean_cmd(), "clean-cmd");
        assert_eq!(task.data().ttype(), &TType::NonBitbake);
        assert!(!task.data().disabled());
    }

    #[test]
    fn test_ws_task_bitbake() {
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let json_task_str: &str = r#"
        {
            "index": "2",
            "name": "task-name",
            "type": "bitbake",
            "recipes": [
                "test-image"
            ]
        }"#;
        let build_data: WsBuildData = Helper::setup_build_data(&work_dir, None, None);
        let task: WsTaskHandler = WsTaskHandler::from_str(json_task_str, &build_data)
            .expect("Failed to parse Task config");
        assert_eq!(
            task.data().build_dir(),
            &PathBuf::from("/workspace/builds/NA")
        );
        assert!(task.data().condition());
        assert_eq!(task.data().name(), "task-name");
        assert_eq!(task.data().ttype(), &TType::Bitbake);
        assert_eq!(task.data().recipes(), &vec!["test-image".to_string()]);
        assert!(!task.data().disabled());
    }

    #[test]
    fn test_ws_task_bb_build_dir() {
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let json_task_str: &str = r#"
        {
            "index": "2",
            "name": "task-name",
            "type": "bitbake",
            "recipes": [
                "test-image"
            ]
        }"#;
        let build_data: WsBuildData = Helper::setup_build_data(&work_dir, None, None);
        let task: WsTaskHandler = WsTaskHandler::from_str(json_task_str, &build_data)
            .expect("Failed to parse Task config");
        assert_eq!(
            task.data().build_dir(),
            &PathBuf::from("/workspace/builds/NA")
        );
        assert!(task.data().condition());
        assert_eq!(task.data().name(), "task-name");
        assert_eq!(task.data().ttype(), &TType::Bitbake);
        assert_eq!(task.data().recipes(), &vec!["test-image".to_string()]);
        assert!(!task.data().disabled());
    }

    #[test]
    fn test_ws_task_nonbitbake_artifacts() {
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let json_task_str: &str = r#"
        {
            "index": "2",
            "name": "task-name",
            "type": "non-bitbake",
            "builddir": "task/build/dir",
            "build": "build-cmd",
            "clean": "clean-cmd",
            "artifacts": [
                {
                    "type": "archive",
                    "name": "test.zip",
                    "artifacts": [
                        {
                            "source": "file3.txt",
                            "dest": "file4.txt"
                        },
                        {
                            "type": "directory",
                            "name": "dir-name",
                            "artifacts": [
                                {
                                    "source": "file1.txt"
                                },
                                {
                                    "source": "file2.txt"
                                },
                                {
                                    "source": "file3.txt"
                                }
                            ]
                        }
                    ]
                }
            ]
        }"#;
        let build_data: WsBuildData = Helper::setup_build_data(&work_dir, None, None);
        let task: WsTaskHandler = WsTaskHandler::from_str(json_task_str, &build_data)
            .expect("Failed to parse Task config");
        assert_eq!(
            task.data().build_dir(),
            &PathBuf::from("/workspace/task/build/dir")
        );
        assert!(task.data().condition());
        assert_eq!(task.data().name(), "task-name");
        assert_eq!(task.data().ttype(), &TType::NonBitbake);
        assert_eq!(task.data().build_cmd(), "build-cmd");
        assert_eq!(task.data().clean_cmd(), "clean-cmd");
        assert!(!task.data().disabled());
        let artifacts: &WsArtifactsHandler = task.artifacts().first().unwrap();
        assert_eq!(artifacts.data().atype(), &AType::Archive);
        assert_eq!(artifacts.data().name(), "test.zip");
        assert!(!artifacts.children().is_empty());
        let archive_artifacts: &Vec<WsArtifactsHandler> = artifacts.children();
        assert_eq!(
            archive_artifacts.get(0).unwrap().data().atype(),
            &AType::File
        );
        assert_eq!(
            archive_artifacts.get(0).unwrap().data().source(),
            "file3.txt"
        );
        assert_eq!(archive_artifacts.get(0).unwrap().data().dest(), "file4.txt");
        assert_eq!(
            archive_artifacts.get(1).unwrap().data().atype(),
            &AType::Directory
        );
        assert_eq!(archive_artifacts.get(1).unwrap().data().name(), "dir-name");
        let dir_artifacts: &Vec<WsArtifactsHandler> = archive_artifacts.get(1).unwrap().children();
        let mut i: usize = 1;
        dir_artifacts.iter().for_each(|f| {
            assert_eq!(f.data().atype(), &AType::File);
            assert_eq!(f.data().source(), &format!("file{}.txt", i));
            assert_eq!(f.data().dest(), "");
            i += 1;
        });
    }

    #[test]
    fn test_ws_task_expand_ctx() {
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let json_build_config: &str = r#"
        {
            "version": "6",
            "name": "test-name",
            "description": "Test Description",
            "arch": "test-arch",
            "context": [
                "ARCHIVE_NAME=test.zip",
                "DIR_NAME=dir-name",
                "FILE_NAME=file2.txt",
                "BITBAKE_IMAGE=test-image",
                "DEST_NAME=file-dest.txt",
                "DEST_FILE_NAME=$#[DEST_NAME]",
                "MANIFEST_FILE=test-manifest.json",
                "KEY_CONTEXT1=VAR1",
                "KEY_CONTEXT2=VAR2",
                "KEY_CONTEXT3=VAR3",
                "KEY_CONTEXT4=VAR4"
            ]
        }"#;
        let json_task_str: &str = r#"
        {
            "index": "2",
            "name": "task-name",
            "type": "bitbake",
            "recipes": [
                "$#[BITBAKE_IMAGE]"
            ],
            "artifacts": [
                {
                    "type": "archive",
                    "name": "$#[ARCHIVE_NAME]",
                    "artifacts": [
                        {
                            "source": "file3.txt",
                            "dest": "file4.txt"
                        },
                        {
                            "type": "manifest",
                            "name": "$#[MANIFEST_FILE]",
                            "content": {
                                "$#[KEY_CONTEXT1]": "value1",
                                "$#[KEY_CONTEXT2]": "value2",
                                "$#[KEY_CONTEXT3]": "value3",
                                "data": {
                                    "$#[KEY_CONTEXT4]": "value4"
                                }
                            }
                        },
                        {
                            "type": "directory",
                            "name": "$#[DIR_NAME]",
                            "artifacts": [
                                {
                                    "source": "file1.txt",
                                    "dest": "$#[DEST_NAME]"
                                },
                                {
                                    "source": "$#[FILE_NAME]",
                                    "dest": "$#[DEST_NAME]"
                                },
                                {
                                    "source": "file3.txt",
                                    "dest": "$#[DEST_NAME]"
                                }
                            ]
                        }
                    ]
                }
            ]
        }"#;
        let build_data: WsBuildData =
            Helper::setup_build_data(&work_dir, Some(json_build_config), None);
        let mut task: WsTaskHandler = WsTaskHandler::from_str(json_task_str, &build_data)
            .expect("Failed to parse Task config");
        task.expand_ctx(build_data.context().ctx()).unwrap();
        assert_eq!(
            task.data().build_dir(),
            &PathBuf::from("/workspace/builds/test-name")
        );
        assert!(task.data().condition());
        assert_eq!(task.data().name(), "task-name");
        assert_eq!(task.data().ttype(), &TType::Bitbake);
        assert_eq!(task.data().recipes(), &vec!["test-image".to_string()]);
        assert!(!task.data().disabled());
        let artifacts: &WsArtifactsHandler = task.artifacts().first().unwrap();
        assert_eq!(artifacts.data().atype(), &AType::Archive);
        assert_eq!(artifacts.data().name(), "test.zip");
        assert!(!artifacts.children().is_empty());
        let archive_artifacts: &Vec<WsArtifactsHandler> = artifacts.children();
        assert_eq!(
            archive_artifacts.get(0).unwrap().data().atype(),
            &AType::File
        );
        assert_eq!(
            archive_artifacts.get(0).unwrap().data().source(),
            "file3.txt"
        );
        assert_eq!(archive_artifacts.get(0).unwrap().data().dest(), "file4.txt");
        assert_eq!(
            archive_artifacts.get(1).unwrap().data().name(),
            "test-manifest.json"
        );
        assert!(!archive_artifacts
            .get(1)
            .unwrap()
            .data()
            .manifest()
            .is_empty());
        assert_eq!(archive_artifacts.get(1).unwrap().data().manifest(), "{\"VAR1\":\"value1\",\"VAR2\":\"value2\",\"VAR3\":\"value3\",\"data\":{\"VAR4\":\"value4\"}}");
        assert_eq!(
            archive_artifacts.get(2).unwrap().data().atype(),
            &AType::Directory
        );
        assert_eq!(archive_artifacts.get(2).unwrap().data().name(), "dir-name");
        let dir_artifacts: &Vec<WsArtifactsHandler> = archive_artifacts.get(2).unwrap().children();
        let mut i: usize = 1;
        dir_artifacts.iter().for_each(|f| {
            assert_eq!(f.data().atype(), &AType::File);
            assert_eq!(f.data().source(), &format!("file{}.txt", i));
            assert_eq!(f.data().dest(), "file-dest.txt");
            i += 1;
        });
    }

    #[test]
    fn test_ws_task_run_bitbake() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let path: &Path = temp_dir.path();
        let work_dir: PathBuf = PathBuf::from(path);
        let json_task_str: &str = r#"
        {
            "index": "2",
            "name": "task-name",
            "type": "bitbake",
            "recipes": [
                "test-image"
            ]
        }"#;
        let build_data: WsBuildData = Helper::setup_build_data(&work_dir, None, None);
        let task: WsTaskHandler = WsTaskHandler::from_str(json_task_str, &build_data)
            .expect("Failed to parse Task config");
        let mut mocked_system: MockSystem = MockSystem::new();
        mocked_system
            .expect_check_call()
            .with(mockall::predicate::eq(CallParams {
                cmd_line: vec![
                    "cd",
                    &format!("{}/builds/NA", work_dir.to_string_lossy().to_string()),
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
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            clap::Command::new("bakery"),
            None,
        );
        task.build(
            &cli,
            &build_data,
            &vec![],
            &HashMap::new(),
            false,
            false,
            false,
        )
        .expect("Failed to run task!");
    }

    /*
    Disable it for now we need some more work to get this working
    #[test]
    fn test_ws_task_run_docker() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let path: &Path = temp_dir.path();
        let work_dir: PathBuf = PathBuf::from(path);
        let json_task_str: &str = r#"
        {
            "index": "2",
            "name": "task-name",
            "type": "bitbake",
            "docker": "test-registry/test-image:0.1",
            "recipes": [
                "test-image"
            ]
        }"#;
        let build_data: WsBuildData = Helper::setup_build_data(&work_dir, None, None);
        let task: WsTaskHandler = WsTaskHandler::from_str(json_task_str, &build_data).expect("Failed to parse Task config");
        let mut mocked_system: MockSystem = MockSystem::new();
        mocked_system
            .expect_check_call()
            .with(mockall::predicate::eq(CallParams {
                cmd_line: vec!["docker", "run", "test-registry/test-image:0.1", "cd", &format!("{}/builds/NA", work_dir.to_string_lossy().to_string()), "&&", "bitbake", "test-image"]
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
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            clap::Command::new("bakery"),
            None,
        );
        task.run(&cli, &build_data, &vec![], &HashMap::new(), false, false).expect("Failed to run task!");
    }
    */

    #[test]
    fn test_ws_task_run_recipes() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let path: &Path = temp_dir.path();
        let work_dir: PathBuf = PathBuf::from(path);
        let json_task_str: &str = r#"
        {
            "index": "2",
            "name": "task-name",
            "recipes": [
                "image:sdk",
                "image"
            ]
        }"#;
        let build_data: WsBuildData = Helper::setup_build_data(&work_dir, None, None);
        let task: WsTaskHandler = WsTaskHandler::from_str(json_task_str, &build_data)
            .expect("Failed to parse Task config");
        let mut mocked_system: MockSystem = MockSystem::new();
        mocked_system
            .expect_check_call()
            .with(mockall::predicate::eq(CallParams {
                cmd_line: vec![
                    "cd",
                    &format!("{}/builds/NA", work_dir.to_string_lossy().to_string()),
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
                    &format!("{}/builds/NA", work_dir.to_string_lossy().to_string()),
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
        let cli: Cli = Cli::new(
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            clap::Command::new("bakery"),
            None,
        );
        task.build(
            &cli,
            &build_data,
            &vec![],
            &HashMap::new(),
            false,
            false,
            false,
        )
        .expect("Failed to run task!");
    }

    #[test]
    fn test_ws_task_bitbake_confs() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let path: &Path = temp_dir.path();
        let work_dir: PathBuf = PathBuf::from(path);
        let build_dir: PathBuf = work_dir.join("builds/default");
        let local_conf_path: PathBuf = build_dir.clone().join("conf/local.conf");
        let bblayers_conf_path: PathBuf = build_dir.clone().join("conf/bblayers.conf");
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
                    "BBPATH=\"${TOPDIR}\"",
                    "STRIX_WORKDIR := \"${@os.path.abspath(os.path.dirname(d.getVar('FILE', True)) + '/../../..')}\"",
                    "STRIXOS_LAYER := \"${STRIXWORKDIR}/layers/meta-strix\"",
                    "GNDIR := \"${@os.path.abspath(os.path.join(os.path.dirname(d.getVar('FILE', True)),'../../../../'))}\"",
                    "BBFILES ?= \"\"",
                    "BBLAYERS ?= \" \\",
                    "  ${STRIXOS_LAYER}/meta-strix-raspberrypi \\",
                    "  ${STRIX_WORKDIR}/layers/poky/meta \\",
                    "  ${STRIX_WORKDIR}/layers/poky/meta-poky \\",
                    "  ${STRIX_WORKDIR}/layers/poky/meta-yocto-bsp \\",
                    "  ${STRIX_WORKDIR}/layers/meta-openembedded/meta-oe \\",
                    "  ${STRIX_WORKDIR}/layers/meta-openembedded/meta-networking \\",
                    "  ${STRIX_WORKDIR}/layers/meta-openembedded/meta-filesystems \\",
                    "  ${STRIX_WORKDIR}/layers/meta-openembedded/meta-python \\",
                    "  ${STRIX_WORKDIR}/layers/meta-raspberrypi \""
                ],
                "localconf": [
                    "BB_NUMBER_THREADS ?= \"${@oe.utils.cpu_count()}\"",
                    "PARALLEL_MAKE ?= \"-j ${@oe.utils.cpu_count()}\"",
                    "RM_OLD_IMAGE ?= \"1\"",
                    "INHERIT += \"rm_work\"",
                    "CONF_VERSION = \"1\"",
                    "PACKAGE_CLASSES = \"package_rpm\"",
                    "SDKMACHINE = \"x86_64\"",
                    "USER_CLASSES = \"buildstats image-mklibs image-prelink\"",
                    "PATCHRESOLVE = \"noop\"",
                    "EXTRA_IMAGE_FEATURES = \"debug-tweaks\"",
                    "BB_DISKMON_DIRS = \" \\",
                    "  STOPTASKS,${TMPDIR},1G,100K \\",
                    "  STOPTASKS,${DL_DIR},1G,100K \\",
                    "  STOPTASKS,${SSTATE_DIR},1G,100K \\",
                    "  STOPTASKS,/tmp,100M,100K \\",
                    "  ABORT,${TMPDIR},100M,1K \\",
                    "  ABORT,${DL_DIR},100M,1K \\",
                    "  ABORT,${SSTATE_DIR},100M,1K \\",
                    "  ABORT,/tmp,10M,1K \""
                ]
            }
        }
        "#;
        let json_task_str: &str = r#"
        {
            "index": "2",
            "name": "task-name",
            "recipes": [
                "image"
            ]
        }"#;
        let build_data: WsBuildData =
            Helper::setup_build_data(&work_dir, Some(json_build_config), None);
        let task: WsTaskHandler = WsTaskHandler::from_str(json_task_str, &build_data)
            .expect("Failed to parse Task config");
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
                build_data.bitbake().init_env_file().display()
            )))
            .once()
            .returning(|_x| ());
        mocked_logger
            .expect_info()
            .with(mockall::predicate::eq(format!(
                "execute bitbake build task '{}'",
                task.data().name()
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
            None,
        );
        task.build(
            &cli,
            &build_data,
            &vec![],
            &HashMap::new(),
            true, // Running dry-run should skip the execution and instead only create the bitbake confs
            false,
            false,
        )
        .expect("Failed to run task!");
        let mut local_conf_content: String = String::from("");
        local_conf_content.push_str("BB_NUMBER_THREADS ?= \"${@oe.utils.cpu_count()}\"\n");
        local_conf_content.push_str("PARALLEL_MAKE ?= \"-j ${@oe.utils.cpu_count()}\"\n");
        local_conf_content.push_str("RM_OLD_IMAGE ?= \"1\"\n");
        local_conf_content.push_str("INHERIT += \"rm_work\"\n");
        local_conf_content.push_str("CONF_VERSION = \"1\"\n");
        local_conf_content.push_str("PACKAGE_CLASSES = \"package_rpm\"\n");
        local_conf_content.push_str("SDKMACHINE = \"x86_64\"\n");
        local_conf_content.push_str("USER_CLASSES = \"buildstats image-mklibs image-prelink\"\n");
        local_conf_content.push_str("PATCHRESOLVE = \"noop\"\n");
        local_conf_content.push_str("EXTRA_IMAGE_FEATURES = \"debug-tweaks\"\n");
        local_conf_content.push_str("BB_DISKMON_DIRS = \" \\\n");
        local_conf_content.push_str("STOPTASKS,${TMPDIR},1G,100K \\\n");
        local_conf_content.push_str("STOPTASKS,${DL_DIR},1G,100K \\\n");
        local_conf_content.push_str("STOPTASKS,${SSTATE_DIR},1G,100K \\\n");
        local_conf_content.push_str("STOPTASKS,/tmp,100M,100K \\\n");
        local_conf_content.push_str("ABORT,${TMPDIR},100M,1K \\\n");
        local_conf_content.push_str("ABORT,${DL_DIR},100M,1K \\\n");
        local_conf_content.push_str("ABORT,${SSTATE_DIR},100M,1K \\\n");
        local_conf_content.push_str("ABORT,/tmp,10M,1K \"\n");
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
        let mut bblayers_conf_content: String = String::from("");
        bblayers_conf_content.push_str("LCONF_VERSION=\"7\"\n");
        bblayers_conf_content.push_str("BBPATH=\"${TOPDIR}\"\n");
        bblayers_conf_content.push_str("STRIX_WORKDIR := \"${@os.path.abspath(os.path.dirname(d.getVar('FILE', True)) + '/../../..')}\"\n");
        bblayers_conf_content.push_str("STRIXOS_LAYER := \"${STRIXWORKDIR}/layers/meta-strix\"\n");
        bblayers_conf_content.push_str("GNDIR := \"${@os.path.abspath(os.path.join(os.path.dirname(d.getVar('FILE', True)),'../../../../'))}\"\n");
        bblayers_conf_content.push_str("BBFILES ?= \"\"\n");
        bblayers_conf_content.push_str("BBLAYERS ?= \" \\\n");
        bblayers_conf_content.push_str("  ${STRIXOS_LAYER}/meta-strix-raspberrypi \\\n");
        bblayers_conf_content.push_str("  ${STRIX_WORKDIR}/layers/poky/meta \\\n");
        bblayers_conf_content.push_str("  ${STRIX_WORKDIR}/layers/poky/meta-poky \\\n");
        bblayers_conf_content.push_str("  ${STRIX_WORKDIR}/layers/poky/meta-yocto-bsp \\\n");
        bblayers_conf_content.push_str("  ${STRIX_WORKDIR}/layers/meta-openembedded/meta-oe \\\n");
        bblayers_conf_content
            .push_str("  ${STRIX_WORKDIR}/layers/meta-openembedded/meta-networking \\\n");
        bblayers_conf_content
            .push_str("  ${STRIX_WORKDIR}/layers/meta-openembedded/meta-filesystems \\\n");
        bblayers_conf_content
            .push_str("  ${STRIX_WORKDIR}/layers/meta-openembedded/meta-python \\\n");
        bblayers_conf_content.push_str("  ${STRIX_WORKDIR}/layers/meta-raspberrypi \"\n");
        helper_verify_bitbake_conf(
            &local_conf_path,
            &local_conf_content,
            &bblayers_conf_path,
            &bblayers_conf_content,
        );
    }
}
