use indexmap::indexmap;
use std::path::PathBuf;

use crate::data::WsContextData;
use crate::error::BError;
use crate::fs::ConfigFileReader;
use crate::workspace::{WsBuildConfigHandler, WsSettingsHandler};

const WORKSPACE_SETTINGS: &str = "workspace.json";

pub struct WsConfigFileHandler {
    work_dir: PathBuf,
    bkry_home_cfg_dir: PathBuf,
    bkry_cfg_dir: PathBuf,
}

impl WsConfigFileHandler {
    fn _load_settings_from_path(&self, path: &PathBuf) -> Result<WsSettingsHandler, BError> {
        let settings_str: String = ConfigFileReader::new(path).read_json()?;
        let mut settings: WsSettingsHandler =
            WsSettingsHandler::from_str(&self.work_dir, &settings_str, Some(path.clone()))?;

        // Create a context with default values and expand the settings
        let context: WsContextData = WsContextData::new(&indexmap! {})?;
        settings.expand_ctx(context.ctx())?;

        Ok(settings)
    }

    pub fn new(work_dir: &PathBuf, home_dir: &PathBuf) -> Self {
        let bkry_home_cfg_dir: PathBuf = home_dir.clone().join(".bakery");
        let bkry_cfg_dir: PathBuf = PathBuf::from("/etc/bakery");
        WsConfigFileHandler {
            work_dir: work_dir.clone(),
            bkry_home_cfg_dir,
            bkry_cfg_dir,
        }
    }

    pub fn ws_settings(&self) -> Result<WsSettingsHandler, BError> {
        let paths: Vec<PathBuf> = vec![
            self.work_dir.join(WORKSPACE_SETTINGS),          // First
            self.bkry_home_cfg_dir.join(WORKSPACE_SETTINGS), // Second
            self.bkry_cfg_dir.join(WORKSPACE_SETTINGS),      // Third
        ];

        /*
         * Iterate over current work/workspace dir, ~/.bakery, /etc/bakery
         */
        for path in paths {
            if path.exists() {
                // Load the setting from the first existing workspace.json found
                return self._load_settings_from_path(&path);
            }
        }

        /*
         * Return default settings the only thing required is the version the rest
         * be defined by the settings handler if it is not defined in the json
         */
        let default_settings: &str = r#"
        {
            "version": "6"
        }"#;
        return WsSettingsHandler::from_str(&self.work_dir, default_settings, None);
    }

    fn config_header(&self, config: &WsBuildConfigHandler) -> String {
        let cfg_bitbake_json: String = config.build_data().bitbake().to_string();
        let cfg_product_json: String = config.build_data().product().to_string();
        let cfg_header_json: String = format!("{},{}", cfg_product_json, cfg_bitbake_json);
        cfg_header_json.clone()
    }

    pub fn verify_ws(&self) -> Result<(), BError> {
        /*
         * The search order for the workspace settings is:
         *
         * 1. Current working directory
         * 2. ~/.bakery/
         * 3. /etc/bakery/
         *
         * If none of these contain 'workspace.json', return an invalid workspace error.
         */
        if !self.work_dir.clone().join(WORKSPACE_SETTINGS).exists()
            && !self
                .bkry_home_cfg_dir
                .clone()
                .join(WORKSPACE_SETTINGS)
                .exists()
            && !self.bkry_cfg_dir.clone().join(WORKSPACE_SETTINGS).exists()
        {
            return Err(BError::InvalidWorkspaceError());
        }

        Ok(())
    }

