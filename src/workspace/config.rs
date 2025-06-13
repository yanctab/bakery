use indexmap::IndexMap;
use serde_json::Value;

use crate::configs::Context;
use crate::data::{WsBuildData, WsContextData};
use crate::error::BError;
use crate::fs::ConfigFileReader;
use crate::workspace::{WsCustomSubCmdHandler, WsSettingsHandler, WsTaskHandler};

pub struct WsBuildConfigHandler {
    data: WsBuildData,
    tasks: IndexMap<String, WsTaskHandler>,
    subcmds: IndexMap<String, WsCustomSubCmdHandler>,
}

impl WsBuildConfigHandler {
    pub fn from_str(json_config: &str, settings: &WsSettingsHandler) -> Result<Self, BError> {
        let data: Value = ConfigFileReader::parse(json_config)?;
        Self::new(&data, settings)
    }

    pub fn new(data: &Value, settings: &WsSettingsHandler) -> Result<Self, BError> {
        let build_data: WsBuildData = WsBuildData::new(data, settings)?;
        let tasks: IndexMap<String, WsTaskHandler> = build_data.get_tasks(data)?;
        let subcmds: IndexMap<String, WsCustomSubCmdHandler> = build_data.get_subcmds(data)?;

        if build_data.version() != "6" {
            return Err(BError::InvalidBuildConfigError(
                build_data.version().to_string(),
            ));
        }

        Ok(WsBuildConfigHandler {
            data: build_data,
            tasks,
            subcmds,
        })
    }

    pub fn build_data(&self) -> &WsBuildData {
        &self.data
    }

    pub fn update_ctx(&mut self, context: &Context) {
        self.data.update_ctx(context);
    }

    pub fn expand_ctx(&mut self) -> Result<(), BError> {
        self.data.expand_ctx()?;
        for (_name, task) in self.tasks.iter_mut() {
            task.expand_ctx(self.data.context().ctx())?;
        }
        for (_name, cmd) in self.subcmds.iter_mut() {
            cmd.expand_ctx(self.data.context().ctx())?;
        }
        Ok(())
    }

    pub fn ctx(&self) -> Result<IndexMap<String, String>, BError> {
        Ok(self.data.context().ctx().variables().clone())
    }

    pub fn task(&self, task: &str) -> Result<&WsTaskHandler, BError> {
        match self.tasks.get(task) {
            Some(config) => {
                return Ok(config);
            }
            None => {
                return Err(BError::ValueError(format!(
                    "Task '{}' does not exists in build config",
                    task
                )));
            }
        }
    }

    pub fn subcmd(&self, cmd: &str) -> Result<&WsCustomSubCmdHandler, BError> {
        match self.subcmds.get(cmd) {
            Some(config) => {
                return Ok(config);
            }
            None => {
                return Err(BError::ValueError(format!(
                    "Sub-command '{}' does not exists in build config",
                    cmd
                )));
            }
        }
    }

    pub fn transfer_tasks(&mut self, tasks: &mut IndexMap<String, WsTaskHandler>) {
        for (key, value) in self.tasks.drain(..) {
            if !tasks.contains_key(&key) {
                tasks.insert(key, value);
            }
        }
    }

    pub fn transfer_subcmds(&mut self, subcmds: &mut IndexMap<String, WsCustomSubCmdHandler>) {
        for (key, value) in self.subcmds.drain(..) {
            if !subcmds.contains_key(&key) {
                subcmds.insert(key.clone(), value);
            } else {
                match subcmds.get(&key) {
                    Some(cmd) => {
                        /*
                         * If the command is the default then we can overwrite it
                         */
                        if cmd.data().cmd()
                            == &format!(
                                "echo \"INFO: currently no '{}' sub-command defined\"",
                                cmd.data().name()
                            )
                        {
                            subcmds.insert(key.clone(), value);
                        }
                    }
                    None => {}
                }
            }
        }
    }

