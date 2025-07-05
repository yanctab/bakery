pub struct BkryConstants;
impl BkryConstants {
    pub const _DOCKER_ARGS: [&'static str; 2] = ["--rm=true", "-t"];
    pub const DOCKER_IMAGE: &'static str = "yanctab/bakery/bakery-workspace";
    pub const DOCKER_TAG: &'static str = env!("CARGO_PKG_VERSION");
    pub const DOCKER_REGISTRY: &'static str = "ghcr.io";
    pub const _WS_SETTINGS_VERSION: &'static str = "6";
    pub const BUILD_CFG_VERSION: &'static str = "6";
    pub const WS_SETTINGS: &str = "workspace.json";
    pub const OPT_DIR: &str = "/opt/bakery";
    pub const CFG_DIR: &str = "/etc/bakery";
    pub const BIN: &str = "/usr/bin/bakery";
    pub const BIN_DIR: &str = "/usr/bin";
    pub const OPT_SCRIPTS_DIR: &str = "/opt/bakery/scripts";
}
