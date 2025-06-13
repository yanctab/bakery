use serde_json::Value;
use std::path::PathBuf;

use crate::configs::Config;
use crate::configs::Context;
use crate::error::BError;
use crate::workspace::WsSettingsHandler;

pub struct WsBitbakeData {
    data: Value,
    product: String, // This is required and is not part of the bitbake segment but is used when putting the bitbake data together
    arch: String, // This is required and is not part of the bitbake segment but is used when putting the bitbake data together
    machine: String, // Optional but if there is a task with type bitbake defined it might fail
    distro: String, // Optional but if there is a task with type bitbake defined it might fail
    deploy_dir: String, // Optional if not set the default deploy dir will be used builds/tmp/deploydir
    docker: String, // Optional if nothing is set the bitbake task will be executed inside the bakery container. Default is an empty string
    bblayers_conf: Vec<String>, // Optional but if there is a task with type bitbake defined it will fail without a bblayers.conf
    local_conf: Vec<String>, // Optional but if there is a task with type bitbake defined it will fail without a local.conf
    init_env: String, // Optional but if not set the default oe-init-build-env file will be used
    settings: WsSettingsHandler,
}

impl Config for WsBitbakeData {}

impl WsBitbakeData {
    pub fn from_str(json_string: &str, settings: &WsSettingsHandler) -> Result<Self, BError> {
        let data: Value = Self::parse(json_string)?;
        Self::from_value(&data, settings)
    }

    pub fn from_value(data: &Value, settings: &WsSettingsHandler) -> Result<Self, BError> {
        let mut bb_data: &Value = data;
        match data.get("bb") {
            Some(value) => {
                bb_data = value;
            }
            None => {}
        }
        // Not part of the bitbake segment in the build config but is required when putting the bitbake
        // build data together
        let product: String = Self::get_str_value("name", data, Some(String::from("NA")))?;
        // Not part of the bitbake segment in the build config but is required when putting the bitbake
        // build data together
        let arch: String = Self::get_str_value("arch", data, Some(String::from("NA")))?;
        let machine: String = Self::get_str_value("machine", bb_data, Some(String::from("NA")))?;
        let distro: String = Self::get_str_value("distro", bb_data, Some(String::from("NA")))?;
        let docker: String = Self::get_str_value("docker", bb_data, Some(String::from("NA")))?;
        let deploy_dir: String = Self::get_str_value(
            "deploydir",
            bb_data,
            Some(String::from("tmp/deploy/images")),
        )?;
        let bblayers_conf: Vec<String> =
            Self::get_array_value("bblayersconf", bb_data, Some(vec![]))?;
        let local_conf: Vec<String> = Self::get_array_value("localconf", bb_data, Some(vec![]))?;
        let init_env: String = Self::get_str_value(
            "initenv",
            bb_data,
            Some(String::from("layers/poky/oe-init-build-env")),
        )?;

        Ok(WsBitbakeData {
            data: bb_data.clone(),
            product,
            arch,
            machine,
            distro,
            docker,
            deploy_dir,
            bblayers_conf,
            local_conf,
            settings: settings.clone(),
            init_env,
        })
    }

    pub fn expand_conf(&self, ctx: &Context, conf: &Vec<String>) -> Result<Vec<String>, BError> {
        let mut config: Vec<String> = Vec::new();
        for line in conf.clone() {
            let expanded: String = ctx.expand_str(&line)?;
            config.push(expanded);
        }

        Ok(config)
    }

    pub fn expand_ctx(&mut self, ctx: &Context) -> Result<(), BError> {
        self.machine = ctx.expand_str(&self.machine)?;
        self.distro = ctx.expand_str(&self.distro)?;
        self.docker = ctx.expand_str(&self.docker)?;
        self.deploy_dir = ctx.expand_str(&self.deploy_dir)?;
        self.init_env = ctx.expand_str(&self.init_env)?;
        self.bblayers_conf = self.expand_conf(ctx, &self.bblayers_conf)?;
        self.local_conf = self.expand_conf(ctx, &self.local_conf)?;
        Ok(())
    }

