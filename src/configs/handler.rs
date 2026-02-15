use indexmap::indexmap;
use std::path::PathBuf;

use crate::constants::BkryConstants;
use crate::data::WsContextData;
use crate::error::BError;
use crate::fs::ConfigFileReader;
use crate::workspace::{WsBuildConfigHandler, WsBuildMetadataHandler, WsSettingsHandler};

pub struct WsConfigFileHandler {
    work_dir: PathBuf,
    bkry_home_cfg_dir: PathBuf,
    bkry_cfg_dir: PathBuf,
}

impl WsConfigFileHandler {
    fn _load_settings_from_path(
        &self,
        path: &PathBuf,
    ) -> Result<Option<WsSettingsHandler>, BError> {
        if !path.exists() {
            return Ok(None);
        }

        let settings_str: String = ConfigFileReader::new(path).read_json()?;
        let mut settings: WsSettingsHandler =
            WsSettingsHandler::from_str(&self.work_dir, &settings_str, Some(path.clone()))?;

        // Create a context with default values and expand the settings
        let context: WsContextData = WsContextData::new(&indexmap! {})?;
        settings.expand_ctx(context.ctx())?;

        Ok(Some(settings))
    }

    fn _merge(
        &self,
        hidden_ws_settings: Option<&mut WsSettingsHandler>,
        ws_settings: Option<&mut WsSettingsHandler>,
        usr_settings: Option<&mut WsSettingsHandler>,
        etc_settings: Option<&mut WsSettingsHandler>,
    ) -> Result<WsSettingsHandler, BError> {
        /*
         * The merge procedure always produces a single settings object. The effective
         * priority is:
         *
         *  - /etc/bkry/workspace.json        (system-wide, lowest priority)
         *  - ./workspace.json                (workspace-specific)
         *  - ~/.bkry/workspace.json          (user-specific, highest priority)
         *
         * Examples:
         *  - Case A: only /etc/bkry/workspace.json exists
         *      -> use /etc/bkry/workspace.json (no merge needed)
         *
         *  - Case B: ~/.bkry/workspace.json and /etc/bkry/workspace.json exist
         *      -> start with /etc/bkry/workspace.json, overlay ~/.bkry/workspace.json
         *
         *  - Case C: ./workspace.json, ~/.bkry/workspace.json and /etc/bkry/workspace.json exist
         *      -> start with /etc/bkry/workspace.json, overlay ~/.bkry/workspace.json,
         *         then overlay ./workspace.json
         *
         *  - Case D: ./workspace.json and /etc/bkry/workspace.json exist
         *      -> start with /etc/bkry/workspace.json, then overlay ./workspace.json
         */
        //println!("hidden: {}, ws: {}, usr: {}, etc: {}", hidden_ws_settings.is_some(), ws_settings.is_some(), usr_settings.is_some(), etc_settings.is_some());
        match (hidden_ws_settings, ws_settings, usr_settings, etc_settings) {
            (Some(hidden), None, None, None) => {
                // Found only .workspace.json in current workspace
                return Ok(hidden.clone());
            }
            (None, Some(ws), None, None) => {
                // Found only workspac.json in current workspace
                return Ok(ws.clone());
            }
            (None, None, Some(usr), None) => {
                // Found ~/.bkry/workspace.json
                return Ok(usr.clone());
            }
            (None, None, None, Some(etc)) => {
                // Found /etc/bkry/workspace.json
                return Ok(etc.clone());
            }
            (None, None, Some(usr), Some(etc)) => {
                // Found ~/.bkry/workspace.json and /etc/bkry/workspace.json merge and return
                let mut settings: WsSettingsHandler = etc.clone();
                settings.merge(usr, false);
                return Ok(settings);
            }
            (None, Some(ws), Some(usr), None) => {
                // Found ~/.bkry/workspace.json and workspace.json merge and return
                let mut settings: WsSettingsHandler = ws.clone();
                settings.merge(usr, false);
                return Ok(settings);
            }
            (None, Some(ws), Some(usr), Some(etc)) => {
                // Found workspace.json and ~/.bkry/workspace.json merge and return
                let mut settings: WsSettingsHandler = etc.clone();
                settings.merge(ws, false);
                settings.merge(usr, false);
                return Ok(settings);
            }
            (Some(hidden), None, Some(usr), Some(etc)) => {
                // Found .workspace.json and ~/.bkry/workspace.json merge and return
                let mut settings: WsSettingsHandler = etc.clone();
                settings.merge(hidden, false);
                settings.merge(usr, false);
                return Ok(settings);
            }
            (None, Some(ws), None, Some(etc)) => {
                // Found workspace.json and /etc/bkry/workspace.json merge and return
                let mut settings: WsSettingsHandler = etc.clone();
                settings.merge(ws, false);
                return Ok(settings);
            }
            (Some(hidden), None, None, Some(etc)) => {
                // Found .workspace.json and /etc/bkry/workspace.json merge and return
                let mut settings: WsSettingsHandler = etc.clone();
                settings.merge(hidden, false);
                return Ok(settings);
            }
            (Some(_hidden), Some(ws), None, None) => {
                // Found .workspace.json and workspace.json ignore the hidden
                return Ok(ws.clone());
            }
            (Some(_hidden), Some(ws), None, Some(etc)) => {
                // Found .workspace.json and workspace.json and /etc/bkry/workspace.json ignore the hidden and merge
                let mut settings: WsSettingsHandler = etc.clone();
                settings.merge(ws, false);
                return Ok(settings);
            }
            _ => {
                /*
                 * Return default settings the only thing required is the version the rest
                 * will be defined by the default values in the settings handler
                 */
                let default_settings: &str = r#"{
                    "version": "6"
                }"#;
                return WsSettingsHandler::from_str(&self.work_dir, default_settings, None);
            }
        }
    }

    pub fn new(work_dir: &PathBuf, home_dir: &PathBuf, cfg_dir: &PathBuf) -> Self {
        let bkry_home_cfg_dir: PathBuf = home_dir.clone().join(".bkry");
        WsConfigFileHandler {
            work_dir: work_dir.clone(),
            bkry_home_cfg_dir,
            bkry_cfg_dir: cfg_dir.clone(),
        }
    }

    pub fn ws_settings(&self) -> Result<WsSettingsHandler, BError> {
        /*
         * Load workspace settings from the available workspace.json files and merge them
         * into the final configuration used by bkry.
         *
         * There should always be a system file at /etc/bkry/workspace.json. If there is
         * no workspace.json in the current directory, bkry is running outside a workspace.
         *
         * Possible files (from lowest to highest precedence):
         *  - /etc/bkry/workspace.json        (system-wide, lowest priority)
         *  - ./workspace.json                (workspace-specific)
         *  - ~/.bkry/workspace.json          (user-specific, highest priority)
         *
         */
        let mut etc_settings: Option<WsSettingsHandler> =
            self._load_settings_from_path(&self.bkry_cfg_dir.join(BkryConstants::WS_SETTINGS))?;
        let mut usr_settings: Option<WsSettingsHandler> = self
            ._load_settings_from_path(&self.bkry_home_cfg_dir.join(BkryConstants::WS_SETTINGS))?;
        let mut workspace_settings: Option<WsSettingsHandler> =
            self._load_settings_from_path(&self.work_dir.join(BkryConstants::WS_SETTINGS))?;
        let mut hidden_workspace_settings: Option<WsSettingsHandler> =
            self._load_settings_from_path(&self.work_dir.join(BkryConstants::WS_HIDDEN_SETTINGS))?;
        let settings: WsSettingsHandler = self._merge(
            hidden_workspace_settings.as_mut(),
            workspace_settings.as_mut(),
            usr_settings.as_mut(),
            etc_settings.as_mut(),
        )?;

        Ok(settings)
    }

    fn config_header(&self, config: &WsBuildConfigHandler) -> String {
        let cfg_bitbake_json: String = config.build_data().bitbake().to_string();
        let cfg_product_json: String = config.build_data().product().to_string();
        let cfg_header_json: String = format!("{},{}", cfg_product_json, cfg_bitbake_json);
        cfg_header_json.clone()
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
                    "name": "dummy",
                    "description": "Dummy build config to be able to handle 'list' sub-command",
                    "arch": "NA"
                }"#;
            return WsBuildConfigHandler::from_str(&dummy_config_json, settings);
        }

        return Err(BError::ValueError(format!(
            "No such build config: '{}' does not exist. Please run 'bkry list' to see a complete list of supported build configurations.",
            build_config.clone().display()
        )));
    }

    pub fn metadata(
        &self,
        _config: &str,
        settings: &WsSettingsHandler,
    ) -> Result<WsBuildMetadataHandler, BError> {
        let metadata: WsBuildMetadataHandler =
            WsBuildMetadataHandler::new(&settings.work_dir(), &self.bkry_home_cfg_dir, None);
        Ok(metadata)
    }

    pub fn verify_ws(&self) -> Result<(), BError> {
        /*
         * The search order for the workspace settings is:
         *
         * 1. Current working directory
         * 2. ~/.bkry/
         * 3. /etc/bkry/
         *
         * If none of these contain 'workspace.json', return an invalid workspace error.
         */
        if !self
            .work_dir
            .clone()
            .join(BkryConstants::WS_SETTINGS)
            .exists()
            && !self
                .bkry_home_cfg_dir
                .clone()
                .join(BkryConstants::WS_SETTINGS)
                .exists()
            && !self
                .bkry_cfg_dir
                .clone()
                .join(BkryConstants::WS_SETTINGS)
                .exists()
        {
            return Err(BError::InvalidWorkspaceError());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;
    use std::path::PathBuf;
    use tempdir::TempDir;

    use crate::configs::WsConfigFileHandler;
    use crate::constants::BkryConstants;
    use crate::error::BError;
    use crate::executers::DockerImage;
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
        let cfg_dir: PathBuf = PathBuf::from(temp_dir.path().join("etc/bkry"));
        let work_dir: PathBuf = PathBuf::from(temp_dir.path()).join("workspace");
        let home_dir: PathBuf = PathBuf::from(temp_dir.path()).join("home");
        Helper::setup_test_ws_default_dirs(&work_dir);
        let settings_str: &str = r#"
        {
            "version": "6"
        }"#;
        let settings_path: PathBuf = PathBuf::from(format!(
            "{}/{}",
            PathBuf::from(work_dir.clone()).display(),
            BkryConstants::WS_SETTINGS
        ));
        let mut configs: IndexMap<PathBuf, String> = IndexMap::new();
        configs.insert(settings_path, settings_str.to_string());
        Helper::setup_test_build_configs_files(&configs);
        let cfg_handler: WsConfigFileHandler =
            WsConfigFileHandler::new(&work_dir, &home_dir, &cfg_dir);
        let settings: WsSettingsHandler = cfg_handler
            .ws_settings()
            .expect("Failed parse workspace settings");
        assert_eq!(
            settings.builds_dir(),
            work_dir
                .clone()
                .join(BkryConstants::BKRY_DEFAULT_BUILDS_DIR)
        );
        assert_eq!(
            settings.cache_dir(),
            work_dir.clone().join(BkryConstants::BKRY_DEFAULT_CACHE_DIR)
        );
        assert_eq!(
            settings.artifacts_dir(),
            work_dir
                .clone()
                .join(BkryConstants::BKRY_DEFAULT_ARTIFACTS_DIR)
        );
        assert_eq!(
            settings.scripts_dir(),
            work_dir
                .clone()
                .join(BkryConstants::BKRY_DEFAULT_SCRIPTS_DIR)
        );
        assert_eq!(
            settings.docker_dir(),
            work_dir
                .clone()
                .join(BkryConstants::BKRY_DEFAULT_DOCKER_DIR)
        );
        assert_eq!(
            settings.configs_dir(),
            work_dir.clone().join(BkryConstants::BKRY_DEFAULT_CFG_DIR)
        );
        assert_eq!(
            settings.include_dir(),
            work_dir
                .clone()
                .join(BkryConstants::BKRY_DEFAULT_INCLUDE_CFG_DIR)
        );
    }

    /*
     * Test that the workspace settings file in the home bkry config dir is used instead
     * of the one in the root of the workspace/work dir
     */
    #[test]
    fn test_cfg_handler_usr_settings() {
        let temp_dir: TempDir =
            TempDir::new("bkry-test-dir").expect("Failed to create temp directory");
        let cfg_dir: PathBuf = PathBuf::from(temp_dir.path().join("etc/bkry"));
        let work_dir: PathBuf = PathBuf::from(temp_dir.path()).join("workspace");
        let home_dir: PathBuf = PathBuf::from(temp_dir.path()).join("home");
        Helper::setup_test_ws(&work_dir, BkryConstants::WS_SETTINGS);
        let ws_settings_1: &str = r#"
        {
            "version": "6",
            "workspace": {
                "configsdir": "ws_config_dir"
            }
        }"#;
        Helper::write_json_conf(
            &work_dir.clone().join(BkryConstants::WS_SETTINGS),
            ws_settings_1,
        );
        let ws_settings_2: &str = r#"
        {
            "version": "6",
            "workspace": {
                "configsdir": "usr_config_dir"
            }
        }"#;
        Helper::write_json_conf(
            &home_dir
                .clone()
                .join(format!(".bkry/{}", BkryConstants::WS_SETTINGS)),
            ws_settings_2,
        );
        let cfg_handler: WsConfigFileHandler =
            WsConfigFileHandler::new(&work_dir, &home_dir, &cfg_dir);
        let settings: WsSettingsHandler = cfg_handler
            .ws_settings()
            .expect("Failed parse workspace settings");
        assert_eq!(
            settings.configs_dir(),
            work_dir.clone().join("usr_config_dir")
        );
    }

    /*
     * Test that the workspace settings file under workspace/work dir is used
     */
    #[test]
    fn test_cfg_handler_settings_ws() {
        let temp_dir: TempDir =
            TempDir::new("bkry-test-dir").expect("Failed to create temp directory");
        let cfg_dir: PathBuf = PathBuf::from(temp_dir.path().join("etc/bkry"));
        let work_dir: PathBuf = PathBuf::from(temp_dir.path()).join("workspace");
        let home_dir: PathBuf = PathBuf::from(temp_dir.path()).join("home");
        Helper::setup_test_ws(&work_dir, BkryConstants::WS_SETTINGS);
        let ws_settings: &str = r#"
        {
            "version": "6",
            "workspace": {
                "configsdir": "work_dir"
            }
        }"#;
        Helper::write_json_conf(
            &work_dir.clone().join(BkryConstants::WS_SETTINGS),
            ws_settings,
        );
        let cfg_handler: WsConfigFileHandler =
            WsConfigFileHandler::new(&work_dir, &home_dir, &cfg_dir);
        let settings: WsSettingsHandler = cfg_handler
            .ws_settings()
            .expect("Failed parse workspace settings");
        assert_eq!(settings.configs_dir(), work_dir.join("work_dir"));
    }

    /*
     * Test that the hidden workspace settings file under workspace/work dir is used
     */
    #[test]
    fn test_cfg_handler_hidden_settings() {
        let temp_dir: TempDir =
            TempDir::new("bkry-test-dir").expect("Failed to create temp directory");
        let cfg_dir: PathBuf = PathBuf::from(temp_dir.path().join("etc/bkry"));
        let work_dir: PathBuf = PathBuf::from(temp_dir.path()).join("workspace");
        let home_dir: PathBuf = PathBuf::from(temp_dir.path()).join("home");
        Helper::setup_test_ws(&work_dir, BkryConstants::WS_HIDDEN_SETTINGS);
        let ws_settings: &str = r#"
        {
            "version": "6",
            "workspace": {
                "configsdir": "work_dir"
            }
        }"#;
        Helper::write_json_conf(
            &work_dir.clone().join(BkryConstants::WS_HIDDEN_SETTINGS),
            ws_settings,
        );
        let cfg_handler: WsConfigFileHandler =
            WsConfigFileHandler::new(&work_dir, &home_dir, &cfg_dir);
        let settings: WsSettingsHandler = cfg_handler
            .ws_settings()
            .expect("Failed parse workspace settings");
        assert_eq!(settings.configs_dir(), work_dir.join("work_dir"));
    }

    /*
     * Test that the workspace settings file under workspace/work dir is used
     * over the hidden workspace settings file.
     */
    #[test]
    fn test_cfg_handler_settings_order() {
        let temp_dir: TempDir =
            TempDir::new("bkry-test-dir").expect("Failed to create temp directory");
        let cfg_dir: PathBuf = PathBuf::from(temp_dir.path().join("etc/bkry"));
        let work_dir: PathBuf = PathBuf::from(temp_dir.path()).join("workspace");
        let home_dir: PathBuf = PathBuf::from(temp_dir.path()).join("home");
        /*
         * Create an default workspace.json in the workspace/work dir this should be the one
         * picked up by bkry
         */
        Helper::setup_test_ws(&work_dir, BkryConstants::WS_SETTINGS);
        let ws_settings: &str = r#"
        {
            "version": "6",
            "workspace": {
                "configsdir": "configs"
            }
        }"#;
        /*
         * Create a .workspace.json in the workspace/work dir this should no be used
         */
        Helper::write_json_conf(
            &work_dir.clone().join(BkryConstants::WS_HIDDEN_SETTINGS),
            ws_settings,
        );
        let cfg_handler: WsConfigFileHandler =
            WsConfigFileHandler::new(&work_dir, &home_dir, &cfg_dir);
        let settings: WsSettingsHandler = cfg_handler
            .ws_settings()
            .expect("Failed parse workspace settings");
        /*
         * The workspace.json under workdir should be used over the .workspace.json
         * so the configsdir defined in workspace.json should be used and not the
         * one from .workspace.json
         */
        assert_eq!(settings.configs_dir(), work_dir.join("configs"));
    }

    /*
     * Make sure that when merge order works as expected. The value from the usr should
     * be picked up.
     */
    #[test]
    fn test_cfg_handler_merge_all() {
        let temp_dir: TempDir =
            TempDir::new("bkry-test-dir").expect("Failed to create temp directory");
        let cfg_dir: PathBuf = PathBuf::from(temp_dir.path().join("etc/bkry"));
        let work_dir: PathBuf = PathBuf::from(temp_dir.path()).join("workspace");
        let home_dir: PathBuf = PathBuf::from(temp_dir.path()).join("home");
        Helper::setup_test_ws(&work_dir, BkryConstants::WS_SETTINGS);
        let ws_settings: &str = r#"
        {
            "version": "6",
            "workspace": {
                "configsdir": "ws_config_dir"
            }
        }"#;
        Helper::write_json_conf(
            &work_dir.clone().join(BkryConstants::WS_SETTINGS),
            ws_settings,
        );
        let usr_settings: &str = r#"
        {
            "version": "6",
            "workspace": {
                "configsdir": "usr_config_dir"
            }
        }"#;
        Helper::write_json_conf(
            &home_dir
                .clone()
                .join(format!(".bkry/{}", BkryConstants::WS_SETTINGS)),
            usr_settings,
        );
        let etc_settings: &str = r#"
        {
            "version": "6",
            "workspace": {
                "configsdir": "etc_config_dir"
            }
        }"#;
        Helper::write_json_conf(
            &cfg_dir.clone().join(BkryConstants::WS_SETTINGS),
            etc_settings,
        );
        let cfg_handler: WsConfigFileHandler =
            WsConfigFileHandler::new(&work_dir, &home_dir, &cfg_dir);
        let settings: WsSettingsHandler = cfg_handler
            .ws_settings()
            .expect("Failed parse workspace settings");
        assert_eq!(
            settings.configs_dir(),
            work_dir.clone().join("usr_config_dir")
        );
    }

    /*
     * Make sure that when merge order works as expected. When it comes to the docker
     * args it is not really a merge done it is simply appending the values.
     */
    #[test]
    fn test_cfg_handler_merge_docker() {
        let temp_dir: TempDir =
            TempDir::new("bkry-test-dir").expect("Failed to create temp directory");
        let cfg_dir: PathBuf = PathBuf::from(temp_dir.path().join("etc/bkry"));
        let work_dir: PathBuf = PathBuf::from(temp_dir.path()).join("workspace");
        let home_dir: PathBuf = PathBuf::from(temp_dir.path()).join("home");
        Helper::setup_test_ws(&work_dir, BkryConstants::WS_SETTINGS);
        let ws_settings: &str = r#"
        {
            "version": "6",
            "docker": {
                "registry": "ws",
                "image": "ws",
                "tag": "1.1.1",
                "args": [
                    "-v ws_vol:ws_vol"
                ]
            }
        }"#;
        Helper::write_json_conf(
            &work_dir.clone().join(BkryConstants::WS_SETTINGS),
            ws_settings,
        );
        let usr_settings: &str = r#"
        {
            "version": "6",
            "docker": {
                "registry": "usr",
                "image": "usr",
                "tag": "2.2.2",
                "args": [
                    "-v usr_vol:usr_vol"
                ]
            }
        }"#;
        Helper::write_json_conf(
            &home_dir
                .clone()
                .join(format!(".bkry/{}", BkryConstants::WS_SETTINGS)),
            usr_settings,
        );
        let etc_settings: &str = r#"
        {
            "version": "6",
            "docker": {
                "registry": "etc",
                "image": "etc",
                "tag": "3.3.3",
                "args": [
                    "-v etc_vol:etc_vol"
                ]
            }
        }"#;
        Helper::write_json_conf(
            &cfg_dir.clone().join(BkryConstants::WS_SETTINGS),
            etc_settings,
        );
        let cfg_handler: WsConfigFileHandler =
            WsConfigFileHandler::new(&work_dir, &home_dir, &cfg_dir);
        let settings: WsSettingsHandler = cfg_handler
            .ws_settings()
            .expect("Failed parse workspace settings");
        assert_eq!(
            format!("{}", settings.docker_image()),
            format!("{}", DockerImage::new("usr/usr:2.2.2").expect("error"))
        );
        assert_eq!(
            settings.docker_args(),
            &vec![
                "-v etc_vol:etc_vol".to_string(),
                "-v ws_vol:ws_vol".to_string(),
                "-v usr_vol:usr_vol".to_string()
            ]
        );
    }

    /*
     * Make sure that the merge is working. When it comes to the supported
     * builds we should never pick it up from the user specific workspace.json
     * unless they have specifically added a new set of supported builds.
     */
    #[test]
    fn test_cfg_handler_merge_builds() {
        let temp_dir: TempDir =
            TempDir::new("bkry-test-dir").expect("Failed to create temp directory");
        let cfg_dir: PathBuf = PathBuf::from(temp_dir.path().join("etc/bkry"));
        let work_dir: PathBuf = PathBuf::from(temp_dir.path()).join("workspace");
        let home_dir: PathBuf = PathBuf::from(temp_dir.path()).join("home");
        let usr_settings: &str = r#"
        {
            "version": "6"
        }"#;
        Helper::write_json_conf(
            &home_dir
                .clone()
                .join(format!(".bkry/{}", BkryConstants::WS_SETTINGS)),
            usr_settings,
        );
        Helper::setup_test_ws(&work_dir, BkryConstants::WS_SETTINGS);
        let ws_settings: &str = r#"
        {
            "version": "6",
            "builds": {
                "supported": [
                    "ws_config1",
                    "ws_config2"
                ]
            }
        }"#;
        Helper::write_json_conf(
            &work_dir.clone().join(BkryConstants::WS_SETTINGS),
            ws_settings,
        );
        let etc_settings: &str = r#"
        {
            "version": "6",
            "builds": {
                "supported": [
                    "etc_config1",
                    "etc_config2"
                ]
            }
        }"#;
        Helper::write_json_conf(
            &cfg_dir.clone().join(BkryConstants::WS_SETTINGS),
            etc_settings,
        );
        let cfg_handler: WsConfigFileHandler =
            WsConfigFileHandler::new(&work_dir, &home_dir, &cfg_dir);
        let settings: WsSettingsHandler = cfg_handler
            .ws_settings()
            .expect("Failed parse workspace settings");
        assert_eq!(
            settings.config().supported,
            vec!["ws_config1".to_string(), "ws_config2".to_string()]
        );
    }

    /*
     * Test that what happens if no build config an Error should be returned
     */
    #[test]
    fn test_cfg_handler_build_config() {
        let temp_dir: TempDir =
            TempDir::new("bkry-test-dir").expect("Failed to create temp directory");
        let cfg_dir: PathBuf = PathBuf::from(temp_dir.path().join("etc/bkry"));
        let work_dir: PathBuf = PathBuf::from(temp_dir.path()).join("workspace");
        let home_dir: PathBuf = PathBuf::from(temp_dir.path()).join("home");
        Helper::setup_test_ws(&work_dir, BkryConstants::WS_SETTINGS);
        let cfg_handler: WsConfigFileHandler =
            WsConfigFileHandler::new(&work_dir, &home_dir, &cfg_dir);
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
                    String::from("No such build config: 'invalid.json' does not exist. Please run 'bkry list' to see a complete list of supported build configurations.")
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
            TempDir::new("bkry-test-dir").expect("Failed to create temp directory");
        let cfg_dir: PathBuf = PathBuf::from(temp_dir.path().join("etc/bkry"));
        let work_dir: PathBuf = PathBuf::from(temp_dir.path()).join("workspace");
        let home_dir: PathBuf = PathBuf::from(temp_dir.path()).join("home");
        Helper::setup_test_ws(&work_dir, BkryConstants::WS_SETTINGS);
        let cfg_handler: WsConfigFileHandler =
            WsConfigFileHandler::new(&work_dir, &home_dir, &cfg_dir);
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
            TempDir::new("bkry-test-dir").expect("Failed to create temp directory");
        let cfg_dir: PathBuf = PathBuf::from(temp_dir.path().join("etc/bkry"));
        let work_dir: PathBuf = PathBuf::from(temp_dir.path()).join("workspace");
        let home_dir: PathBuf = PathBuf::from(temp_dir.path()).join("home");
        Helper::setup_test_ws(&work_dir, BkryConstants::WS_SETTINGS);
        let cfg_handler: WsConfigFileHandler =
            WsConfigFileHandler::new(&work_dir, &home_dir, &cfg_dir);
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
        let cfg_dir: PathBuf = PathBuf::from(temp_dir.path().join("etc/bkry"));
        let work_dir: PathBuf = PathBuf::from(temp_dir.path()).join("workspace");
        let home_dir: PathBuf = PathBuf::from(temp_dir.path()).join("home");
        Helper::setup_test_ws_default_dirs(&work_dir);
        let cfg_handler: WsConfigFileHandler =
            WsConfigFileHandler::new(&work_dir, &home_dir, &cfg_dir);
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
}
