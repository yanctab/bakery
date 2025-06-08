use chrono;
use indexmap::{indexmap, IndexMap};
use serde_json::Value;
use std::path::PathBuf;

use crate::configs::Context;
use crate::data::context;
use crate::data::{WsBitbakeData, WsConfigData, WsContextData, WsIncludeData, WsProductData};
use crate::error::BError;
use crate::fs::ConfigFileReader;
use crate::workspace::{
    WsArtifactsHandler, WsCustomSubCmdHandler, WsSettingsHandler, WsTaskHandler,
};

pub struct WsBuildData {
    data: Value,
    config: WsConfigData,
    product: WsProductData,
    bitbake: WsBitbakeData,
    include: WsIncludeData,
    context: WsContextData,
    settings: WsSettingsHandler,
}

impl WsBuildData {
    fn get_task(&self, data: &Value) -> Result<WsTaskHandler, BError> {
        WsTaskHandler::new(data, &self)
    }

    fn get_artifact(
        &self,
        data: &Value,
        task_build_dir: &PathBuf,
    ) -> Result<WsArtifactsHandler, BError> {
        WsArtifactsHandler::new(data, task_build_dir, &self)
    }

    pub fn from_str(json_config: &str, settings: &WsSettingsHandler) -> Result<Self, BError> {
        let data: Value = ConfigFileReader::parse(json_config)?;
        Self::new(&data, settings)
    }

    pub fn new(data: &Value, settings: &WsSettingsHandler) -> Result<Self, BError> {
        // Parse the individual segments of the build config

        // The config segments contains data related with the actual config
        // like version and configuration name which normally is the same
        // as the product name but that might change in the future
        let config: WsConfigData = WsConfigData::from_value(data)?;
        // The product segments contains product specific data such as
        // product name and arch
        let product: WsProductData = WsProductData::from_value(data)?;
        // The bitbake segment contains all the bitbake related data
        // needed when executing a bitbake task defined in the build
        // config
        let bitbake: WsBitbakeData = WsBitbakeData::from_value(data, settings)?;
        // The include segment is to define additional json files that contains
        // defined tasks that are used by multiple build configs
        let include: WsIncludeData = WsIncludeData::from_value(data, settings)?;
        // The context segment contains all the context variables used
        // by other parts of the build config
        let mut context: WsContextData = WsContextData::from_value(data)?;

        // Setup context with "built-in" variables that will always
        // be available
        // TODO: use local to get the date and time format correct
        let ctx_built_in_variables: IndexMap<String, String> = indexmap! {
            context::CTX_KEY_MACHINE.to_string() => bitbake.machine().to_string(),
            context::CTX_KEY_ARCH.to_string() => product.arch().to_string(),
            context::CTX_KEY_DISTRO.to_string() => bitbake.distro().to_string(),
            context::CTX_KEY_NAME.to_string() => product.name().to_string(),
            context::CTX_KEY_CONFIG_NAME.to_string() => product.name().to_string(),
            context::CTX_KEY_CONFIG.to_string() => product.name().to_string(),
            context::CTX_KEY_PRODUCT_NAME.to_string() => product.product().to_string(),
            context::CTX_KEY_PROJECT_NAME.to_string() => product.project().to_string(),
            context::CTX_KEY_ARTIFACTS_DIR.to_string() => settings.artifacts_dir().to_string_lossy().to_string(),
            context::CTX_KEY_LAYERS_DIR.to_string() => settings.layers_dir().to_string_lossy().to_string(),
            context::CTX_KEY_SCRIPTS_DIR.to_string() => settings.scripts_dir().to_string_lossy().to_string(),
            context::CTX_KEY_BUILDS_DIR.to_string() => settings.builds_dir().to_string_lossy().to_string(),
            context::CTX_KEY_WORK_DIR.to_string() => settings.work_dir().to_string_lossy().to_string(),
            context::CTX_KEY_DATE.to_string() => chrono::offset::Local::now().format("%Y-%m-%d").to_string(),
            context::CTX_KEY_TIME.to_string() => chrono::offset::Local::now().format("%H:%M").to_string(),
        };

        context.update(&ctx_built_in_variables);
        // Update the "built-in" bitbake paths in the context variables
        let bb_build_dir: PathBuf = settings
            .builds_dir()
            .clone()
            .join(PathBuf::from(product.name().to_string()));
        let bb_deploy_dir: PathBuf = bb_build_dir
            .clone()
            .join(PathBuf::from(bitbake.deploy_dir().clone()));
        let ctx_bitbake_variables: IndexMap<String, String> = indexmap! {
            context::CTX_KEY_BB_BUILD_DIR.to_string() => bb_build_dir.to_string_lossy().to_string(),
            context::CTX_KEY_BB_DEPLOY_DIR.to_string() => bb_deploy_dir.to_string_lossy().to_string(),
        };
        context.update(&ctx_bitbake_variables);

        Ok(WsBuildData {
            data: data.to_owned(),
            config,
            product,
            bitbake,
            include,
            context,
            settings: settings.clone(), // for now lets clone it
        })
    }

