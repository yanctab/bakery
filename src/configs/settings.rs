use crate::configs::Config;
use crate::error::BError;
use serde_json::Value;

// Not the ideal solution we should see if it is possible to
// read them from the Cargo.toml and then incorporate them
// into the binary
pub const _BAKERY_DOCKER_ARGS: [&str; 2] = ["--rm=true", "-t"];
pub const BAKERY_DOCKER_IMAGE: &str = "yanctab/bakery/bakery-workspace";
pub const BAKERY_DOCKER_TAG: &str = env!("CARGO_PKG_VERSION");
pub const BAKERY_DOCKER_REGISTRY: &str = "ghcr.io";

#[derive(Clone)]
pub struct WsSettings {
    pub version: String,
    pub configs_dir: String,
    pub builds_dir: String,
    pub artifacts_dir: String,
    pub layers_dir: String,
    pub include_dir: String,
    pub scripts_dir: String,
    pub docker_dir: String,
    pub cache_dir: String,
    pub supported: Vec<String>,
    pub docker_tag: String,
    pub docker_image: String,
    pub docker_registry: String,
    pub docker_args: Vec<String>,
    pub docker_disabled: String,
    pub docker_top_dir: String,
}

impl Config for WsSettings {}

impl WsSettings {
    pub fn from_str(json_string: &str) -> Result<Self, BError> {
        let data: Value = Self::parse(json_string)?;
        let version: String = Self::get_str_value("version", &data, None)?;
        let mut configs_dir: String = String::from("configs");
        let mut include_dir: String = String::from("configs/include");
        let mut builds_dir: String = String::from("builds");
        let mut artifacts_dir: String = String::from("artifacts");
        let mut layers_dir: String = String::from("layers");
        let mut scripts_dir: String = String::from("scripts");
        let mut docker_dir: String = String::from("docker");
        let mut cache_dir: String = String::from(".cache");
        let supported: Vec<String>;
        let mut docker_image: String = String::from(BAKERY_DOCKER_IMAGE);
        let mut docker_tag: String = String::from(BAKERY_DOCKER_TAG);
        let mut docker_registry: String = String::from(BAKERY_DOCKER_REGISTRY);
        let mut docker_args: Vec<String> = vec![];
        let mut docker_disabled: String = String::from("false");
        let mut docker_top_dir: String = String::from("");

        match Self::get_value("workspace", &data) {
            Ok(ws_data) => {
                configs_dir =
                    Self::get_str_value("configsdir", ws_data, Some(String::from("configs")))?;
                include_dir = Self::get_str_value(
                    "includedir",
                    ws_data,
                    Some(String::from("configs/include")),
                )?;
                builds_dir =
                    Self::get_str_value("buildsdir", ws_data, Some(String::from("builds")))?;
                artifacts_dir =
                    Self::get_str_value("artifactsdir", ws_data, Some(String::from("artifacts")))?;
                layers_dir =
                    Self::get_str_value("layersdir", ws_data, Some(String::from("layers")))?;
                scripts_dir =
                    Self::get_str_value("scriptsdir", ws_data, Some(String::from("scripts")))?;
                docker_dir =
                    Self::get_str_value("dockerdir", ws_data, Some(String::from("docker")))?;
                cache_dir = Self::get_str_value("cachedir", ws_data, Some(String::from(".cache")))?;
            }
            Err(_err) => {}
        }

        match Self::get_value("builds", &data) {
            Ok(build_data) => {
                supported = Self::get_array_value("supported", build_data, Some(vec![]))?;
            }
            Err(_err) => {
                supported = vec![];
            }
        }

        match Self::get_value("docker", &data) {
            Ok(docker_data) => {
                docker_disabled =
                    Self::get_str_value("disabled", docker_data, Some(String::from("false")))?;
                docker_image = Self::get_str_value(
                    "image",
                    docker_data,
                    Some(String::from(BAKERY_DOCKER_IMAGE)),
                )?;
                docker_tag =
                    Self::get_str_value("tag", docker_data, Some(String::from(BAKERY_DOCKER_TAG)))?;
                docker_registry = Self::get_str_value(
                    "registry",
                    docker_data,
                    Some(String::from(BAKERY_DOCKER_REGISTRY)),
                )?;
                docker_args = Self::get_array_value("args", docker_data, Some(vec![]))?;
                docker_top_dir =
                    Self::get_str_value("topdir", docker_data, Some(String::from("")))?;
            }
            Err(_err) => {}
        }

        Ok(WsSettings {
            version,
            configs_dir,
            include_dir,
            builds_dir,
            artifacts_dir,
            layers_dir,
            scripts_dir,
            docker_dir,
            cache_dir,
            supported,
            docker_tag,
            docker_image,
            docker_registry,
            docker_args,
            docker_disabled,
            docker_top_dir,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::helper::Helper;

    #[test]
    fn test_settings_config_workspace_dirs() {
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
        let settings = Helper::setup_ws_settings(json_test_str);
        assert_eq!(&settings.configs_dir, "configs_test");
        assert_eq!(&settings.include_dir, "include_test");
        assert_eq!(&settings.artifacts_dir, "artifacts_test");
        assert_eq!(&settings.layers_dir, "layers_test");
        assert_eq!(&settings.builds_dir, "builds_test");
        assert_eq!(&settings.scripts_dir, "scripts_test");
        assert_eq!(&settings.docker_dir, "docker_test");
        assert_eq!(&settings.cache_dir, "cache_test");
    }

    #[test]
    fn test_settings_config_default_workspace_dirs() {
        let json_test_str = r#"
        {
            "version": "6"
        }"#;
        let settings = Helper::setup_ws_settings(json_test_str);
        assert_eq!(&settings.configs_dir, "configs");
        assert_eq!(&settings.include_dir, "configs/include");
        assert_eq!(&settings.artifacts_dir, "artifacts");
        assert_eq!(&settings.layers_dir, "layers");
        assert_eq!(&settings.builds_dir, "builds");
        assert_eq!(&settings.scripts_dir, "scripts");
        assert_eq!(&settings.docker_dir, "docker");
        assert_eq!(&settings.cache_dir, ".cache");
    }

    #[test]
    fn test_settings_config_no_configs_workspace_node() {
        let json_test_str = r#"
        {
            "version": "6"
        }"#;
        let settings = Helper::setup_ws_settings(json_test_str);
        assert_eq!(&settings.configs_dir, "configs");
    }

    #[test]
    fn test_settings_config_no_builds_dir() {
        let json_test_str = r#"
        {
            "version": "6",
            "workspace": {
              "artifactsdir": "artifacts_test"
            }
        }"#;
        let settings = Helper::setup_ws_settings(json_test_str);
        assert_eq!(&settings.builds_dir, "builds");
    }

    #[test]
    fn test_settings_config_no_builds_workspace_node() {
        let json_test_str = r#"
        {
            "version": "6"
        }"#;
        let settings = Helper::setup_ws_settings(json_test_str);
        assert_eq!(&settings.builds_dir, "builds");
    }

    #[test]
    fn test_settings_config_no_artifacts_dir() {
        let json_test_str = r#"
        {
            "version": "6",
            "workspace": {
              "buildsdir": "builds_test"
            }
        }"#;
        let settings = Helper::setup_ws_settings(json_test_str);
        assert_eq!(&settings.artifacts_dir, "artifacts");
    }

    #[test]
    fn test_settings_config_no_artifacts_workspace_node() {
        let json_test_str = r#"
        {
            "version": "6"
        }"#;
        let settings = Helper::setup_ws_settings(json_test_str);
        assert_eq!(&settings.artifacts_dir, "artifacts");
    }

    #[test]
    fn test_settings_config_no_scripts_dir() {
        let json_test_str = r#"
        {
            "version": "6",
            "workspace": {
              "buildsdir": "builds_test"
            }
        }"#;
        let settings = Helper::setup_ws_settings(json_test_str);
        assert_eq!(&settings.scripts_dir, "scripts");
    }

    #[test]
    fn test_settings_config_no_scripts_workspace_node() {
        let json_test_str = r#"
        {
            "version": "6"
        }"#;
        let settings = Helper::setup_ws_settings(json_test_str);
        assert_eq!(&settings.scripts_dir, "scripts");
    }

    #[test]
    fn test_settings_config_no_docker_dir() {
        let json_test_str = r#"
        {
            "version": "6",
            "workspace": {
              "buildsdir": "builds_test"
            }
        }"#;
        let settings = Helper::setup_ws_settings(json_test_str);
        assert_eq!(&settings.docker_dir, "docker");
    }

    #[test]
    fn test_settings_config_no_docker_workspace_node() {
        let json_test_str = r#"
        {
            "version": "6"
        }"#;
        let settings = Helper::setup_ws_settings(json_test_str);
        assert_eq!(&settings.docker_dir, "docker");
    }

    #[test]
    fn test_settings_config_no_cache_dir() {
        let json_test_str = r#"
        {
            "version": "6",
            "workspace": {
              "buildsdir": "builds_test"
            }
        }"#;
        let settings = Helper::setup_ws_settings(json_test_str);
        assert_eq!(&settings.cache_dir, ".cache");
    }

    #[test]
    fn test_settings_config_no_cache_workspace_node() {
        let json_test_str = r#"
        {
            "version": "6"
        }"#;
        let settings = Helper::setup_ws_settings(json_test_str);
        assert_eq!(&settings.cache_dir, ".cache");
    }

    #[test]
    fn test_settings_config_docker_image() {
        let json_test_str = r#"
        {
            "version": "6",
            "docker": {
                "image": "test-workspace"
            }
        }"#;
        let settings = Helper::setup_ws_settings(json_test_str);
        assert_eq!(&settings.docker_image, "test-workspace");
    }

    #[test]
    fn test_settings_config_no_docker_image() {
        let json_test_str = r#"
        {
            "version": "6",
            "docker": {
                "tag": "0.1"
            }
        }"#;
        let settings = Helper::setup_ws_settings(json_test_str);
        assert_eq!(
            &settings.docker_image,
            "yanctab/bakery/bakery-workspace"
        );
    }

    #[test]
    fn test_settings_config_no_docker_image_node() {
        let json_test_str = r#"
        {
            "version": "6"
        }"#;
        let settings = Helper::setup_ws_settings(json_test_str);
        assert_eq!(
            &settings.docker_image,
            "yanctab/bakery/bakery-workspace"
        );
    }

    #[test]
    fn test_settings_config_docker_tag() {
        let json_test_str = r#"
        {
            "version": "6",
            "docker": {
                "tag": "0.1"
            }
        }"#;
        let settings = Helper::setup_ws_settings(json_test_str);
        assert_eq!(&settings.docker_tag, "0.1");
    }

    #[test]
    fn test_settings_config_no_docker_tag() {
        let json_test_str = r#"
        {
            "version": "6",
            "docker": {
                "image": "test-workspace"
            }
        }"#;
        let settings = Helper::setup_ws_settings(json_test_str);
        assert_eq!(&settings.docker_tag, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_settings_config_no_docker_tag_node() {
        let json_test_str = r#"
        {
            "version": "6"
        }"#;
        let settings = Helper::setup_ws_settings(json_test_str);
        assert_eq!(&settings.docker_tag, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_settings_config_default_docker_enabled() {
        let json_test_str = r#"
        {
            "version": "6"
        }"#;
        let settings = Helper::setup_ws_settings(json_test_str);
        assert_eq!(&settings.docker_disabled, "false");
    }

    #[test]
    fn test_settings_config_docker_disabled() {
        let json_test_str = r#"
        {
            "version": "6",
            "docker": {
                "disabled": "true"
            }
        }"#;
        let settings = Helper::setup_ws_settings(json_test_str);
        assert_eq!(&settings.docker_disabled, "true");
    }

    #[test]
    fn test_settings_config_docker_registry() {
        let json_test_str = r#"
        {
            "version": "6",
            "docker": {
                "registry": "test-registry"
            }
        }"#;
        let settings = Helper::setup_ws_settings(json_test_str);
        assert_eq!(&settings.docker_registry, "test-registry");
    }

    #[test]
    fn test_settings_config_no_docker_registry() {
        let json_test_str = r#"
        {
            "version": "6",
            "docker": {
                "image": "test-workspace"
            }
        }"#;
        let settings = Helper::setup_ws_settings(json_test_str);
        assert_eq!(&settings.docker_registry, "ghcr.io");
    }

    #[test]
    fn test_settings_config_no_docker_registry_node() {
        let json_test_str = r#"
        {
            "version": "6"
        }"#;
        let settings = Helper::setup_ws_settings(json_test_str);
        assert_eq!(&settings.docker_registry, "ghcr.io");
    }

    #[test]
    fn test_settings_config_docker_args() {
        let json_test_str = r#"
        {
            "version": "6",
            "docker": {
                "args": [
                  "--rm=true",
                  "-t",
                  "--dns=8.8.8.8"
                ]
              }
        }"#;
        let settings = Helper::setup_ws_settings(json_test_str);
        assert_eq!(
            &settings.docker_args,
            &vec![
                String::from("--rm=true"),
                String::from("-t"),
                String::from("--dns=8.8.8.8")
            ]
        );
    }

    #[test]
    fn test_settings_config_no_docker_args() {
        let json_test_str = r#"
        {
            "version": "6"
        }"#;
        let settings = Helper::setup_ws_settings(json_test_str);
        assert!(&settings.docker_args.is_empty());
    }

    #[test]
    fn test_settings_config_build_configs() {
        let json_test_str = r#"
        {
            "version": "6",
            "builds": {
                "supported": [
                  "machine1-test",
                  "machine2-test"
                ]
            }
        }"#;
        let settings = Helper::setup_ws_settings(json_test_str);
        assert_eq!(
            &settings.supported,
            &vec![String::from("machine1-test"), String::from("machine2-test")]
        );
        let mut i: i32 = 1;
        for supported in &settings.supported {
            assert_eq!(supported, &format!("machine{}-test", i));
            i += 1;
        }
    }

    #[test]
    fn test_settings_config_no_supported_build_configs() {
        let json_test_str = r#"
        {
            "version": "6",
            "builds": {
            }
        }"#;
        let settings = Helper::setup_ws_settings(json_test_str);
        assert_eq!(settings.supported.is_empty(), true);
    }

    #[test]
    fn test_settings_config_no_build_node() {
        let json_test_str = r#"
        {
            "version": "6"
        }"#;
        let settings = Helper::setup_ws_settings(json_test_str);
        assert_eq!(settings.supported.is_empty(), true);
    }
}
