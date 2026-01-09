pub struct BkryConstants;
impl BkryConstants {
    pub const _DOCKER_ARGS: [&'static str; 2] = ["--rm=true", "-t"];
    pub const DOCKER_IMAGE: &'static str = "yanctab/bakery/bakery-workspace";
    pub const DOCKER_TAG: &'static str = env!("CARGO_PKG_VERSION");
    pub const DOCKER_REGISTRY: &'static str = "ghcr.io";
    pub const _WS_SETTINGS_VERSION: &'static str = "6";
    pub const BUILD_CFG_VERSION: &'static str = "6";
    pub const WS_SETTINGS: &str = "workspace.json";
    pub const _WS_HIDDEN_SETTINGS: &str = ".workspace.json";
    pub const BKRY_OPT_DIR: &str = "/opt/bakery";
    pub const BKRY_CFG_DIR: &str = "/etc/bakery";
    pub const BKRY_BIN: &str = "/usr/bin/bakery";
    pub const BKRY_BIN_DIR: &str = "/usr/bin";
    pub const BKRY_OPT_SCRIPTS_DIR: &str = "/opt/bakery/scripts";
    pub const BKRY_DEFAULT_CFG_DIR: &str = "configs";
    pub const BKRY_DEFAULT_INCLUDE_CFG_DIR: &str = "configs/include";
    pub const BKRY_DEFAULT_ARTIFACTS_DIR: &str = "artifacts";
    pub const BKRY_DEFAULT_BUILDS_DIR: &str = "builds";
    pub const BKRY_DEFAULT_LAYERS_DIR: &str = "layers";
    pub const BKRY_DEFAULT_DOCKER_DIR: &str = "docker";
    pub const BKRY_DEFAULT_CACHE_DIR: &str = ".cache";
    pub const BKRY_DEFAULT_SCRIPTS_DIR: &str = "scripts";
}