    pub fn setup_build_config(
        &self,
        path: &PathBuf,
        settings: &WsSettingsHandler,
    ) -> Result<WsBuildConfigHandler, BError> {
        let build_config_json: String = ConfigFileReader::new(&path).read_json()?;
        let mut main_config: WsBuildConfigHandler =
            WsBuildConfigHandler::from_str(&build_config_json, settings)?;
        let cfg_header_json: String = self.config_header(&main_config);

        /*
         * Iterate over any included build config and extend the main build config with the included
         * build configs. Currently the included build configs will only extend the main config with
         * the tasks and any of the built-in sub-commands sync, setup, upload, deploy
         */
        for config in main_config.build_data().included_configs().iter() {
            let cfg_include_json: String = ConfigFileReader::new(config).read_json()?;
            /*
             * The included build config does not and should not contain anything but the tasks and custom sub commands but because
             * each task is handling it's own build dir which is setup by the bb segment we need to inject the bb to the WsBuildConfigHandler
             * string.
             */
            let cfg_json: String = format!(
                "{{{},{}}}",
                cfg_header_json,
                cfg_include_json
                    .trim_start()
                    .trim_start_matches('{')
                    .trim_end()
                    .trim_end_matches('}')
            );
            let mut cfg: WsBuildConfigHandler =
                WsBuildConfigHandler::from_str(&cfg_json, settings)?;
            main_config.merge(&mut cfg);
        }

        /*
         * We expand this because it is the first time both the build configuration
         * and the settings are loaded. The build configuration requires the settings
         * to determine where to load the configuration from, while the settings may
         * depend on context variables defined in the build configuration.
         * This creates a circular dependency, which is not ideal.
         */
        main_config.expand_ctx()?;

        return Ok(main_config);
    }

    pub fn build_config(
        &self,
        name: &str,
        settings: &WsSettingsHandler,
    ) -> Result<WsBuildConfigHandler, BError> {
        let mut build_config: PathBuf = PathBuf::from(name);
        build_config.set_extension("json");
        let mut path: PathBuf = settings.work_dir().join(build_config.clone());

        /* We start by looking for the build config in the workspace/work directory */
        if path.exists() {
            return self.setup_build_config(&path, settings);
        }

        /*
         * If we cannot locate the build config in the workspace/work dir we continue to look
         * for it under the configs dir
         */
        path = settings.configs_dir().join(build_config.clone());
        if path.exists() {
            return self.setup_build_config(&path, settings);
        }

        /* TODO: we should remove this and most likely refactor the code so that the sub-commands are responsible for the build config */
        if build_config.display().to_string() == "NA.json".to_string() {
            let dummy_config_json: &str = r#"
                {
                    "version": "6",
                    "name": "all",
                    "description": "Dummy build config to be able to handle 'list' sub-command",
                    "arch": "NA"
                }"#;
            return WsBuildConfigHandler::from_str(&dummy_config_json, settings);
        }

        return Err(BError::ValueError(format!(
            "No such build config: '{}' does not exist!",
            build_config.clone().display()
        )));
    }
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;
    use std::path::PathBuf;
    use tempdir::TempDir;

    use crate::configs::WsConfigFileHandler;
    use crate::error::BError;
    use crate::helper::Helper;
    use crate::workspace::{
        WsBuildConfigHandler, WsCustomSubCmdHandler, WsSettingsHandler, WsTaskHandler,
    };

