use crate::configs::Context;
use crate::error::BError;
use crate::{configs::WsSettings, executers::DockerImage};

use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct WsSettingsHandler {
    work_dir: PathBuf,
    ws_settings: WsSettings,
    docker: DockerImage,
}

impl WsSettingsHandler {
    pub fn from_str(work_dir: &PathBuf, json_settings: &str) -> Result<Self, BError> {
        let work_dir: PathBuf = work_dir.clone();
        let result: Result<WsSettings, BError> = WsSettings::from_str(json_settings);
        match result {
            Ok(rsettings) => Ok(Self::new(work_dir, rsettings)),
            Err(e) => Err(e),
        }
    }

    pub fn new(work_dir: PathBuf, settings: WsSettings) -> Self {
        let docker: DockerImage = DockerImage {
            image: settings.docker_image.clone(),
            tag: settings.docker_tag.clone(),
            registry: settings.docker_registry.clone(),
        };
        WsSettingsHandler {
            work_dir,
            ws_settings: settings,
            docker,
        }
    }

    pub fn verify_ws_dir(&self, dir: &Path) -> Result<(), BError> {
        if !dir.is_dir() || !dir.exists() {
            return Err(BError::WsError(format!(
                "Workspace directory '{}' dosen't exists",
                dir.display()
            )));
        }
        return Ok(());
    }

    pub fn verify_ws(&self) -> Result<(), BError> {
        let ws_config: PathBuf = self.work_dir().join("workspace.json");
        if !ws_config.exists() || !ws_config.is_file() {
            return Err(BError::WsError(format!(
                "Not a bakery workspace, file 'workspace.json' missing"
            )));
        }
        self.verify_ws_dir(self.configs_dir().as_path())?;
        self.verify_ws_dir(self.scripts_dir().as_path())?;
        Ok(())
    }

    pub fn work_dir(&self) -> PathBuf {
        self.work_dir.clone()
    }

    pub fn config(&self) -> &WsSettings {
        &self.ws_settings
    }

    pub fn append_dir(&self, dir: &String) -> PathBuf {
        let mut path_buf: PathBuf = self.work_dir();
        if dir.is_empty() {
            return path_buf;
        }
        path_buf.push(&dir);
        path_buf
    }

    pub fn builds_dir(&self) -> PathBuf {
        self.append_dir(&self.ws_settings.builds_dir)
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.append_dir(&self.ws_settings.cache_dir)
    }

    pub fn artifacts_dir(&self) -> PathBuf {
        self.append_dir(&self.ws_settings.artifacts_dir)
    }

    pub fn layers_dir(&self) -> PathBuf {
        self.append_dir(&self.ws_settings.layers_dir)
    }

    pub fn configs_dir(&self) -> PathBuf {
        self.append_dir(&self.ws_settings.configs_dir)
    }

    pub fn include_dir(&self) -> PathBuf {
        self.append_dir(&self.ws_settings.include_dir)
    }

    pub fn scripts_dir(&self) -> PathBuf {
        self.append_dir(&self.ws_settings.scripts_dir)
    }

    pub fn docker_dir(&self) -> PathBuf {
        self.append_dir(&self.ws_settings.docker_dir)
    }

    pub fn docker_top_dir(&self) -> PathBuf {
        if !self.ws_settings.docker_top_dir.is_empty() {
            return self
                .work_dir()
                .join(self.ws_settings.docker_top_dir.clone());
        }
        return self.work_dir();
    }

    pub fn docker_image(&self) -> DockerImage {
        self.docker.clone()
    }

    pub fn docker_args(&self) -> &Vec<String> {
        &self.ws_settings.docker_args
    }

    pub fn docker_disabled(&self) -> bool {
        match self.ws_settings.docker_disabled.as_str() {
            "true" => {
                return true;
            }
            "false" => {
                return false;
            }
            _ => {
                return false;
            }
        }
    }

    pub fn supported_builds(&self) -> &Vec<String> {
        &self.ws_settings.supported
    }

    pub fn expand_ctx(&mut self, ctx: &Context) -> Result<(), BError> {
        self.ws_settings.expand_ctx(ctx)?;
        Ok(())
    }