    pub fn bblayers_conf(&self) -> String {
        let mut conf_str: String = String::new();
        for line in self.bblayers_conf.clone() {
            conf_str.push_str(format!("{}\n", line).as_str());
        }
        conf_str
    }

    pub fn local_conf(&self) -> String {
        let mut conf_str: String = String::new();
        for line in self.local_conf.clone() {
            conf_str.push_str(format!("{}\n", line).as_str());
        }
        conf_str.push_str(&format!("MACHINE ?= \"{}\"\n", self.machine()));
        // TODO: we should define a method product_name() call that instead
        conf_str.push_str(&format!("PRODUCT_NAME ?= \"{}\"\n", self.product));
        conf_str.push_str(&format!("DISTRO ?= \"{}\"\n", self.distro));
        conf_str.push_str(&format!(
            "SSTATE_DIR ?= \"{}\"\n",
            self.sstate_dir().to_str().unwrap()
        ));
        conf_str.push_str(&format!(
            "DL_DIR ?= \"{}\"\n",
            self.dl_dir().to_str().unwrap()
        ));
        conf_str
    }

    pub fn to_string(&self) -> String {
        let bb_str: String = format!("\"bb\": {}", self.data.to_string());
        bb_str.clone()
    }

    pub fn machine(&self) -> &str {
        &self.machine
    }

    pub fn distro(&self) -> &str {
        &self.distro
    }

    pub fn build_dir(&self) -> PathBuf {
        self.settings
            .builds_dir()
            .clone()
            .join(PathBuf::from(self.product.clone()))
    }

    pub fn docker_image(&self) -> &str {
        &self.docker
    }

    pub fn build_config_dir(&self) -> PathBuf {
        self.build_dir().join("conf".to_string())
    }

    pub fn local_conf_path(&self) -> PathBuf {
        self.build_config_dir().join("local.conf".to_string())
    }

    pub fn bblayers_conf_path(&self) -> PathBuf {
        self.build_config_dir().join("bblayers.conf")
    }

    pub fn deploy_dir(&self) -> PathBuf {
        self.build_dir()
            .join(PathBuf::from(self.deploy_dir.clone()))
    }

    pub fn sstate_dir(&self) -> PathBuf {
        self.settings
            .cache_dir()
            .clone()
            .join(&self.arch)
            .join("sstate-cache".to_string())
    }

    pub fn dl_dir(&self) -> PathBuf {
        self.settings
            .cache_dir()
            .clone()
            .join("download".to_string())
    }

    pub fn init_env_file(&self) -> PathBuf {
        // TODO: we should probably setup an option to configure what OE init script
        // to source to setup the env.
        self.settings.work_dir().clone().join(self.init_env.clone())
    }
}

#[cfg(test)]
mod tests {
    use indexmap::{indexmap, IndexMap};
    use std::path::PathBuf;

    use crate::configs::Context;
    use crate::data::WsBitbakeData;
    use crate::workspace::WsSettingsHandler;

    #[test]
    fn test_ws_bitbake_data_default() {
        let json_settings: &str = r#"
        {
            "version": "6"
        }"#;
        let json_build_config = r#"
        {
            "version": "6"
        }"#;
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let settings: WsSettingsHandler =
            WsSettingsHandler::from_str(&work_dir, json_settings, None)
                .expect("Failed to parse settings");
        let data: WsBitbakeData = WsBitbakeData::from_str(json_build_config, &settings)
            .expect("Failed to parse product data");
        assert_eq!(data.machine(), "NA");
        assert_eq!(data.distro(), "NA");
        assert_eq!(data.docker_image(), "NA");
        assert_eq!(
            data.build_dir(),
            PathBuf::from(String::from("/workspace/builds/NA"))
        );
        assert_eq!(
            data.build_config_dir(),
            PathBuf::from(String::from("/workspace/builds/NA/conf"))
        );
        assert_eq!(
            data.deploy_dir(),
            PathBuf::from(String::from("/workspace/builds/NA/tmp/deploy/images"))
        );
        assert_eq!(
            data.bblayers_conf_path(),
            PathBuf::from(String::from("/workspace/builds/NA/conf/bblayers.conf"))
        );
        assert_eq!(
            data.local_conf_path(),
            PathBuf::from(String::from("/workspace/builds/NA/conf/local.conf"))
        );
        assert!(data.bblayers_conf().is_empty());
        assert!(!data.local_conf().is_empty());
        assert_eq!(
            data.sstate_dir(),
            PathBuf::from(String::from("/workspace/.cache/NA/sstate-cache"))
        );
        assert_eq!(
            data.dl_dir(),
            PathBuf::from(String::from("/workspace/.cache/download"))
        );
        assert_eq!(
            data.init_env_file(),
            PathBuf::from(String::from("/workspace/layers/poky/oe-init-build-env"))
        );
    }

