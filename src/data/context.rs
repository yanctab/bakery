use indexmap::{indexmap, IndexMap};
use serde_json::Value;
use std::path::PathBuf;

use crate::configs::Config;
use crate::configs::Context;
use crate::constants::BkryConstants;
use crate::error::BError;

pub struct WsContextData {
    context: Context,
}

/*
 * Built in context variables
 */

/* BKRY Built in variables controlled by the build config or adjusted using one of the options */
pub const CTX_KEY_MACHINE: &str = "BKRY_MACHINE";
pub const CTX_KEY_ARCH: &str = "BKRY_ARCH";
pub const CTX_KEY_DISTRO: &str = "BKRY_DISTRO";
pub const CTX_KEY_PLATFORM_VERSION: &str = "BKRY_PLATFORM_VERSION";
pub const CTX_KEY_BUILD_ID: &str = "BKRY_BUILD_ID";
pub const CTX_KEY_PLATFORM_RELEASE: &str = "BKRY_PLATFORM_RELEASE";
pub const CTX_KEY_BUILD_SHA: &str = "BKRY_BUILD_SHA";
pub const CTX_KEY_BUILD_VARIANT: &str = "BKRY_BUILD_VARIANT";
pub const CTX_KEY_RELEASE_BUILD: &str = "BKRY_RELEASE_BUILD";
pub const CTX_KEY_ARCHIVER: &str = "BKRY_ARCHIVER";
pub const CTX_KEY_DEBUG_SYMBOLS: &str = "BKRY_DEBUG_SYMBOLS";
pub const CTX_KEY_DEVICE: &str = "BKRY_DEVICE";
pub const CTX_KEY_IMAGE: &str = "BKRY_IMAGE";
pub const CTX_KEY_DATE: &str = "BKRY_DATE";
pub const CTX_KEY_TIME: &str = "BKRY_TIME";
pub const CTX_KEY_BRANCH: &str = "BKRY_BRANCH";
pub const CTX_KEY_RESET: &str = "BKRY_RESET";
pub const CTX_KEY_EYECANDY: &str = "BKRY_EYECANDY";
/*
 * TODO: we should clean this up in some way it should
 * not have to be this many context variables for
 * almost the same thing.
 */

/* By default all of these are the same unless they are specificly defined in the build config */
pub const CTX_KEY_PRODUCT_NAME: &str = "BKRY_PRODUCT_NAME";
pub const CTX_KEY_PROJECT_NAME: &str = "BKRY_PROJECT_NAME";
pub const CTX_KEY_CONFIG_NAME: &str = "BKRY_CONFIG_NAME";
pub const CTX_KEY_NAME: &str = "BKRY_NAME";

/* The build config should always match the BKRY_NAME used by the bakery.bashrc */
pub const CTX_KEY_CONFIG: &str = "BKRY_BUILD_CONFIG";

/* BKRY directories either static or set by workspace.json */
pub const CTX_KEY_BB_BUILD_DIR: &str = "BKRY_BB_BUILD_DIR";
pub const CTX_KEY_BB_DEPLOY_DIR: &str = "BKRY_BB_DEPLOY_DIR";
pub const CTX_KEY_ARTIFACTS_DIR: &str = "BKRY_ARTIFACTS_DIR";
pub const CTX_KEY_WORKSPACE_DIR: &str = "BKRY_WORKSPACE_DIR";
pub const CTX_KEY_HOME_CFG_DIR: &str = "BKRY_HOME_CFG_DIR";
pub const CTX_KEY_CFG_DIR: &str = "BKRY_CFG_DIR";
pub const CTX_KEY_BIN_DIR: &str = "BKRY_BIN_DIR";
pub const CTX_KEY_OPT_DIR: &str = "BKRY_OPT_DIR";
pub const CTX_KEY_OPT_SCRIPTS_DIR: &str = "BKRY_OPT_SCRIPTS_DIR";
pub const CTX_KEY_LAYERS_DIR: &str = "BKRY_LAYERS_DIR";
pub const CTX_KEY_SCRIPTS_DIR: &str = "BKRY_SCRIPTS_DIR";
pub const CTX_KEY_BUILDS_DIR: &str = "BKRY_BUILDS_DIR";
pub const CTX_KEY_WORK_DIR: &str = "BKRY_WORK_DIR";

impl Config for WsContextData {}

impl WsContextData {
    fn _env_home() -> String {
        match std::env::var_os("HOME") {
            Some(var) => {
                return var
                    .into_string()
                    .or::<String>(Ok(String::from("")))
                    .unwrap();
            }
            None => {
                return String::new();
            }
        }
    }