    pub fn merge(&mut self, cfg: &mut WsBuildConfigHandler) {
        cfg.transfer_tasks(&mut self.tasks);
        cfg.transfer_subcmds(&mut self.subcmds);
    }

    pub fn tasks(&self) -> &IndexMap<String, WsTaskHandler> {
        &self.tasks
    }

    pub fn subcmds(&self) -> &IndexMap<String, WsCustomSubCmdHandler> {
        &self.subcmds
    }

    pub fn deploy(&self) -> &WsCustomSubCmdHandler {
        &self
            .subcmd("deploy")
            .expect("Failed to get deploy built-in sub-command")
    }

    pub fn upload(&self) -> &WsCustomSubCmdHandler {
        &self
            .subcmd("upload")
            .expect("Failed to get upload built-in sub-command")
    }

    pub fn setup(&self) -> &WsCustomSubCmdHandler {
        &self
            .subcmd("setup")
            .expect("Failed to get setup built-in sub-command")
    }

    pub fn sync(&self) -> &WsCustomSubCmdHandler {
        &self
            .subcmd("sync")
            .expect("Failed to get sync built-in sub-command")
    }

    pub fn description(&self) -> &str {
        &self.data.product().description()
    }

    //pub fn config_enabled(&self) -> bool {
    //    self.config.enabled()
    //}
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::error::BError;
    use crate::workspace::{
        WsBuildConfigHandler, WsCustomSubCmdHandler, WsSettingsHandler, WsTaskHandler,
    };

    #[test]
    fn test_ws_config_default() {
        let json_settings = r#"
        {
            "version": "6"
        }"#;
        let json_build_config = r#"
        {
            "version": "6",
            "name": "test-name",
            "description": "Test Description",
            "arch": "test-arch"
        }"#;
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let mut ws_settings: WsSettingsHandler =
            WsSettingsHandler::from_str(&work_dir, json_settings, None).unwrap();
        let ws_config: WsBuildConfigHandler =
            WsBuildConfigHandler::from_str(json_build_config, &mut ws_settings)
                .expect("Failed to parse build config");
        assert_eq!(ws_config.build_data().version(), "6".to_string());
        assert_eq!(ws_config.build_data().name(), "test-name".to_string());
        assert_eq!(
            ws_config.build_data().product().name(),
            "test-name".to_string()
        );
        assert_eq!(
            ws_config.build_data().product().arch(),
            "test-arch".to_string()
        );
        assert_eq!(
            ws_config.build_data().product().description(),
            "Test Description".to_string()
        );
        assert_eq!(ws_config.build_data().bitbake().distro(), "NA".to_string());
        assert_eq!(ws_config.build_data().bitbake().machine(), "NA".to_string());
        assert_eq!(
            ws_config.build_data().bitbake().build_dir(),
            PathBuf::from("/workspace/builds/test-name")
        );
        assert_eq!(
            ws_config.build_data().bitbake().build_config_dir(),
            PathBuf::from("/workspace/builds/test-name/conf")
        );
        assert_eq!(
            ws_config.build_data().bitbake().deploy_dir(),
            PathBuf::from("/workspace/builds/test-name/tmp/deploy/images")
        );
        assert_eq!(
            ws_config.build_data().bitbake().dl_dir(),
            PathBuf::from("/workspace/.cache/download")
        );
        assert_eq!(
            ws_config.build_data().bitbake().sstate_dir(),
            PathBuf::from("/workspace/.cache/test-arch/sstate-cache")
        );
        assert_eq!(
            ws_config.build_data().bitbake().bblayers_conf_path(),
            PathBuf::from("/workspace/builds/test-name/conf/bblayers.conf")
        );
        assert!(ws_config.build_data().bitbake().bblayers_conf().is_empty());
        assert_eq!(
            ws_config.build_data().bitbake().local_conf_path(),
            PathBuf::from("/workspace/builds/test-name/conf/local.conf")
        );
        assert!(!ws_config.build_data().bitbake().local_conf().is_empty());
        let mut conf_str: String = String::new();
        conf_str.push_str(&format!(
            "MACHINE ?= \"{}\"\n",
            ws_config.build_data().bitbake().machine()
        ));
        conf_str.push_str(&format!(
            "PRODUCT_NAME ?= \"{}\"\n",
            ws_config.build_data().product().name()
        ));
        conf_str.push_str(&format!(
            "DISTRO ?= \"{}\"\n",
            ws_config.build_data().bitbake().distro()
        ));
        conf_str.push_str(&format!(
            "SSTATE_DIR ?= \"{}\"\n",
            ws_config
                .build_data()
                .bitbake()
                .sstate_dir()
                .to_str()
                .unwrap()
        ));
        conf_str.push_str(&format!(
            "DL_DIR ?= \"{}\"\n",
            ws_config.build_data().bitbake().dl_dir().to_str().unwrap()
        ));
        assert_eq!(ws_config.build_data().bitbake().local_conf(), conf_str);
        assert_eq!(
            ws_config.build_data().bitbake().docker_image(),
            "NA".to_string()
        );
    }