    pub fn merge(&mut self, data: &mut WsSettingsHandler) {
        self.ws_settings.merge(&mut data.config().clone());
        self.docker = DockerImage {
            image: self.ws_settings.docker_image.clone(),
            tag: self.ws_settings.docker_tag.clone(),
            registry: self.ws_settings.docker_registry.clone(),
        };
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::executers::DockerImage;
    use crate::helper::Helper;
    use crate::workspace::WsSettingsHandler;

    #[test]
    fn test_settings_default_ws_dirs() {
        let json_test_str = r#"
        {
            "version": "6"
        }"#;
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let settings: WsSettingsHandler =
            WsSettingsHandler::new(work_dir, Helper::setup_ws_settings(json_test_str));
        assert_eq!(settings.builds_dir(), PathBuf::from("/workspace/builds"));
        assert_eq!(settings.cache_dir(), PathBuf::from("/workspace/.cache"));
        assert_eq!(
            settings.artifacts_dir(),
            PathBuf::from("/workspace/artifacts")
        );
        assert_eq!(settings.layers_dir(), PathBuf::from("/workspace/layers"));
        assert_eq!(settings.scripts_dir(), PathBuf::from("/workspace/scripts"));
        assert_eq!(settings.docker_dir(), PathBuf::from("/workspace/docker"));
        assert_eq!(settings.configs_dir(), PathBuf::from("/workspace/configs"));
        assert_eq!(
            settings.include_dir(),
            PathBuf::from("/workspace/configs/include")
        );
    }

    #[test]
    fn test_settings_ws_dirs() {
        let json_test_str = r#"
        {
            "version": "6",
            "workspace": {
              "configsdir": "configs_test",
              "includedir": "include_test",
              "artifactsdir": "artifacts_test",
              "layersdir": "layers_test",
              "buildsdir": "builds_test",
              "artifactsdir": "artifacts_test",
              "scriptsdir": "scripts_test",
              "dockerdir": "docker_test",
              "cachedir": "cache_test"
            }
        }"#;
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let settings: WsSettingsHandler =
            WsSettingsHandler::new(work_dir, Helper::setup_ws_settings(json_test_str));
        assert_eq!(
            settings.builds_dir(),
            PathBuf::from("/workspace/builds_test")
        );
        assert_eq!(settings.cache_dir(), PathBuf::from("/workspace/cache_test"));
        assert_eq!(
            settings.artifacts_dir(),
            PathBuf::from("/workspace/artifacts_test")
        );
        assert_eq!(
            settings.layers_dir(),
            PathBuf::from("/workspace/layers_test")
        );
        assert_eq!(
            settings.scripts_dir(),
            PathBuf::from("/workspace/scripts_test")
        );
        assert_eq!(
            settings.docker_dir(),
            PathBuf::from("/workspace/docker_test")
        );
        assert_eq!(
            settings.configs_dir(),
            PathBuf::from("/workspace/configs_test")
        );
        assert_eq!(
            settings.include_dir(),
            PathBuf::from("/workspace/include_test")
        );
    }

    #[test]
    fn test_settings_ws_top_dir() {
        let json_test_str = r#"
        {
            "version": "6",
            "workspace": {
              "layersdir": ""
            }
        }"#;
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let settings: WsSettingsHandler =
            WsSettingsHandler::new(work_dir, Helper::setup_ws_settings(json_test_str));
        /* Making sure the expanded path doesn't end with '/' */
        assert_eq!(
            settings.layers_dir().to_string_lossy(),
            String::from("/workspace")
        );
        assert_eq!(
            settings.work_dir().to_string_lossy(),
            String::from("/workspace")
        );
    }

    #[test]
    fn test_settings_default_docker() {
        let json_test_str = r#"
        {
            "version": "6"
        }"#;
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let settings: WsSettingsHandler =
            WsSettingsHandler::new(work_dir, Helper::setup_ws_settings(json_test_str));
        let docker_image: DockerImage = settings.docker_image();
        assert_eq!(
            format!("{}", docker_image),
            format!(
                "ghcr.io/yanctab/bakery/bakery-workspace:{}",
                env!("CARGO_PKG_VERSION")
            )
        );
    }

    #[test]
    fn test_settings_docker() {
        let json_test_str = r#"
        {
            "version": "6",
            "docker": {
                "tag": "0.1",
                "image": "test-image",
                "registry": "test-registry"
            }
        }"#;
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let settings: WsSettingsHandler =
            WsSettingsHandler::new(work_dir, Helper::setup_ws_settings(json_test_str));
        let docker_image: DockerImage = settings.docker_image();
        assert_eq!(format!("{}", docker_image), "test-registry/test-image:0.1");
    }

    #[test]
    fn test_settings_default_docker_args() {
        let json_test_str = r#"
        {
            "version": "6",
            "docker": {
                "tag": "0.1",
                "image": "test-image",
                "registry": "test-registry"
            }
        }"#;
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let settings: WsSettingsHandler =
            WsSettingsHandler::new(work_dir, Helper::setup_ws_settings(json_test_str));
        assert!(settings.docker_args().is_empty());
    }

    #[test]
    fn test_settings_docker_args() {
        let json_test_str = r#"
        {
            "version": "6",
            "docker": {
                "tag": "0.1",
                "image": "test-image",
                "registry": "test-registry",
                "args": [
                    "arg1",
                    "arg2",
                    "arg3"
                ]
            }
        }"#;
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let settings: WsSettingsHandler =
            WsSettingsHandler::new(work_dir, Helper::setup_ws_settings(json_test_str));
        assert_eq!(
            settings.docker_args(),
            &vec!["arg1".to_string(), "arg2".to_string(), "arg3".to_string()]
        );
    }

    #[test]
    fn test_settings_default_supported_builds() {
        let json_test_str = r#"
        {
            "version": "6"
        }"#;
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let settings: WsSettingsHandler =
            WsSettingsHandler::new(work_dir, Helper::setup_ws_settings(json_test_str));
        assert!(settings.supported_builds().is_empty());
    }

    #[test]
    fn test_settings_supported_builds() {
        let json_test_str = r#"
        {
            "version": "6",
            "builds": {
                "supported": [
                    "build1",
                    "build2"
                ]
            }
        }"#;
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let settings: WsSettingsHandler =
            WsSettingsHandler::new(work_dir, Helper::setup_ws_settings(json_test_str));
        assert_eq!(
            settings.supported_builds(),
            &vec!["build1".to_string(), "build2".to_string()]
        );
    }
}