    pub fn get_artifacts(
        &self,
        data: &Value,
        task_build_dir: &PathBuf,
    ) -> Result<Vec<WsArtifactsHandler>, BError> {
        match data.get("artifacts") {
            Some(value) => {
                if value.is_array() {
                    if let Some(artifact_vec) = value.as_array() {
                        let mut artifacts: Vec<WsArtifactsHandler> = Vec::new();
                        for artifact_data in artifact_vec.iter() {
                            let artifact: WsArtifactsHandler =
                                self.get_artifact(artifact_data, task_build_dir)?;
                            artifacts.push(artifact);
                        }
                        return Ok(artifacts);
                    }
                    return Err(BError::ParseArtifactsError(
                        "Invalid 'artifacts' node in build config".to_string(),
                    ));
                } else {
                    return Err(BError::ParseArtifactsError(
                        "No 'artifacts' array node found in build config".to_string(),
                    ));
                }
            }
            None => {
                return Ok(Vec::new());
            }
        }
    }

    pub fn get_tasks(&self, data: &Value) -> Result<IndexMap<String, WsTaskHandler>, BError> {
        match data.get("tasks") {
            Some(value) => {
                if value.is_object() {
                    if let Some(task_map) = value.as_object() {
                        let mut tasks_vec: Vec<(usize, String, WsTaskHandler)> = vec![];

                        for (name, data) in task_map.iter() {
                            let task: WsTaskHandler = self.get_task(data)?;
                            if let Some(index) = data
                                .get("index")
                                .and_then(|v| v.as_str())
                                .and_then(|s| s.parse::<usize>().ok())
                            {
                                tasks_vec.push((index, name.clone(), task));
                            } else {
                                return Err(BError::ParseError(
                                    "Missing or invalid index".to_string(),
                                ));
                            }
                        }

                        // Sort by index
                        tasks_vec.sort_by_key(|(index, _, _)| *index);

                        // Insert in sorted order
                        let mut tasks: IndexMap<String, WsTaskHandler> = IndexMap::new();
                        for (_, name, task) in tasks_vec {
                            tasks.insert(name, task);
                        }

                        return Ok(tasks);
                    }
                    return Err(BError::ParseTasksError(
                        "Invalid 'task' format in build config".to_string(),
                    ));
                } else {
                    return Err(BError::ParseTasksError(
                        "No 'tasks' node found in build config".to_string(),
                    ));
                }
            }
            None => {
                return Ok(IndexMap::new());
            }
        }
    }

    pub fn get_subcmds(
        &self,
        data: &Value,
    ) -> Result<IndexMap<String, WsCustomSubCmdHandler>, BError> {
        let names = ["deploy", "upload", "setup", "sync"];
        let subcmds: IndexMap<String, WsCustomSubCmdHandler> = names
            .iter()
            .map(|&cmd| {
                let subcmd: WsCustomSubCmdHandler = WsCustomSubCmdHandler::new(cmd, data)?;
                Ok((cmd.to_owned(), subcmd))
            })
            .collect::<Result<IndexMap<_, _>, BError>>()?;
        Ok(subcmds)
    }