    #[test]
    fn test_ws_config_context_docker() {
        let json_settings = r#"
        {
            "version": "6"
        }"#;
        let json_build_config = r#"
        {
            "version": "6",
            "name": "test-name",
            "description": "Test Description",
            "arch": "test-arch",
            "context": [
                "DOCKER_REGISTRY=test-registry",
                "DOCKER_TAG=0.1",
                "DOCKER_IMAGE=test-image"
            ],
            "bb": {
                "docker": "$#[DOCKER_REGISTRY]/$#[DOCKER_IMAGE]:$#[DOCKER_TAG]"
            }
        }"#;
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let mut ws_settings: WsSettingsHandler =
            WsSettingsHandler::from_str(&work_dir, json_settings, None).unwrap();
        let mut ws_config: WsBuildConfigHandler =
            WsBuildConfigHandler::from_str(json_build_config, &mut ws_settings)
                .expect("Failed to parse build config");
        ws_config.expand_ctx().unwrap();
        assert_eq!(
            ws_config.build_data().bitbake().docker_image(),
            "test-registry/test-image:0.1"
        );
    }

    #[test]
    fn test_ws_config_empty_tasks() {
        let json_settings = r#"
        {
            "version": "6"
        }"#;
        let json_build_config = r#"
        {
            "version": "6",
            "name": "test-name",
            "description": "Test Description",
            "arch": "test-arch"
        }"#;
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let mut ws_settings: WsSettingsHandler =
            WsSettingsHandler::from_str(&work_dir, json_settings, None).unwrap();
        let ws_config: WsBuildConfigHandler =
            WsBuildConfigHandler::from_str(json_build_config, &mut ws_settings)
                .expect("Failed to parse build config");
        assert!(ws_config.tasks().is_empty());
    }