    fn _mutable_key(key: &str) -> bool {
        match key {
            CTX_KEY_PLATFORM_VERSION
            | CTX_KEY_BUILD_ID
            | CTX_KEY_PLATFORM_RELEASE
            | CTX_KEY_BUILD_SHA
            | CTX_KEY_BUILD_VARIANT
            | CTX_KEY_RELEASE_BUILD
            | CTX_KEY_ARCHIVER
            | CTX_KEY_DEVICE
            | CTX_KEY_IMAGE
            | CTX_KEY_DATE
            | CTX_KEY_TIME
            | CTX_KEY_BRANCH
            | CTX_KEY_RESET
            | CTX_KEY_EYECANDY
            | CTX_KEY_DEBUG_SYMBOLS => true,
            CTX_KEY_MACHINE
            | CTX_KEY_ARCH
            | CTX_KEY_DISTRO
            | CTX_KEY_ARTIFACTS_DIR
            | CTX_KEY_SCRIPTS_DIR
            | CTX_KEY_BB_BUILD_DIR
            | CTX_KEY_BB_DEPLOY_DIR
            | CTX_KEY_PRODUCT_NAME
            | CTX_KEY_PROJECT_NAME
            | CTX_KEY_NAME
            | CTX_KEY_CONFIG_NAME
            | CTX_KEY_CONFIG
            | CTX_KEY_WORKSPACE_DIR
            | CTX_KEY_WORK_DIR
            | CTX_KEY_LAYERS_DIR
            | CTX_KEY_HOME_CFG_DIR
            | CTX_KEY_BIN_DIR
            | CTX_KEY_OPT_SCRIPTS_DIR
            | CTX_KEY_OPT_DIR
            | CTX_KEY_CFG_DIR
            | CTX_KEY_BUILDS_DIR => false,
            _ => false,
        }
    }

    pub fn from_str(json_string: &str) -> Result<Self, BError> {
        let data: Value = Self::parse(json_string)?;
        Self::from_value(&data)
    }

    pub fn from_value(data: &Value) -> Result<Self, BError> {
        let variables: IndexMap<String, String> = Self::get_hashmap_value("context", &data)?;
        Self::new(&variables)
    }

    pub fn new(variables: &IndexMap<String, String>) -> Result<Self, BError> {
        /*
         * TODO: If any of these variables are set to a non-empty string,
         * they may unintentionally overwrite context variables defined in the build config.
         * This happens because we currently load context variables from the build config first,
         * and then apply these built-in variables afterward.
         *
         * The issue is that some built-in variables derive their values from settings,
         * while others do not. Since we ignore empty strings during updates, they won't overwrite anything.
         * However, if a built-in variable has a default (non-empty) value here,
         * it will overwrite whatever was set by the build config.
         *
         * We may need to revisit the update order or refine the merge logic to prevent unintended overrides.
         */
        let ctx_default_variables: IndexMap<String, String> = indexmap! {
            CTX_KEY_MACHINE.to_string() => "".to_string(),
            CTX_KEY_ARCH.to_string() => "".to_string(),
            CTX_KEY_DISTRO.to_string() => "".to_string(),
            CTX_KEY_PRODUCT_NAME.to_string() => "".to_string(),
            CTX_KEY_PROJECT_NAME.to_string() => "".to_string(),
            CTX_KEY_NAME.to_string() => "".to_string(),
            CTX_KEY_CONFIG_NAME.to_string() => "".to_string(),
            CTX_KEY_BB_BUILD_DIR.to_string() => "".to_string(),
            CTX_KEY_BB_DEPLOY_DIR.to_string() => "".to_string(),
            CTX_KEY_ARTIFACTS_DIR.to_string() => "".to_string(),
            CTX_KEY_LAYERS_DIR.to_string() => "".to_string(),
            CTX_KEY_SCRIPTS_DIR.to_string() => "".to_string(),
            CTX_KEY_BUILDS_DIR.to_string() => "".to_string(),
            CTX_KEY_PLATFORM_VERSION.to_string() => "0.0.0".to_string(),
            CTX_KEY_BUILD_ID.to_string() => "0".to_string(),
            CTX_KEY_PLATFORM_RELEASE.to_string() => "".to_string(),
            CTX_KEY_BUILD_SHA.to_string() => "".to_string(),
            CTX_KEY_RELEASE_BUILD.to_string() => "".to_string(),
            CTX_KEY_BUILD_VARIANT.to_string() => "dev".to_string(),
            CTX_KEY_ARCHIVER.to_string() => "".to_string(),
            CTX_KEY_DEBUG_SYMBOLS.to_string() => "".to_string(),
            CTX_KEY_DEVICE.to_string() => "".to_string(),
            CTX_KEY_IMAGE.to_string() => "".to_string(),
            CTX_KEY_TIME.to_string() => "".to_string(),
            CTX_KEY_DATE.to_string() => "".to_string(),
            CTX_KEY_BRANCH.to_string() => "NA".to_string(),
            CTX_KEY_RESET.to_string() => "false".to_string(),
            CTX_KEY_CONFIG.to_string() => "".to_string(),
            CTX_KEY_EYECANDY.to_string() => "false".to_string(),
            CTX_KEY_WORK_DIR.to_string() => "".to_string(),
            CTX_KEY_WORKSPACE_DIR.to_string() => "".to_string(),
            CTX_KEY_HOME_CFG_DIR.to_string() => format!("{}/.bakery", Self::_env_home()),
            CTX_KEY_CFG_DIR.to_string() => BkryConstants::BKRY_CFG_DIR.to_string(),
            CTX_KEY_BIN_DIR.to_string() => BkryConstants::BKRY_BIN_DIR.to_string(),
            CTX_KEY_OPT_DIR.to_string() => BkryConstants::BKRY_OPT_DIR.to_string(),
            CTX_KEY_OPT_SCRIPTS_DIR.to_string() => BkryConstants::BKRY_OPT_SCRIPTS_DIR.to_string(),
        };
        let mut ctx: Context = Context::new(&ctx_default_variables);
        ctx.update(&variables);
        Ok(WsContextData { context: ctx })
    }