    pub fn included_configs(&self) -> Vec<PathBuf> {
        self.include.configs().clone()
    }

    pub fn name(&self) -> &str {
        self.config.name()
    }

    pub fn version(&self) -> &str {
        self.config.version()
    }

    pub fn valid(&self) -> bool {
        return self.config.version() != "NA"
            && self.product().name() != "NA"
            && self.product().description() != "NA";
    }

    pub fn settings(&self) -> &WsSettingsHandler {
        &self.settings
    }

    pub fn context(&self) -> &WsContextData {
        &self.context
    }

    pub fn update_ctx(&mut self, context: &Context) {
        self.context.update_ctx(context);
    }

    pub fn expand_ctx(&mut self) -> Result<(), BError> {
        self.config.expand_ctx(self.context.ctx())?;
        self.product.expand_ctx(self.context.ctx())?;
        self.settings.expand_ctx(self.context.ctx())?;
        self.bitbake.expand_ctx(self.context.ctx())?;
        Ok(())
    }

    pub fn product(&self) -> &WsProductData {
        &self.product
    }

    pub fn bitbake(&self) -> &WsBitbakeData {
        &self.bitbake
    }
}

#[cfg(test)]
mod tests {
    use chrono;
    use indexmap::IndexMap;
    use serde_json::Value;
    use std::path::PathBuf;

    use crate::data::{AType, WsBitbakeData, WsBuildData};
    use crate::error::BError;
    use crate::fs::ConfigFileReader;
    use crate::helper::Helper;
    use crate::workspace::{WsArtifactsHandler, WsTaskHandler};

    #[test]
    fn test_ws_build_data_default() {
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let data: WsBuildData = Helper::setup_build_data(&work_dir, None, None);
        assert_eq!(data.version(), "6");
        assert_eq!(data.name(), "NA");
    }