    #[test]
    fn test_ws_task_config_condition() {
        let json_settings = r#"
        {
            "version": "6"
        }"#;
        let json_build_config = r#"
        {
            "version": "6",
            "name": "test-name",
            "description": "Test Description",
            "arch": "test-arch",
            "context": [
                "TASK1_CONDITION=1",
                "TASK2_CONDITION=y",
                "TASK3_CONDITION=Y",
                "TASK4_CONDITION=yes",
                "TASK5_CONDITION=Yes",
                "TASK6_CONDITION=YES",
                "TASK7_CONDITION=True",
                "TASK8_CONDITION=TRUE",
                "TASK9_CONDITION=true"
            ],
            "tasks": {
                "task1": {
                    "index": "1",
                    "name": "task1",
                    "type": "non-bitbake",
                    "condition": "$#[TASK1_CONDITION]"
                },
                "task2": {
                    "index": "2",
                    "name": "task2",
                    "type": "non-bitbake",
                    "condition": "$#[TASK2_CONDITION]"
                },
                "task3": {
                    "index": "3",
                    "name": "task3",
                    "type": "non-bitbake",
                    "condition": "$#[TASK3_CONDITION]"
                },
                "task4": {
                    "index": "4",
                    "name": "task4",
                    "type": "non-bitbake",
                    "condition": "$#[TASK4_CONDITION]"
                },
                "task5": {
                    "index": "5",
                    "name": "task5",
                    "type": "non-bitbake",
                    "condition": "$#[TASK5_CONDITION]"
                },
                "task6": {
                    "index": "6",
                    "name": "task6",
                    "type": "non-bitbake",
                    "condition": "$#[TASK6_CONDITION]"
                },
                "task7": {
                    "index": "7",
                    "name": "task7",
                    "type": "non-bitbake",
                    "condition": "$#[TASK7_CONDITION]"
                },
                "task8": {
                    "index": "8",
                    "name": "task8",
                    "type": "non-bitbake",
                    "condition": "$#[TASK8_CONDITION]"
                },
                "task9": {
                    "index": "9",
                    "name": "task9",
                    "type": "non-bitbake",
                    "condition": "$#[TASK8_CONDITION]"
                }
            }
        }"#;
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let mut ws_settings: WsSettingsHandler =
            WsSettingsHandler::from_str(&work_dir, json_settings, None).unwrap();
        let mut ws_config: WsBuildConfigHandler =
            WsBuildConfigHandler::from_str(json_build_config, &mut ws_settings)
                .expect("Failed to parse build config");
        ws_config.expand_ctx().unwrap();
        for mut i in 1..9 {
            let result: Result<&WsTaskHandler, BError> =
                ws_config.task(format!("task{}", i).as_str());
            match result {
                Ok(task) => {
                    if !task.data().condition() {
                        panic!("Failed to evaluate condition nbr {}", i);
                    }
                }
                Err(e) => {
                    panic!("{}", e.to_string());
                }
            }
            i += 1;
        }
    }

    #[test]
    fn test_ws_task_config_build_dir() {
        let json_settings = r#"
        {
            "version": "6"
        }"#;
        let json_build_config = r#"
        {
            "version": "6",
            "name": "test-name",
            "description": "Test Description",
            "arch": "test-arch",
            "context": [
                "TASK1_BUILD_DIR=task1/build"
            ],
            "tasks": {
                "task1": {
                    "index": "1",
                    "name": "task1",
                    "type": "non-bitbake",
                    "builddir": "$#[TASK1_BUILD_DIR]/dir/"
                },
                "task2": {
                    "index": "2",
                    "name": "task2",
                    "type": "bitbake",
                    "recipes": [
                        "test-image"
                    ]
                }
            }
        }"#;
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let mut ws_settings: WsSettingsHandler =
            WsSettingsHandler::from_str(&work_dir, json_settings, None).unwrap();
        let mut ws_config: WsBuildConfigHandler =
            WsBuildConfigHandler::from_str(json_build_config, &mut ws_settings)
                .expect("Failed to parse build config");
        ws_config.expand_ctx().unwrap();
        assert_eq!(
            ws_config.task("task1").unwrap().data().build_dir(),
            &PathBuf::from("/workspace/task1/build/dir")
        );
        assert_eq!(
            ws_config.task("task2").unwrap().data().build_dir(),
            &PathBuf::from("/workspace/builds/test-name")
        );
    }