    pub fn _is_mutable(&self, key: &str) -> bool {
        Self::_mutable_key(key)
    }

    pub fn ctx(&self) -> &Context {
        &self.context
    }

    pub fn update(&mut self, variables: &IndexMap<String, String>) {
        let mut v: IndexMap<String, String> = IndexMap::new();
        for (key, value) in variables {
            //println!("key: {}, value: {}", key, value);
            /*
             * Only update the context variable if the new value is not empty.
             * If the value is "NA" (case-insensitive), we skip the update unless the current context value is empty.
             * This ensures that "NA" is treated as a placeholder and doesn't overwrite valid data,
             * but it can be used to initialize an empty field.
             */
            if !value.is_empty() {
                if value.to_lowercase() != "na" || self.context.value(key).is_empty() {
                    //println!("Update: {}={}", key, value);
                    v.insert(key.to_owned(), value.to_owned());
                }
            }
        }
        //println!("{:?}", v);
        self.context.update(&v);
    }

    pub fn update_ctx(&mut self, context: &Context) {
        self.update(context.variables());
    }

    pub fn expand_ctx(&mut self) -> Result<(), BError> {
        self.context.expand()
    }

    pub fn get_ctx_path(&self, key: &str) -> PathBuf {
        PathBuf::from(self.get_ctx_value(key))
    }

    pub fn get_ctx_value(&self, key: &str) -> String {
        self.context.value(key)
    }
}

#[cfg(test)]
mod tests {
    use indexmap::{indexmap, IndexMap};
    use std::path::PathBuf;

    use crate::constants::BkryConstants;
    use crate::data::context::{
        CTX_KEY_ARCH, CTX_KEY_ARCHIVER, CTX_KEY_ARTIFACTS_DIR, CTX_KEY_BB_BUILD_DIR,
        CTX_KEY_BB_DEPLOY_DIR, CTX_KEY_BIN_DIR, CTX_KEY_BRANCH, CTX_KEY_BUILDS_DIR,
        CTX_KEY_BUILD_ID, CTX_KEY_BUILD_SHA, CTX_KEY_BUILD_VARIANT, CTX_KEY_CFG_DIR,
        CTX_KEY_CONFIG, CTX_KEY_CONFIG_NAME, CTX_KEY_DATE, CTX_KEY_DEBUG_SYMBOLS, CTX_KEY_DEVICE,
        CTX_KEY_DISTRO, CTX_KEY_EYECANDY, CTX_KEY_HOME_CFG_DIR, CTX_KEY_IMAGE, CTX_KEY_LAYERS_DIR,
        CTX_KEY_MACHINE, CTX_KEY_NAME, CTX_KEY_OPT_DIR, CTX_KEY_OPT_SCRIPTS_DIR,
        CTX_KEY_PLATFORM_RELEASE, CTX_KEY_PLATFORM_VERSION, CTX_KEY_PRODUCT_NAME,
        CTX_KEY_PROJECT_NAME, CTX_KEY_RELEASE_BUILD, CTX_KEY_RESET, CTX_KEY_SCRIPTS_DIR,
        CTX_KEY_TIME, CTX_KEY_WORKSPACE_DIR, CTX_KEY_WORK_DIR,
    };
    use crate::data::WsContextData;
    use crate::workspace::WsSettingsHandler;