    #[test]
    fn test_ws_build_data_no_tasks() {
        let json_build_config = r#"
        {
            "version": "6",
            "name": "test-name",
            "description": "Test Description",
            "arch": "test-arch"
        }"#;
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let data: WsBuildData = Helper::setup_build_data(&work_dir, Some(json_build_config), None);
        let json_data: Value =
            ConfigFileReader::parse(json_build_config).expect("Failed to parse json");
        let tasks: IndexMap<String, WsTaskHandler> =
            data.get_tasks(&json_data).expect("Failed to parse tasks");
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_ws_build_data_tasks_error() {
        let json_build_config: &str = r#"
        {
            "version": "6",
            "name": "test-name",
            "description": "Test Description",
            "arch": "test-arch",
            "tasks": "error"
        }"#;
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let data: WsBuildData = Helper::setup_build_data(&work_dir, Some(json_build_config), None);
        let json_data: Value =
            ConfigFileReader::parse(json_build_config).expect("Failed to parse json");
        let result: Result<IndexMap<String, WsTaskHandler>, BError> = data.get_tasks(&json_data);
        match result {
            Ok(_rconfig) => {
                panic!("We should have recived an error because we have no valid config!");
            }
            Err(e) => {
                assert_eq!(
                    e.to_string(),
                    String::from("Invalid 'task' node in build config. No 'tasks' node found in build config")
                );
            }
        }
    }

    #[test]
    fn test_ws_build_data_task() {
        let json_task_str: &str = r#"
        {
            "index": "2",
            "name": "task-name",
            "type": "bitbake",
            "recipes": [
                "test-image"
            ]
        }"#;
        let json_build_config: &str = r#"
        {
            "version": "6",
            "name": "test-name",
            "description": "Test Description",
            "arch": "test-arch"
        }"#;
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let data: WsBuildData = Helper::setup_build_data(&work_dir, Some(json_build_config), None);
        let json_data: Value =
            ConfigFileReader::parse(json_task_str).expect("Failed to parse json");
        let task: WsTaskHandler = data.get_task(&json_data).expect("Failed to parse task");
        assert_eq!(
            task.data().build_dir(),
            &PathBuf::from("/workspace/builds/test-name")
        );
        assert_eq!(task.data().name(), "task-name");
    }

    #[test]
    fn test_ws_build_data_task_expand_ctx() {
        let json_task_str: &str = r#"
        {
            "index": "2",
            "name": "task-name",
            "type": "bitbake",
            "recipes": [
                "$#[RECIPE_NAME]"
            ]
        }"#;
        let json_build_config: &str = r#"
        {
            "version": "6",
            "name": "test-name",
            "description": "Test Description",
            "arch": "test-arch",
            "context": [
                "RECIPE_NAME=test-image"
            ]
        }"#;
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let data: WsBuildData = Helper::setup_build_data(&work_dir, Some(json_build_config), None);
        let json_data: Value =
            ConfigFileReader::parse(json_task_str).expect("Failed to parse json");
        let mut task: WsTaskHandler = data.get_task(&json_data).expect("Failed to parse task");
        task.expand_ctx(data.context().ctx()).unwrap();
        assert_eq!(task.data().recipes(), &vec!["test-image"]);
    }

    #[test]
    fn test_ws_build_tasks() {
        let json_build_config: &str = r#"
        {
            "version": "6",
            "name": "test-name",
            "description": "Test Description",
            "arch": "test-arch",
            "tasks": {
                "task1": {
                    "index": "1",
                    "name": "task1",
                    "type": "non-bitbake"
                },
                "task2": {
                    "index": "2",
                    "name": "task2",
                    "type": "non-bitbake"
                }
            }
        }"#;
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let data: WsBuildData = Helper::setup_build_data(&work_dir, Some(json_build_config), None);
        let json_data: Value =
            ConfigFileReader::parse(json_build_config).expect("Failed to parse json");
        let tasks: IndexMap<String, WsTaskHandler> =
            data.get_tasks(&json_data).expect("Failed to parse tasks");
        assert!(!tasks.is_empty());
        let mut i: usize = 1;
        tasks.iter().for_each(|(name, task)| {
            assert_eq!(name, &format!("task{}", i));
            assert_eq!(task.data().name(), &format!("task{}", i));
            i += 1;
        });
    }

    #[test]
    fn test_ws_build_tasks_order() {
        let json_build_config: &str = r#"
        {
            "version": "6",
            "name": "test-name",
            "description": "Test Description",
            "arch": "test-arch",
            "tasks": {
                "btask": {
                    "index": "1",
                    "name": "btask",
                    "type": "non-bitbake"
                },
                "atask": {
                    "index": "2",
                    "name": "atask",
                    "type": "non-bitbake"
                }
            }
        }"#;
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let data: WsBuildData = Helper::setup_build_data(&work_dir, Some(json_build_config), None);
        let json_data: Value =
            ConfigFileReader::parse(json_build_config).expect("Failed to parse json");
        let tasks: IndexMap<String, WsTaskHandler> =
            data.get_tasks(&json_data).expect("Failed to parse tasks");
        assert!(!tasks.is_empty());
        let (name_0, task_0) = tasks.get_index(0).unwrap();
        assert_eq!(name_0.as_str(), "btask");
        assert_eq!(task_0.data().name(), "btask");
        let (name_1, task_1) = tasks.get_index(1).unwrap();
        assert_eq!(name_1.as_str(), "atask");
        assert_eq!(task_1.data().name(), "atask");
    }

    #[test]
    fn test_ws_build_data_artifacts_error() {
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let task_build_dir: PathBuf = work_dir.clone().join("task/dir");
        let json_artifact_config: &str = r#"
        {
            "index": "2",
            "name": "task2",
            "type": "non-bitbake",
            "artifacts": "error"
        }"#;
        let data: WsBuildData = Helper::setup_build_data(&work_dir, None, None);
        let json_data: Value =
            ConfigFileReader::parse(json_artifact_config).expect("Failed to parse json");
        let result: Result<Vec<WsArtifactsHandler>, BError> =
            data.get_artifacts(&json_data, &task_build_dir);
        match result {
            Ok(_rconfig) => {
                panic!("We should have recived an error because we have no valid config!");
            }
            Err(e) => {
                assert_eq!(
                    e.to_string(),
                    String::from("Invalid 'artifact' node in build config. No 'artifacts' array node found in build config")
                );
            }
        }
    }

    #[test]
    fn test_ws_build_data_artifact() {
        let json_artifact_config: &str = r#"
        {
            "type": "manifest",
            "name": "test-manifest",
            "content": {
                "key1": "value1",
                "key2": "value2",
                "data": {
                    "key3": "value3"
                }
            }
        }"#;
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let task_build_dir: PathBuf = work_dir.clone().join("task/dir");
        let data: WsBuildData = Helper::setup_build_data(&work_dir, None, None);
        let json_data: Value =
            ConfigFileReader::parse(json_artifact_config).expect("Failed to parse json");
        let artifacts: WsArtifactsHandler = data
            .get_artifact(&json_data, &task_build_dir)
            .expect("Failed to parse artifacts");
        assert_eq!(artifacts.data().atype(), &AType::Manifest);
        assert_eq!(artifacts.data().name(), "test-manifest");
    }

    #[test]
    fn test_ws_build_data_expand_artifact() {
        let json_build_config: &str = r#"
        {
            "version": "6",
            "name": "test-name",
            "description": "Test Description",
            "arch": "test-arch",
            "context": [
                "MANIFEST_FILE_NAME=test-manifest"
            ]
        }"#;
        let json_artifact_config: &str = r#"
        {
            "type": "manifest",
            "name": "$#[MANIFEST_FILE_NAME]",
            "content": {
                "key1": "value1",
                "key2": "value2",
                "data": {
                    "key3": "value3"
                }
            }
        }"#;
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let task_build_dir: PathBuf = work_dir.clone().join("task/dir");
        let data: WsBuildData = Helper::setup_build_data(&work_dir, Some(json_build_config), None);
        let json_data: Value =
            ConfigFileReader::parse(json_artifact_config).expect("Failed to parse json");
        let mut artifact: WsArtifactsHandler = data
            .get_artifact(&json_data, &task_build_dir)
            .expect("Failed to parse artifacts");
        artifact.expand_ctx(data.context().ctx()).unwrap();
        assert_eq!(artifact.data().atype(), &AType::Manifest);
        assert_eq!(artifact.data().name(), "test-manifest");
    }

    #[test]
    fn test_ws_build_data_time_date() {
        let json_artifact_config: &str = r#"
        {
            "type": "manifest",
            "name": "test-manifest",
            "content": {
                "date": "$#[BKRY_DATE]",
                "time": "$#[BKRY_TIME]"
            }
        }"#;
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let task_build_dir: PathBuf = work_dir.clone().join("task/dir");
        let data: WsBuildData = Helper::setup_build_data(&work_dir, None, None);
        let json_data: Value =
            ConfigFileReader::parse(json_artifact_config).expect("Failed to parse json");
        let mut artifact: WsArtifactsHandler = data
            .get_artifact(&json_data, &task_build_dir)
            .expect("Failed to parse artifacts");
        artifact.expand_ctx(data.context().ctx()).unwrap();
        assert_eq!(artifact.data().atype(), &AType::Manifest);
        assert_eq!(artifact.data().name(), "test-manifest");
        assert_eq!(
            artifact.data().manifest(),
            format!(
                "{{\"date\":\"{}\",\"time\":\"{}\"}}",
                chrono::offset::Local::now().format("%Y-%m-%d").to_string(),
                chrono::offset::Local::now().format("%H:%M").to_string()
            )
        );
    }

    #[test]
    fn test_ws_build_data_artifacts() {
        let json_artifacts_config: &str = r#"
        {
            "artifacts": [
                {
                    "source": "file1.txt",
                    "dest": "file1.txt"
                },
                {
                    "source": "file2.txt",
                    "dest": "file2.txt"
                }
            ]
        }"#;
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let task_build_dir: PathBuf = work_dir.clone().join("task/dir");
        let data: WsBuildData = Helper::setup_build_data(&work_dir, None, None);
        let json_data: Value =
            ConfigFileReader::parse(json_artifacts_config).expect("Failed to parse json");
        let artifacts: Vec<WsArtifactsHandler> = data
            .get_artifacts(&json_data, &task_build_dir)
            .expect("Failed to parse artifacts");
        assert!(!artifacts.is_empty());
        let mut i: usize = 1;
        artifacts.iter().for_each(|a| {
            assert_eq!(a.data().atype(), &AType::File);
            assert_eq!(a.data().source(), &format!("file{}.txt", i));
            i += 1;
        });
    }

    #[test]
    fn test_ws_build_data_built_in_ctx() {
        let json_task_str: &str = r#"
        {
            "index": "2",
            "name": "task-name",
            "type": "bitbake",
            "recipes": [
                "$#[RECIPE_NAME]"
            ]
        }"#;
        let json_build_config: &str = r#"
        {
            "version": "6",
            "name": "test-name",
            "description": "Test Description",
            "arch": "test-arch",
            "bb": {
                "machine": "test-machine",
                "distro": "test-distro",
                "deploydir": "tmp/test/deploy",
                "docker": "test-registry/test-image:0.1",
                "initenv": "$#[BKRY_LAYERS_DIR]/meta-test/oe-my-init-env",
                "bblayersconf": [
                    "BAKERY_WORKDIR=\"${TOPDIR}/../..\"",
                    "BBLAYERS ?= \" \\",
                    "       $#[BKRY_LAYERS_DIR]/meta-test \\",
                    "       $#[BKRY_BUILDS_DIR]/workspace \\",
                    "\""
                ],
                "localconf": [
                    "ARTIFACTS_DIR ?= $#[BKRY_ARTIFACTS_DIR]",
                    "LAYERS_DIR ?= $#[BKRY_LAYERS_DIR]",
                    "SCRIPTS_DIR ?= $#[BKRY_SCRIPTS_DIR]",
                    "BUILDS_DIR ?= $#[BKRY_BUILDS_DIR]",
                    "WORK_DIR ?= $#[BKRY_WORK_DIR]"
                ]
            }

        }"#;
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let mut data: WsBuildData =
            Helper::setup_build_data(&work_dir, Some(json_build_config), None);
        data.expand_ctx().unwrap();
        let bitbake: &WsBitbakeData = data.bitbake();
        assert_eq!(bitbake.local_conf(), "ARTIFACTS_DIR ?= /workspace/artifacts\nLAYERS_DIR ?= /workspace/layers\nSCRIPTS_DIR ?= /workspace/scripts\nBUILDS_DIR ?= /workspace/builds\nWORK_DIR ?= /workspace\nMACHINE ?= \"test-machine\"\nPRODUCT_NAME ?= \"test-name\"\nDISTRO ?= \"test-distro\"\nSSTATE_DIR ?= \"/workspace/.cache/test-arch/sstate-cache\"\nDL_DIR ?= \"/workspace/.cache/download\"\n");
        assert_eq!(bitbake.bblayers_conf(), "BAKERY_WORKDIR=\"${TOPDIR}/../..\"\nBBLAYERS ?= \" \\\n       /workspace/layers/meta-test \\\n       /workspace/builds/workspace \\\n\"\n");
        assert_eq!(
            bitbake.init_env_file(),
            PathBuf::from("/workspace/layers/meta-test/oe-my-init-env")
        );
    }
}