    #[test]
    fn test_ws_config_context_task_build_dir() {
        let json_settings = r#"
        {
            "version": "6"
        }"#;
        let json_build_config = r#"
        {
            "version": "6",
            "name": "test-name",
            "description": "Test Description",
            "arch": "test-arch",
            "context": [
                "TASK1_BUILD_DIR=task1/build/dir"
            ],
            "tasks": {
                "task1": {
                    "index": "0",
                    "name": "task1-name",
                    "type": "non-bitbake",
                    "builddir": "test/$#[TASK1_BUILD_DIR]",
                    "build": "build-cmd",
                    "clean": "clean-cmd",
                    "artifacts": []
                },
                "task2": {
                    "index": "1",
                    "name": "task2-name",
                    "type": "bitbake",
                    "recipes": [
                        "image-recipe"
                    ],
                    "artifacts": []
                }
            }
        }"#;
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let mut ws_settings: WsSettingsHandler =
            WsSettingsHandler::from_str(&work_dir, json_settings, None).unwrap();
        let mut ws_config: WsBuildConfigHandler =
            WsBuildConfigHandler::from_str(json_build_config, &mut ws_settings)
                .expect("Failed to parse build config");
        ws_config.expand_ctx().unwrap();
        assert_eq!(
            ws_config.task("task1").unwrap().data().build_dir(),
            &PathBuf::from("/workspace/test/task1/build/dir")
        );
        assert_eq!(
            ws_config.task("task2").unwrap().data().build_dir(),
            &PathBuf::from("/workspace/builds/test-name")
        );
        {
            let result: Result<&WsTaskHandler, BError> = ws_config.task("task3");
            match result {
                Ok(_task) => {
                    panic!("We should have recived an error because we have no task3 defined!");
                }
                Err(e) => {
                    assert_eq!(
                        e.to_string(),
                        "Task 'task3' does not exists in build config".to_string()
                    );
                }
            }
        }
    }

    #[test]
    fn test_ws_config_tasks() {
        let json_settings = r#"
        {
            "version": "6"
        }"#;
        let json_build_config = r#"
        {
            "version": "6",
            "name": "test-name",
            "description": "Test Description",
            "arch": "test-arch",
            "tasks": {
                "task0": {
                    "index": "0",
                    "name": "task0",
                    "type": "non-bitbake",
                    "builddir": "test/task0",
                    "build": "cmd0",
                    "clean": "clean0",
                    "artifacts": [
                        {
                            "source": "test/file0-1.txt"
                        }
                    ]
                },
                "task1": {
                    "index": "1",
                    "name": "task1",
                    "type": "non-bitbake",
                    "builddir": "test/task1",
                    "build": "cmd1",
                    "clean": "clean1",
                    "artifacts": [
                        {
                            "source": "test/file1-1.txt"
                        }
                    ]
                },
                "task2": {
                    "index": "2",
                    "name": "task2",
                    "type": "non-bitbake",
                    "builddir": "test/task2",
                    "build": "cmd2",
                    "clean": "clean2",
                    "artifacts": [
                        {
                            "source": "test/file2-1.txt"
                        }
                    ]
                }
            }
        }"#;
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let mut ws_settings: WsSettingsHandler =
            WsSettingsHandler::from_str(&work_dir, json_settings, None).unwrap();
        let ws_config: WsBuildConfigHandler =
            WsBuildConfigHandler::from_str(json_build_config, &mut ws_settings)
                .expect("Failed to parse build config");

        let mut i: i32 = 0;
        ws_config.tasks().iter().for_each(|(name, task)| {
            assert_eq!(name, &format!("task{}", i));
            assert_eq!(
                task.data().build_dir(),
                &PathBuf::from(format!("/workspace/test/task{}", i))
            );
            assert_eq!(task.data().build_cmd(), &format!("cmd{}", i));
            assert_eq!(task.data().clean_cmd(), &format!("clean{}", i));
            task.artifacts().iter().for_each(|a| {
                assert_eq!(a.data().source(), &format!("test/file{}-1.txt", i));
            });
            i += 1;
        });
    }

    #[test]
    fn test_ws_config_incompatible_version() {
        let json_settings = r#"
        {
            "version": "5"
        }"#;
        let json_build_config = r#"
        {
            "version": "5",
            "name": "test-name",
            "description": "Test Description",
            "arch": "test-arch"
        }"#;
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let mut ws_settings: WsSettingsHandler =
            WsSettingsHandler::from_str(&work_dir, json_settings, None).unwrap();
        let result: Result<WsBuildConfigHandler, BError> =
            WsBuildConfigHandler::from_str(json_build_config, &mut ws_settings);
        match result {
            Ok(_cfg) => {
                assert!(false, "Expected an error");
            }
            Err(err) => {
                assert_eq!(
                    "The build config version '5' is not compatible with current bakery version. \
                    Update config to match the format of version '6'",
                    err.to_string()
                );
            }
        }
    }