    /*
     * Test that if no workspace settings file is available the default is used.
     * All the directories should be the default once
     */
    #[test]
    fn test_cfg_handler_settings_default() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = PathBuf::from(temp_dir.path()).join("workspace");
        let home_dir: PathBuf = PathBuf::from(temp_dir.path()).join("home");
        Helper::setup_test_ws_default_dirs(&work_dir);
        let settings_str: &str = r#"
        {
            "version": "5"
        }"#;
        let settings_path: PathBuf = PathBuf::from(format!(
            "{}/workspace.json",
            PathBuf::from(work_dir.clone()).display()
        ));
        let mut configs: IndexMap<PathBuf, String> = IndexMap::new();
        configs.insert(settings_path, settings_str.to_string());
        Helper::setup_test_build_configs_files(&configs);
        let cfg_handler: WsConfigFileHandler = WsConfigFileHandler::new(&work_dir, &home_dir);
        let settings: WsSettingsHandler = cfg_handler
            .ws_settings()
            .expect("Failed parse workspace settings");
        assert_eq!(settings.builds_dir(), work_dir.clone().join("builds"));
        assert_eq!(settings.cache_dir(), work_dir.clone().join(".cache"));
        assert_eq!(settings.artifacts_dir(), work_dir.clone().join("artifacts"));
        assert_eq!(settings.scripts_dir(), work_dir.clone().join("scripts"));
        assert_eq!(settings.docker_dir(), work_dir.clone().join("docker"));
        assert_eq!(settings.configs_dir(), work_dir.clone().join("configs"));
        assert_eq!(
            settings.include_dir(),
            work_dir.clone().join("configs/include")
        );
    }

    /*
     * Make sure that
     */
    #[test]
    fn test_cfg_handler_settings_home_dir() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = PathBuf::from(temp_dir.path()).join("workspace");
        let home_dir: PathBuf = PathBuf::from(temp_dir.path()).join("home");
        Helper::setup_test_ws_default_dirs(&work_dir);
        let ws_settings_1: &str = r#"
        {
            "version": "6",
            "workspace": {
                "configsdir": "config1_dir"
            }
        }"#;
        Helper::write_json_conf(&work_dir.clone().join("workspace.json"), ws_settings_1);
        let ws_settings_2: &str = r#"
        {
            "version": "6",
            "workspace": {
                "configsdir": "config2_dir"
            }
        }"#;
        Helper::write_json_conf(
            &home_dir.clone().join(".bakery/workspace.json"),
            ws_settings_2,
        );
        let cfg_handler: WsConfigFileHandler = WsConfigFileHandler::new(&work_dir, &home_dir);
        let settings: WsSettingsHandler = cfg_handler
            .ws_settings()
            .expect("Failed parse workspace settings");
        assert_eq!(settings.configs_dir(), work_dir.clone().join("config1_dir"));
    }

    /*
     * Test that the workspace settings file workspace/work dir is used
     */
    #[test]
    fn test_cfg_handler_settings_work_dir() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = PathBuf::from(temp_dir.path()).join("workspace");
        let home_dir: PathBuf = PathBuf::from(temp_dir.path()).join("home");
        Helper::setup_test_ws_default_dirs(&work_dir);
        let ws_settings: &str = r#"
        {
            "version": "6",
            "workspace": {
                "configsdir": "work_dir"
            }
        }"#;
        Helper::write_json_conf(&work_dir.clone().join("workspace.json"), ws_settings);
        let cfg_handler: WsConfigFileHandler = WsConfigFileHandler::new(&work_dir, &home_dir);
        let settings: WsSettingsHandler = cfg_handler
            .ws_settings()
            .expect("Failed parse workspace settings");
        assert_eq!(settings.configs_dir(), work_dir.join("work_dir"));
    }

    /*
     * Test that what happens if no build config an Error should be returned
     */
    #[test]
    fn test_cfg_handler_build_config() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = PathBuf::from(temp_dir.path()).join("workspace");
        let home_dir: PathBuf = PathBuf::from(temp_dir.path()).join("home");
        Helper::setup_test_ws_default_dirs(&work_dir);
        let cfg_handler: WsConfigFileHandler = WsConfigFileHandler::new(&work_dir, &home_dir);
        let settings: WsSettingsHandler = cfg_handler
            .ws_settings()
            .expect("Failed parse workspace settings");
        let result: Result<WsBuildConfigHandler, BError> =
            cfg_handler.build_config("invalid", &settings);
        match result {
            Ok(_build_cfg) => {
                panic!("Was expecting an error!");
            }
            Err(e) => {
                assert_eq!(
                    e.to_string(),
                    String::from("No such build config: 'invalid.json' does not exist!")
                );
            }
        }
    }

    /*
     * Test that if there exists a build config in the workspace/work dir then that is picked up
     */
    #[test]
    fn test_cfg_handler_ws_root_build_config() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = PathBuf::from(temp_dir.path()).join("workspace");
        let home_dir: PathBuf = PathBuf::from(temp_dir.path()).join("home");
        Helper::setup_test_ws_default_dirs(&work_dir);
        let cfg_handler: WsConfigFileHandler = WsConfigFileHandler::new(&work_dir, &home_dir);
        let settings: WsSettingsHandler = cfg_handler
            .ws_settings()
            .expect("Failed parse workspace settings");
        let build_conf_ws_root_dir = r#"
        {
            "version": "6",
            "name": "ws-root-build-config",
            "description": "Test Description",
            "arch": "test-arch"
        }"#;
        Helper::write_json_conf(
            &settings.work_dir().join("test.json"),
            build_conf_ws_root_dir,
        );
        let build_conf_configs_dir = r#"
        {
            "version": "6",
            "name": "ws-configs-build-config",
            "description": "Test Description",
            "arch": "test-arch"
        }"#;
        Helper::write_json_conf(
            &settings.configs_dir().join("test.json"),
            build_conf_configs_dir,
        );
        let config: WsBuildConfigHandler = cfg_handler
            .build_config("test", &settings)
            .expect("Failed parse build config");
        assert_eq!(config.build_data().name(), "ws-root-build-config");
    }

    /*
     * Test that the build config is picked up from the configs dir
     */
    #[test]
    fn test_cfg_handler_ws_configs_build_config() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = PathBuf::from(temp_dir.path()).join("workspace");
        let home_dir: PathBuf = PathBuf::from(temp_dir.path()).join("home");
        Helper::setup_test_ws_default_dirs(&work_dir);
        let cfg_handler: WsConfigFileHandler = WsConfigFileHandler::new(&work_dir, &home_dir);
        let settings: WsSettingsHandler = cfg_handler
            .ws_settings()
            .expect("Failed parse workspace settings");
        let build_conf_configs_dir = r#"
        {
            "version": "6",
            "name": "ws-configs-build-config",
            "description": "Test Description",
            "arch": "test-arch"
        }"#;
        Helper::write_json_conf(
            &settings.configs_dir().join("test.json"),
            build_conf_configs_dir,
        );
        let config: WsBuildConfigHandler = cfg_handler
            .build_config("test", &settings)
            .expect("Failed parse build config");
        assert_eq!(config.build_data().name(), "ws-configs-build-config");
    }

    #[test]
    fn test_cfg_handler_ws_include_configs() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = PathBuf::from(temp_dir.path()).join("workspace");
        let home_dir: PathBuf = PathBuf::from(temp_dir.path()).join("home");
        Helper::setup_test_ws_default_dirs(&work_dir);
        let cfg_handler: WsConfigFileHandler = WsConfigFileHandler::new(&work_dir, &home_dir);
        let settings: WsSettingsHandler = cfg_handler
            .ws_settings()
            .expect("Failed parse workspace settings");
        let main_build_config = r#"
        {
            "version": "6",
            "name": "test-product",
            "description": "Test Description",
            "arch": "test-arch",
            "bb": {
                "machine": "test-machine",
                "distro": "test-distro",
                "deploydir": "tmp/test/deploy",
                "docker": "test-registry/test-image:0.1",
                "initenv": "layers/test/oe-my-init-env",
                "bblayersconf": [
                    "BB_LAYERS_CONF_TEST_LINE_1",
                    "BB_LAYERS_CONF_TEST_LINE_2",
                    "BB_LAYERS_CONF_TEST_LINE_3"
                ],
                "localconf": [
                    "BB_LOCAL_CONF_TEST_LINE_1",
                    "BB_LOCAL_CONF_TEST_LINE_2",
                    "BB_LOCAL_CONF_TEST_LINE_3"
                ]
            },
            "include": [
                "config1",
                "config2"
            ],
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
        Helper::write_json_conf(&settings.work_dir().join("main.json"), main_build_config);
        let build_config1 = r#"
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
                    "recipes": [
                        "test"
                    ],
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
        Helper::write_json_conf(&settings.include_dir().join("config1.json"), build_config1);
        let build_config2 = r#"
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
        Helper::write_json_conf(&settings.include_dir().join("config2.json"), build_config2);
        let config: WsBuildConfigHandler = cfg_handler
            .build_config("main", &settings)
            .expect("Failed parse build config");
        assert_eq!(config.build_data().name(), "test-product");
        let t0: &WsTaskHandler = config.tasks().get("task0").unwrap();
        assert_eq!(t0.data().build_cmd(), "main");
        assert_eq!(
            t0.data().build_dir(),
            &settings.work_dir().join("test/main")
        );
        let t1: &WsTaskHandler = config.tasks().get("task1").unwrap();
        assert_eq!(t1.data().recipes(), &vec!["test"]);
        assert_eq!(
            t1.data().build_dir(),
            &settings.work_dir().join("builds/test-product")
        );
        let t2: &WsTaskHandler = config.tasks().get("task2").unwrap();
        assert_eq!(t2.data().build_cmd(), "config2");
        assert_eq!(
            t2.data().build_dir(),
            &settings.work_dir().join("test/config2")
        );
        let setup: &WsCustomSubCmdHandler = config.subcmds().get("setup").unwrap();
        assert_eq!(setup.data().cmd(), "main");
        let sync: &WsCustomSubCmdHandler = config.subcmds().get("sync").unwrap();
        assert_eq!(sync.data().cmd(), "config1");
        let upload: &WsCustomSubCmdHandler = config.subcmds().get("upload").unwrap();
        assert_eq!(upload.data().cmd(), "config2");
    }

    #[test]
    fn test_cfg_handler_ws_configs_using_build_config_ctx() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = PathBuf::from(temp_dir.path()).join("workspace");
        let home_dir: PathBuf = PathBuf::from(temp_dir.path()).join("home");
        Helper::setup_test_ws_default_dirs(&work_dir);
        /*
         * The context variable $#[BKRY_NAME] is comming from the build config and the
         * config handler needs the worspace settings to be able to locate the build
         * config. Bakery should parse the workspace setting and expand default context
         * variables and leave any build config variables that are not available and should
         * be expanded in a second expand context call.
         */
        let ws_settings: &str = r#"
        {
            "version": "5",
            "workspace": {
                "artifactsdir": "artifacts/$#[BKRY_NAME]",
                "includedir": "$#[BKRY_CFG_DIR]/include",
                "scriptsdir": "$#[BKRY_OPT_SCRIPTS_DIR]"
            }
        }"#;
        Helper::write_json_conf(&work_dir.join("workspace.json"), ws_settings);
        let cfg_handler: WsConfigFileHandler = WsConfigFileHandler::new(&work_dir, &home_dir);
        let settings: WsSettingsHandler = cfg_handler
            .ws_settings()
            .expect("Failed parse workspace settings");
        /*
         * Calling ws_settings() should have expanded any context variable that was
         * not empty. Any build config context variable would be empty at this stage
         * so the $#[BKRY_NAME] should not have been expanded at this point.
         */
        assert_eq!(
            settings.artifacts_dir(),
            work_dir.join("artifacts/$#[BKRY_NAME]")
        );
        assert_eq!(settings.include_dir(), PathBuf::from("/etc/bakery/include"));
        assert_eq!(settings.scripts_dir(), PathBuf::from("/opt/bakery/scripts"));
        let build_conf_configs_dir = r#"
        {
            "version": "6",
            "name": "ws-configs-build-config",
            "description": "Test Description",
            "arch": "test-arch"
        }"#;
        Helper::write_json_conf(
            &settings.configs_dir().join("test.json"),
            build_conf_configs_dir,
        );
        let config: WsBuildConfigHandler = cfg_handler
            .build_config("test", &settings)
            .expect("Failed parse build config");
        /*
         * Calling build_config should have expanded the settings available in the
         * build data.
         */
        assert_eq!(
            config.build_data().context().get_ctx_value("BKRY_NAME"),
            "ws-configs-build-config"
        );
        assert_eq!(config.build_data().name(), "ws-configs-build-config");
        assert_eq!(
            config.build_data().settings().artifacts_dir(),
            work_dir.join("artifacts/ws-configs-build-config")
        );
        assert_eq!(
            config.build_data().settings().include_dir(),
            PathBuf::from("/etc/bakery/include")
        );
        assert_eq!(
            config.build_data().settings().scripts_dir(),
            PathBuf::from("/opt/bakery/scripts")
        );
    }
}
