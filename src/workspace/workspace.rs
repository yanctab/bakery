use indexmap::IndexMap;
use std::env;
use std::fs::DirEntry;
use std::io::Error;
use std::path::PathBuf;

use crate::data::{WsContextData, WsProductData};
use crate::error::BError;
use crate::fs::ConfigFileReader;
use crate::workspace::{WsBuildConfigHandler, WsSettingsHandler};

pub struct Workspace {
    config: WsBuildConfigHandler,
    configs: IndexMap<PathBuf, String>,
}

impl Workspace {
    fn setup_work_directory(work_dir: &Option<PathBuf>) -> PathBuf {
        let path_buf: PathBuf = PathBuf::new();
        match work_dir {
            Some(w_dir) => path_buf.join(w_dir),
            None => {
                let path: Result<PathBuf, Error> = env::current_dir();
                match path {
                    Ok(w_dir) => path_buf.join(w_dir),
                    Err(_e) => {
                        panic!("{}", _e.to_string());
                    }
                }
            }
        }
    }

    fn setup_settings(work_dir: PathBuf, settings: Option<WsSettingsHandler>) -> WsSettingsHandler {
        match settings {
            Some(ws_settings) => ws_settings,
            None => {
                /*
                 * If settings is not supplied we use default
                 * settings by supplying a basic settings json
                 * string. This string should be valid and never fail
                 * so we don't care about error because we assume it
                 * will be fine. This should be added as part of one
                 * the tests.
                 */
                let default_settings: &str = r#"
                {
                    "version": "6"
                }"#;
                WsSettingsHandler::from_str(&work_dir, default_settings, None).unwrap()
            }
        }
    }

    fn setup_config(
        settings: &mut WsSettingsHandler,
        config: Option<WsBuildConfigHandler>,
    ) -> WsBuildConfigHandler {
        match config {
            Some(ws_config) => ws_config,
            None => {
                // If config is not supplied we use default
                // config by supplying a basic build config json
                // string. This string should be valid and never fail
                // so we don't care about error because we assume it
                // will be fine. This should be added as part of
                // the tests.
                let default_config: &str = r#"
                {
                    "version": "6",
                    "name": "default",
                    "description": "Default build config",
                    "arch": "NA"
                }"#;
                WsBuildConfigHandler::from_str(default_config, settings).unwrap()
            }
        }
    }