    #[test]
    fn test_ws_config_merge() {
        let json_settings = r#"
        {
            "version": "6"
        }"#;
        let json_main_build_config = r#"
        {
            "version": "6",
            "name": "main",
            "description": "Test Description",
            "arch": "test-arch",
            "tasks": {
                "task0": {
                    "index": "0",
                    "name": "task0",
                    "type": "non-bitbake",
                    "builddir": "test/main",
                    "build": "main",
                    "clean": "main",
                    "artifacts": [
                        {
                            "source": "test/main-file.txt"
                        }
                    ]
                }
            },
            "setup": {
                "cmd": "main"
            }
        }"#;
        let json_include_config1 = r#"
        {
            "version": "6",
            "tasks": {
                "task0": {
                    "index": "0",
                    "name": "task0",
                    "type": "non-bitbake",
                    "builddir": "test/config1",
                    "build": "config1",
                    "clean": "config1",
                    "artifacts": [
                        {
                            "source": "test/config.txt"
                        }
                    ]
                },
                "task1": {
                    "index": "1",
                    "name": "task1",
                    "type": "non-bitbake",
                    "builddir": "test/config1",
                    "build": "config1",
                    "clean": "config1",
                    "artifacts": [
                        {
                            "source": "test/config.txt"
                        }
                    ]
                }
            },
            "setup": {
                "cmd": "config1"
            },
            "sync": {
                "cmd": "config1"
            }
        }"#;
        let json_include_config2 = r#"
        {
            "version": "6",
            "tasks": {
                "task2": {
                    "index": "2",
                    "name": "task2",
                    "type": "non-bitbake",
                    "builddir": "test/config2",
                    "build": "config2",
                    "clean": "config2",
                    "artifacts": [
                        {
                            "source": "test/config.txt"
                        }
                    ]
                }
            },
            "upload": {
                "cmd": "config2"
            }
        }"#;
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let mut ws_settings: WsSettingsHandler =
            WsSettingsHandler::from_str(&work_dir, json_settings, None).unwrap();
        let mut ws_main_config: WsBuildConfigHandler =
            WsBuildConfigHandler::from_str(json_main_build_config, &mut ws_settings)
                .expect("Failed to parse build config");
        let mut ws_include_config1: WsBuildConfigHandler =
            WsBuildConfigHandler::from_str(json_include_config1, &mut ws_settings)
                .expect("Failed to parse build config");
        let mut ws_include_config2: WsBuildConfigHandler =
            WsBuildConfigHandler::from_str(json_include_config2, &mut ws_settings)
                .expect("Failed to parse build config");
        ws_main_config.merge(&mut ws_include_config1);
        ws_main_config.merge(&mut ws_include_config2);
        let t0: &WsTaskHandler = ws_main_config.tasks().get("task0").unwrap();
        assert_eq!(t0.data().build_cmd(), "main");
        let t1: &WsTaskHandler = ws_main_config.tasks().get("task1").unwrap();
        assert_eq!(t1.data().build_cmd(), "config1");
        let t2: &WsTaskHandler = ws_main_config.tasks().get("task2").unwrap();
        assert_eq!(t2.data().build_cmd(), "config2");
        let setup: &WsCustomSubCmdHandler = ws_main_config.subcmds().get("setup").unwrap();
        assert_eq!(setup.data().cmd(), "main");
        let sync: &WsCustomSubCmdHandler = ws_main_config.subcmds().get("sync").unwrap();
        assert_eq!(sync.data().cmd(), "config1");
        let upload: &WsCustomSubCmdHandler = ws_main_config.subcmds().get("upload").unwrap();
        assert_eq!(upload.data().cmd(), "config2");
    }
}