    #[test]
    fn test_ws_bitbake_data() {
        let json_settings: &str = r#"
        {
            "version": "6"
        }"#;
        let json_build_config = r#"
        {
            "version": "6",
            "name": "test-name",
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
            }
        }"#;
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let settings: WsSettingsHandler =
            WsSettingsHandler::from_str(&work_dir, json_settings, None)
                .expect("Failed to parse settings");
        let data: WsBitbakeData = WsBitbakeData::from_str(json_build_config, &settings)
            .expect("Failed to parse product data");
        assert_eq!(data.machine(), "test-machine");
        assert_eq!(data.distro(), "test-distro");
        assert_eq!(data.docker_image(), "test-registry/test-image:0.1");
        assert_eq!(
            data.build_dir(),
            PathBuf::from(String::from("/workspace/builds/test-name"))
        );
        assert_eq!(
            data.build_config_dir(),
            PathBuf::from(String::from("/workspace/builds/test-name/conf"))
        );
        assert_eq!(
            data.deploy_dir(),
            PathBuf::from(String::from("/workspace/builds/test-name/tmp/test/deploy"))
        );
        assert_eq!(
            data.bblayers_conf_path(),
            PathBuf::from(String::from(
                "/workspace/builds/test-name/conf/bblayers.conf"
            ))
        );
        assert_eq!(
            data.local_conf_path(),
            PathBuf::from(String::from("/workspace/builds/test-name/conf/local.conf"))
        );
        assert!(!data.bblayers_conf().is_empty());
        let mut conf_str: String = String::new();
        conf_str.push_str("BB_LAYERS_CONF_TEST_LINE_1\n");
        conf_str.push_str("BB_LAYERS_CONF_TEST_LINE_2\n");
        conf_str.push_str("BB_LAYERS_CONF_TEST_LINE_3\n");
        assert_eq!(data.bblayers_conf(), conf_str);
        assert!(!data.local_conf().is_empty());
        let mut conf_str: String = String::new();
        conf_str.push_str("BB_LOCAL_CONF_TEST_LINE_1\n");
        conf_str.push_str("BB_LOCAL_CONF_TEST_LINE_2\n");
        conf_str.push_str("BB_LOCAL_CONF_TEST_LINE_3\n");
        conf_str.push_str("MACHINE ?= \"test-machine\"\n");
        conf_str.push_str("PRODUCT_NAME ?= \"test-name\"\n");
        conf_str.push_str("DISTRO ?= \"test-distro\"\n");
        conf_str.push_str("SSTATE_DIR ?= \"/workspace/.cache/test-arch/sstate-cache\"\n");
        conf_str.push_str("DL_DIR ?= \"/workspace/.cache/download\"\n");
        assert_eq!(data.local_conf(), conf_str);
        assert_eq!(
            data.sstate_dir(),
            PathBuf::from(String::from("/workspace/.cache/test-arch/sstate-cache"))
        );
        assert_eq!(
            data.dl_dir(),
            PathBuf::from(String::from("/workspace/.cache/download"))
        );
        assert_eq!(
            data.init_env_file(),
            PathBuf::from(String::from("/workspace/layers/test/oe-my-init-env"))
        );
    }

    #[test]
    fn test_ws_bitbake_ctx_conf() {
        let json_settings: &str = r#"
        {
            "version": "6"
        }"#;
        let json_build_config = r#"
        {
            "version": "6",
            "name": "test-name",
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
                    "BAKERY_WORKSPACE_DIR ?= \"$#[BKRY_BUILDS_DIR]/workspace\"",
                    "PACKAGE_CLASSES ?= \"package_rpm\"",
                    "BB_DISKMON_DIRS ?= \"\\",
                    "    STOPTASKS,${TMPDIR},1G,100K \\",
                    "    HALT,${SSTATE_DIR},100M,1K \\",
                    "    HALT,/tmp,10M,1K\"",
                    "CONF_VERSION = \"2\""
                ]
            }
        }"#;
        let work_dir: PathBuf = PathBuf::from("/bakery-ws");
        let settings: WsSettingsHandler =
            WsSettingsHandler::from_str(&work_dir, json_settings, None)
                .expect("Failed to parse settings");
        let mut data: WsBitbakeData = WsBitbakeData::from_str(json_build_config, &settings)
            .expect("Failed to parse product data");
        let variables: IndexMap<String, String> = indexmap! {
            "BKRY_BUILDS_DIR".to_string() => settings.builds_dir().to_string_lossy().to_string(),
            "BKRY_LAYERS_DIR".to_string() => settings.layers_dir().to_string_lossy().to_string()
        };
        let context: Context = Context::new(&variables);
        data.expand_ctx(&context).unwrap();
        assert_eq!(data.bblayers_conf(), "BAKERY_WORKDIR=\"${TOPDIR}/../..\"\nBBLAYERS ?= \" \\\n       /bakery-ws/layers/meta-test \\\n       /bakery-ws/builds/workspace \\\n\"\n");
        assert_eq!(data.local_conf(), "BAKERY_WORKSPACE_DIR ?= \"/bakery-ws/builds/workspace\"\nPACKAGE_CLASSES ?= \"package_rpm\"\nBB_DISKMON_DIRS ?= \"\\\n    STOPTASKS,${TMPDIR},1G,100K \\\n    HALT,${SSTATE_DIR},100M,1K \\\n    HALT,/tmp,10M,1K\"\nCONF_VERSION = \"2\"\nMACHINE ?= \"test-machine\"\nPRODUCT_NAME ?= \"test-name\"\nDISTRO ?= \"test-distro\"\nSSTATE_DIR ?= \"/bakery-ws/.cache/test-arch/sstate-cache\"\nDL_DIR ?= \"/bakery-ws/.cache/download\"\n");
    }

    #[test]
    fn test_ws_bitbake_data_to_string() {
        let json_settings: &str = r#"
        {
            "version": "6"
        }"#;
        let json_build_config = r#"
        {
            "version": "6",
            "name": "test-name",
            "arch": "test-arch",
            "bb": {
                "machine": "test-machine",
                "distro": "test-distro",
                "deploydir": "tmp/test/deploy",
                "docker": "test-registry/test-image:0.1",
                "initenv": "layers/test/oe-my-init-env",
                "bblayersconf": [
                ],
                "localconf": [
                ]
            }
        }"#;
        let json_bb = r#""bb": {"bblayersconf":[],"deploydir":"tmp/test/deploy","distro":"test-distro","docker":"test-registry/test-image:0.1","initenv":"layers/test/oe-my-init-env","localconf":[],"machine":"test-machine"}"#;
        let work_dir: PathBuf = PathBuf::from("/workspace");
        let settings: WsSettingsHandler =
            WsSettingsHandler::from_str(&work_dir, json_settings, None)
                .expect("Failed to parse settings");
        let data: WsBitbakeData = WsBitbakeData::from_str(json_build_config, &settings)
            .expect("Failed to parse product data");
        assert_eq!(data.to_string(), json_bb);
    }
}