    fn setup_list_of_available_configs(
        settings: &WsSettingsHandler,
        config: &WsBuildConfigHandler,
    ) -> Result<IndexMap<PathBuf, String>, BError> {
        let mut list_of_files: Vec<String> = Vec::new();
        if settings.supported_builds().is_empty() {
            // If the list of supported builds is empty in the settings
            // all build configs are supported
            let config_dir: std::fs::ReadDir = std::fs::read_dir(settings.configs_dir())?;

            for entry in config_dir {
                let e: DirEntry = entry
                    .map_err(|err| BError::WsError(format!("Failed read dir entry: '{}'", err)))?;
                let path: PathBuf = e.path();

                if path.is_file() {
                    if let Some(extension) = path.extension() {
                        if extension == "json" {
                            if let Some(file_name) = path.file_name() {
                                if let Some(file_name_str) = file_name.to_str() {
                                    let build_config_json: String =
                                        ConfigFileReader::new(&path).read_json()?;
                                    let config: WsBuildConfigHandler =
                                        WsBuildConfigHandler::from_str(
                                            &build_config_json,
                                            settings,
                                        )?;
                                    if config.build_data().valid() {
                                        list_of_files.push(file_name_str.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else {
            // we could have used map() but in this case it make sense to use
            // for_each()
            settings.supported_builds().iter().for_each(|build| {
                list_of_files.push(format!("{}.json", build));
            });
        }

        let mut build_configs: IndexMap<PathBuf, String> = IndexMap::new();
        if list_of_files.len() == 1 && list_of_files.get(0).unwrap() == &"default.json".to_string()
        {
            // This means that we have a default build config which is the absolut
            // minimum of a build config and will only be used if a command
            // is used where a build config is not specified.
            build_configs.insert(
                PathBuf::from(settings.configs_dir().join("default.json")),
                config.description().to_string(),
            );
        } else {
            for f in list_of_files {
                let config_path: PathBuf = settings.configs_dir().join(f);
                let config_str: String = ConfigFileReader::new(&config_path).read_json()?;
                let product: WsProductData = WsProductData::from_str(&config_str)?;
                build_configs.insert(config_path, product.description().to_string());
            }
        }

        Ok(build_configs)
    }

    pub fn new(
        workdir: Option<PathBuf>,
        settings: Option<WsSettingsHandler>,
        config: Option<WsBuildConfigHandler>,
    ) -> Result<Self, BError> {
        let work_dir: PathBuf = Self::setup_work_directory(&workdir);
        let mut settings: WsSettingsHandler = Self::setup_settings(work_dir, settings);
        let config: WsBuildConfigHandler = Self::setup_config(&mut settings, config);
        let configs: IndexMap<PathBuf, String> =
            Self::setup_list_of_available_configs(&settings, &config)?;

        Ok(Workspace { config, configs })
    }

    pub fn settings(&self) -> &WsSettingsHandler {
        &self.config.build_data().settings()
    }

    pub fn config(&self) -> &WsBuildConfigHandler {
        &self.config
    }

    // Returns a dictionary including all build configurations names
    // and their description
    pub fn build_configs(&self) -> &IndexMap<PathBuf, String> {
        &self.configs
    }

    // Returns true if the config is part of the list
    // of build configs supported by the workspace
    pub fn valid_config(&self, config: &str) -> bool {
        self.build_configs().contains_key(
            &self
                .config()
                .build_data()
                .settings()
                .configs_dir()
                .join(format!("{}.json", config)),
        )
    }

    pub fn update_ctx(&mut self, context: &WsContextData) -> Result<(), BError> {
        self.config.update_ctx(context.ctx());
        self.expand_ctx()?;
        Ok(())
    }

    pub fn expand_ctx(&mut self) -> Result<(), BError> {
        self.config.expand_ctx()?;
        Ok(())
    }

    pub fn context(&self) -> Result<IndexMap<String, String>, BError> {
        self.config().ctx()
    }

    pub fn verify_ws(&mut self) -> Result<(), BError> {
        self.config.build_data().settings().verify_ws()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;
    use std::path::{Path, PathBuf};
    use tempdir::TempDir;

    use crate::executers::DockerImage;
    use crate::helper::Helper;
    use crate::workspace::Workspace;

    #[test]
    fn test_workspace_default() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let test_work_dir: &Path = temp_dir.path();
        Helper::setup_test_ws_default_dirs(test_work_dir);
        let ws: Workspace = Workspace::new(Some(PathBuf::from(test_work_dir)), None, None)
            .expect("Failed to setup workspace");
        assert_eq!(ws.settings().builds_dir(), test_work_dir.join("builds"));
        assert_eq!(ws.settings().cache_dir(), test_work_dir.join(".cache"));
        assert_eq!(
            ws.settings().artifacts_dir(),
            test_work_dir.join("artifacts")
        );
        assert_eq!(ws.settings().scripts_dir(), test_work_dir.join("scripts"));
        assert_eq!(ws.settings().docker_dir(), test_work_dir.join("docker"));
        assert_eq!(ws.settings().configs_dir(), test_work_dir.join("configs"));
        assert_eq!(
            ws.settings().include_dir(),
            test_work_dir.join("configs/include")
        );
        let docker_image: DockerImage = ws.settings().docker_image();
        assert_eq!(
            format!("{}", docker_image),
            format!(
                "ghcr.io/yanctab/bakery/bakery-workspace:{}",
                env!("CARGO_PKG_VERSION")
            )
        );
        assert!(ws.settings().docker_args().is_empty());
        assert!(ws.settings().supported_builds().is_empty());
    }

    #[test]
    fn test_workspace_build_configs() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let test_work_dir: &Path = temp_dir.path();
        let mut configs: IndexMap<PathBuf, String> = IndexMap::new();
        let config1_str: &str = r#"
        {
            "version": "6",
            "name": "test-name1",
            "description": "Test1 Description",
            "arch": "test-arch",
            "bb": {}
        }"#;
        let config2_str: &str = r#"
        {
            "version": "6",
            "name": "test-name2",
            "description": "Test2 Description",
            "arch": "test-arch",
            "bb": {}
        }"#;
        let config1_path: PathBuf = PathBuf::from(format!(
            "{}/configs/test-name1.json",
            PathBuf::from(test_work_dir).display()
        ));
        let config2_path: PathBuf = PathBuf::from(format!(
            "{}/configs/test-name2.json",
            PathBuf::from(test_work_dir).display()
        ));
        configs.insert(config1_path, config1_str.to_string());
        configs.insert(config2_path, config2_str.to_string());
        Helper::setup_test_ws_default_dirs(test_work_dir);
        Helper::setup_test_build_configs_files(&configs);
        let ws: Workspace = Workspace::new(Some(PathBuf::from(test_work_dir)), None, None)
            .expect("Failed to setup workspace");
        assert!(!ws.build_configs().is_empty());
        ws.build_configs().iter().for_each(|(config, description)| {
            // We cannot garanty the order
            if config.file_name().unwrap() == "test-name1.json" {
                assert_eq!(
                    config.as_path(),
                    test_work_dir.join("configs/test-name1.json")
                );
                assert_eq!(description.to_string(), "Test1 Description");
            } else {
                assert_eq!(
                    config.as_path(),
                    test_work_dir.join("configs/test-name2.json")
                );
                assert_eq!(description.to_string(), "Test2 Description");
            }
        });
    }

    #[test]
    fn test_workspace_default_settings() {
        let test_work_dir: &str = "/test_work_dir";
        let json_settings: &str = r#"
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
            "name": "test-name",
            "description": "Test Description",
            "arch": "test-arch",
            "bb": {}
        }"#;
        let ws: Workspace = Helper::setup_ws(test_work_dir, json_settings, json_build_config);
        assert_eq!(
            ws.settings().builds_dir(),
            PathBuf::from("/test_work_dir/builds")
        );
        assert_eq!(
            ws.settings().cache_dir(),
            PathBuf::from("/test_work_dir/.cache")
        );
        assert_eq!(
            ws.settings().artifacts_dir(),
            PathBuf::from("/test_work_dir/artifacts")
        );
        assert_eq!(
            ws.settings().scripts_dir(),
            PathBuf::from("/test_work_dir/scripts")
        );
        assert_eq!(
            ws.settings().docker_dir(),
            PathBuf::from("/test_work_dir/docker")
        );
        assert_eq!(
            ws.settings().configs_dir(),
            PathBuf::from("/test_work_dir/configs")
        );
        assert_eq!(
            ws.settings().include_dir(),
            PathBuf::from("/test_work_dir/configs/include")
        );
        let docker_image: DockerImage = ws.settings().docker_image();
        assert_eq!(
            format!("{}", docker_image),
            format!(
                "ghcr.io/yanctab/bakery/bakery-workspace:{}",
                env!("CARGO_PKG_VERSION")
            )
        );
        assert!(ws.settings().docker_args().is_empty());
        assert_eq!(
            ws.settings().supported_builds(),
            &vec!["default".to_string()]
        );
    }

    #[test]
    fn test_workspace_context_settings() {
        let test_work_dir: &str = "/test_work_dir";
        let json_settings: &str = r#"
        {
            "version": "6",
            "builds": {
                "supported": [
                    "default"
                ]
            },
            "workspace": {
                "artifactsdir": "artifacts/$#[BKRY_NAME]"
            }
        }"#;
        let json_build_config: &str = r#"
        {
            "version": "6",
            "name": "test-name",
            "description": "Test Description",
            "arch": "test-arch",
            "bb": {}
        }"#;
        let mut ws: Workspace = Helper::setup_ws(test_work_dir, json_settings, json_build_config);
        ws.expand_ctx().expect("Failed to expand the context");
        assert_eq!(
            ws.settings().artifacts_dir(),
            PathBuf::from("/test_work_dir/artifacts/test-name")
        );
    }

    #[test]
    fn test_workspace_valid_config() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let test_work_dir: &Path = temp_dir.path();
        let mut configs: IndexMap<PathBuf, String> = IndexMap::new();
        let config_str: &str = r#"
        {
            "version": "6",
            "name": "test-name",
            "description": "Test Description",
            "arch": "test-arch",
            "bb": {}
        }"#;
        let config_path: PathBuf = PathBuf::from(format!(
            "{}/configs/test-name.json",
            PathBuf::from(test_work_dir).display()
        ));
        configs.insert(config_path, config_str.to_string());
        Helper::setup_test_ws_default_dirs(test_work_dir);
        Helper::setup_test_build_configs_files(&configs);
        let ws: Workspace = Workspace::new(Some(PathBuf::from(test_work_dir)), None, None)
            .expect("Failed to setup workspace");
        assert!(!ws.build_configs().is_empty());
        let (path, description) = ws.build_configs().first().unwrap();
        assert_eq!(path.as_path(), test_work_dir.join("configs/test-name.json"));
        assert_eq!(description, "Test Description");
        assert!(ws.valid_config("test-name"));
    }
}