    #[test]
    fn test_ws_context_data_default() {
        let json_default_build_config = r#"
        {
            "version": "6"
        }"#;
        let data: WsContextData = WsContextData::from_str(json_default_build_config)
            .expect("Failed to parse context data");
        assert!(data.get_ctx_value(CTX_KEY_MACHINE).is_empty());
        assert!(data.get_ctx_value(CTX_KEY_ARCH).is_empty());
        assert!(data.get_ctx_value(CTX_KEY_DISTRO).is_empty());
        assert!(data.get_ctx_value(CTX_KEY_PRODUCT_NAME).is_empty());
        assert!(data.get_ctx_value(CTX_KEY_PROJECT_NAME).is_empty());
        assert!(data.get_ctx_value(CTX_KEY_CONFIG_NAME).is_empty());
        assert!(data.get_ctx_value(CTX_KEY_NAME).is_empty());
        assert!(data.get_ctx_value(CTX_KEY_DEVICE).is_empty());
        assert!(data.get_ctx_value(CTX_KEY_IMAGE).is_empty());
        assert_eq!(data.get_ctx_path(CTX_KEY_BB_BUILD_DIR), PathBuf::from(""));
        assert_eq!(data.get_ctx_path(CTX_KEY_BB_DEPLOY_DIR), PathBuf::from(""));
        assert_eq!(data.get_ctx_path(CTX_KEY_ARTIFACTS_DIR), PathBuf::from(""));
        assert_eq!(data.get_ctx_path(CTX_KEY_LAYERS_DIR), PathBuf::from(""));
        assert_eq!(data.get_ctx_path(CTX_KEY_SCRIPTS_DIR), PathBuf::from(""));
        assert_eq!(data.get_ctx_path(CTX_KEY_BUILDS_DIR), PathBuf::from(""));
        assert_eq!(
            data.get_ctx_value(CTX_KEY_PLATFORM_VERSION),
            String::from("0.0.0")
        );
        assert_eq!(data.get_ctx_value(CTX_KEY_BUILD_ID), String::from("0"));
        assert_eq!(data.get_ctx_path(CTX_KEY_WORK_DIR), PathBuf::from(""));
        assert_eq!(data.get_ctx_path(CTX_KEY_WORKSPACE_DIR), PathBuf::from(""));
        assert_eq!(
            data.get_ctx_value(CTX_KEY_BUILD_VARIANT),
            String::from("dev")
        );
        assert_eq!(data.get_ctx_value(CTX_KEY_DATE), String::from(""));
        assert_eq!(data.get_ctx_value(CTX_KEY_TIME), String::from(""));
        assert!(data.get_ctx_value(CTX_KEY_PLATFORM_RELEASE).is_empty());
        assert!(data.get_ctx_value(CTX_KEY_BUILD_SHA).is_empty());
        assert!(data.get_ctx_value(CTX_KEY_RELEASE_BUILD).is_empty());
        assert!(data.get_ctx_value(CTX_KEY_ARCHIVER).is_empty());
        assert!(data.get_ctx_value(CTX_KEY_DEBUG_SYMBOLS).is_empty());
        assert_eq!(data.get_ctx_value(CTX_KEY_BRANCH), String::from("NA"));
        assert_eq!(data.get_ctx_value(CTX_KEY_RESET), String::from("false"));
        assert_eq!(data.get_ctx_value(CTX_KEY_CONFIG), String::from(""));
        assert_eq!(data.get_ctx_value(CTX_KEY_EYECANDY), String::from("false"));
        assert_eq!(
            data.get_ctx_value(CTX_KEY_HOME_CFG_DIR),
            format!(
                "{}/.bakery",
                std::env::var_os("HOME").unwrap().into_string().unwrap()
            )
        );
        assert_eq!(
            data.get_ctx_value(CTX_KEY_CFG_DIR),
            String::from(BkryConstants::BKRY_CFG_DIR)
        );
        assert_eq!(
            data.get_ctx_value(CTX_KEY_BIN_DIR),
            String::from(BkryConstants::BKRY_BIN_DIR)
        );
        assert_eq!(
            data.get_ctx_value(CTX_KEY_OPT_DIR),
            String::from(BkryConstants::BKRY_OPT_DIR)
        );
        assert_eq!(
            data.get_ctx_value(CTX_KEY_OPT_SCRIPTS_DIR),
            String::from(BkryConstants::BKRY_OPT_SCRIPTS_DIR)
        );
    }

    #[test]
    fn test_ws_context_data() {
        let json_settings: &str = r#"
        {
            "version": "6"
        }"#;
        let json_build_config = r#"
        {
            "version": "6",
            "context": [
                "KEY1=value1",
                "KEY2=value2",
                "KEY3=value3"
            ]
        }"#;
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let settings: WsSettingsHandler =
            WsSettingsHandler::from_str(&work_dir, json_settings, None)
                .expect("Failed to parse settings");
        let mut data: WsContextData =
            WsContextData::from_str(json_build_config).expect("Failed to parse context data");
        let ctx_built_in_variables: IndexMap<String, String> = indexmap! {
            CTX_KEY_MACHINE.to_string() => "test-machine".to_string(),
            CTX_KEY_ARCH.to_string() => "test-arch".to_string(),
            CTX_KEY_DISTRO.to_string() => "test-distro".to_string(),
            CTX_KEY_BUILD_VARIANT.to_string() => "test-variant".to_string(),
            CTX_KEY_PRODUCT_NAME.to_string() => "test".to_string(),
            CTX_KEY_WORK_DIR.to_string() => settings.work_dir().to_string_lossy().to_string(),
            CTX_KEY_WORKSPACE_DIR.to_string() => settings.workspace_dir().to_string_lossy().to_string(),
        };
        data.update(&ctx_built_in_variables);
        assert_eq!(data.get_ctx_value(CTX_KEY_MACHINE), "test-machine");
        assert_eq!(data.get_ctx_value(CTX_KEY_ARCH), "test-arch");
        assert_eq!(data.get_ctx_value(CTX_KEY_DISTRO), "test-distro");
        assert_eq!(data.get_ctx_value(CTX_KEY_BUILD_VARIANT), "test-variant");
        assert_eq!(data.get_ctx_value(CTX_KEY_PRODUCT_NAME), "test");
        assert_eq!(data.get_ctx_value("KEY1"), "value1");
        assert_eq!(data.get_ctx_value("KEY2"), "value2");
        assert_eq!(data.get_ctx_value("KEY3"), "value3");
        assert_eq!(data.get_ctx_path(CTX_KEY_WORK_DIR), settings.work_dir());
        assert_eq!(
            data.get_ctx_path(CTX_KEY_WORKSPACE_DIR),
            settings.workspace_dir()
        );
    }

    #[test]
    fn test_ws_builtin_context_data() {
        let json_build_config = r#"
        {
            "version": "6",
            "context": [
                "BKRY_IMAGE=image",
                "BKRY_DEVICE=device",
                "BKRY_BRANCH=branch",
                "BKRY_RESET=true",
                "BKRY_EYECANDY=true"
            ]
        }"#;
        let data: WsContextData =
            WsContextData::from_str(json_build_config).expect("Failed to parse context data");
        assert_eq!(data.get_ctx_value(CTX_KEY_IMAGE), "image");
        assert_eq!(data.get_ctx_value(CTX_KEY_DEVICE), "device");
        assert_eq!(data.get_ctx_value(CTX_KEY_BRANCH), "branch");
        assert_eq!(data.get_ctx_value(CTX_KEY_RESET), "true");
        assert_eq!(data.get_ctx_value(CTX_KEY_EYECANDY), "true");
    }

    #[test]
    fn test_ws_context_chain() {
        let json_build_config = r#"
        {
            "version": "6",
            "context": [
                "BKRY_IMAGE=image",
                "TEST_CTX_CHAIN=test/$#[BKRY_IMAGE]"
            ]
        }"#;
        let mut data: WsContextData =
            WsContextData::from_str(json_build_config).expect("Failed to parse context data");
        data.expand_ctx().expect("expand ctx failed with error");
        assert_eq!(data.get_ctx_value(CTX_KEY_IMAGE), "image");
        assert_eq!(data.get_ctx_value("TEST_CTX_CHAIN"), "test/image");
    }
}
