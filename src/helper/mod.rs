use crate::constants::BkryConstants;
use crate::data::WsBuildData;
use crate::workspace::{Workspace, WsArtifactsHandler, WsBuildConfigHandler, WsSettingsHandler};

use crate::configs::WsSettings;
use crate::error::BError;
use crate::executers::DockerImage;
use crate::fs::Archiver;

use indexmap::IndexMap;
use rand::prelude::*;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use users::Groups;

pub struct Helper;

impl Helper {
    pub fn write_json_conf(path: &PathBuf, json_str: &str) {
        if let Some(parent_dir) = path.parent() {
            std::fs::create_dir_all(parent_dir).expect("Failed create parent dir");
        }

        let mut file: File = File::create(&path).expect("Failed to create file");

        // Write the JSON string to the file.
        file.write_all(json_str.as_bytes())
            .expect("Failed to write json to file");
    }

    pub fn setup_test_ws_default_dirs(work_dir: &Path) {
        std::fs::create_dir_all(work_dir.join(BkryConstants::BKRY_DEFAULT_CFG_DIR))
            .expect("Failed to create config dir!");
        std::fs::create_dir_all(work_dir.join(BkryConstants::BKRY_DEFAULT_INCLUDE_CFG_DIR))
            .expect("Failed to create include dir!");
        std::fs::create_dir_all(work_dir.join(BkryConstants::BKRY_DEFAULT_BUILDS_DIR))
            .expect("Failed to create builds dir!");
        std::fs::create_dir_all(work_dir.join(BkryConstants::BKRY_DEFAULT_ARTIFACTS_DIR))
            .expect("Failed to create artifacts dir!");
        std::fs::create_dir_all(work_dir.join(BkryConstants::BKRY_DEFAULT_SCRIPTS_DIR))
            .expect("Failed to create scripts dir!");
        std::fs::create_dir_all(work_dir.join(BkryConstants::BKRY_DEFAULT_DOCKER_DIR))
            .expect("Failed to create docker dir!");
        std::fs::create_dir_all(work_dir.join(BkryConstants::BKRY_DEFAULT_CACHE_DIR))
            .expect("Failed to create cache dir!");
        let default_settings: &str = r#"
        {
            "version": "5"
        }"#;
        Helper::write_json_conf(&work_dir.join(BkryConstants::WS_SETTINGS), default_settings);
    }

    pub fn setup_test_ws_dirs(ws_settings: &WsSettingsHandler) {
        std::fs::create_dir_all(ws_settings.configs_dir()).expect("Failed to create config dir!");
        std::fs::create_dir_all(ws_settings.include_dir()).expect("Failed to create include dir!");
        std::fs::create_dir_all(ws_settings.builds_dir()).expect("Failed to create builds dir!");
        std::fs::create_dir_all(ws_settings.artifacts_dir())
            .expect("Failed to create artifacts dir!");
        std::fs::create_dir_all(ws_settings.scripts_dir()).expect("Failed to create scripts dir!");
        std::fs::create_dir_all(ws_settings.docker_dir()).expect("Failed to create docker dir!");
        std::fs::create_dir_all(ws_settings.cache_dir()).expect("Failed to create cache dir!");
    }

    pub fn setup_test_build_configs_files(configs: &IndexMap<PathBuf, String>) {
        configs.iter().for_each(|(path, data)| {
            println!("Creating test build config file: {}", path.display());
            let mut file = File::create(path).expect("Failed to create build config");
            file.write_all(data.as_bytes())
                .expect("Failed to write data to build config");
        });
    }

    pub fn create_test_files(files: &Vec<PathBuf>) {
        let mut rng: ThreadRng = rand::thread_rng();

        files.iter().for_each(|f| {
            //println!("Creating test file: {}", f.display());
            if let Some(parent_dir) = f.parent() {
                std::fs::create_dir_all(parent_dir).expect("Failed to create parent dir");
            }
            let mut file: File = File::create(f).expect("Failed to create test file");
            let mut buffer = [0u8; 2048]; // Adjust the buffer size as needed
            rng.fill(&mut buffer);
            file.write_all(&buffer)
                .expect("Failed to write random data to file");
        });
    }

    pub fn list_files_in_dir(
        dir: &Path,
        files: &mut Vec<PathBuf>,
        strip: &Path,
    ) -> std::io::Result<()> {
        if dir.is_dir() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    // Recursively visit sub-directory and collect its file paths
                    Self::list_files_in_dir(&path, files, strip)?;
                } else {
                    // Add the file path to the list
                    //println!("path: {}", path.display());
                    let p: PathBuf = path
                        .strip_prefix(strip.as_os_str())
                        .expect("Failed to strip prefix")
                        .to_path_buf();
                    files.push(p.clone());
                }
            }
        }

        Ok(())
    }

    pub fn list_files_in_archive(
        archive: &Archiver,
        work_dir: &Path,
    ) -> Result<Vec<PathBuf>, BError> {
        // TODO: the prefered solution would be to use the entries() of the tar::Archive struct
        // but for some reason it will always only return one entry so we are not able to list
        // the files without unpack the content.

        let mut archived_files: Vec<PathBuf> = Vec::new();
        let unpack_dir: PathBuf = work_dir.join(PathBuf::from("unpack/"));

        if !archive.path().exists() {
            return Err(BError::ArchiverError(format!(
                "No such archive '{}'",
                archive.path().display()
            )));
        }

        if archive.extension() == "tar" {
            let file: File = File::open(archive.path())?;
            let mut tar: tar::Archive<Box<dyn std::io::Read>>;
            if archive.compression() == "gz" {
                tar = tar::Archive::new(Box::new(flate2::read::GzDecoder::new(file)));
            } else if archive.compression() == "bz2" {
                tar = tar::Archive::new(Box::new(bzip2::read::BzDecoder::new(file)));
            } else {
                return Err(BError::ArchiverError(format!(
                    "Unsupported compression '{}'!",
                    archive.compression()
                )));
            }

            tar.unpack(unpack_dir.to_str().unwrap()).unwrap();

            Helper::list_files_in_dir(&unpack_dir, &mut archived_files, &unpack_dir)
                .expect("Failed to list files in dir");
        } else if archive.extension() == "zip" {
            let file: File = File::open(archive.path())?;
            let mut zip: zip::ZipArchive<_> =
                zip::ZipArchive::new(file).expect("Failed to setup zip archive");
            for i in 0..zip.len() {
                let file: zip::read::ZipFile<'_> = zip
                    .by_index(i)
                    .expect("Failed to read content from archive");
                archived_files.push(PathBuf::from(file.name()));
            }
        }

        Ok(archived_files)
    }

    pub fn verify_archived_files(
        expected_files: &Vec<PathBuf>,
        archived_files: &Vec<PathBuf>,
        work_dir: &Path,
    ) {
        // strip the workdir from the files
        let files: Vec<PathBuf> = expected_files
            .iter()
            .map(|f| {
                let p = f
                    .strip_prefix(work_dir.as_os_str())
                    .expect("Failed to strip prefix")
                    .to_path_buf();
                p
            })
            .collect();

        // Convert to HashSet so we are not failing if the order of the vectors are not matching
        let expected: HashSet<_> = files.iter().map(|p| p.as_path()).collect();
        let archived: HashSet<_> = archived_files.iter().map(|p| p.as_path()).collect();
        assert_eq!(expected, archived);
    }

    /*
    pub fn setup_task_config(json_test_str: &str) -> TaskConfig {
        let result: Result<TaskConfig, BError> = TaskConfig::from_str(json_test_str);
        match result {
            Ok(rconfig) => {
                rconfig
            }
            Err(e) => {
                eprintln!("Error parsing tasks from build config: {}", e);
                panic!();
            }
        }
    }
    */

    pub fn setup_ws_settings(json_test_str: &str) -> WsSettings {
        let result: Result<WsSettings, BError> = WsSettings::from_str(json_test_str);
        let settings: WsSettings;
        match result {
            Ok(rsettings) => {
                settings = rsettings;
            }
            Err(e) => {
                eprintln!("Error parsing JSON: {}", e);
                panic!();
            }
        }
        settings
    }

    /*
    pub fn setup_build_config(json_test_str: &str) -> BuildConfig {
        let result: Result<BuildConfig, BError> = BuildConfig::from_str(json_test_str);
        match result {
            Ok(rconfig) => {
                rconfig
            }
            Err(e) => {
                eprintln!("Error parsing build config: {}", e);
                panic!();
            }
        }
    }
    */

    pub fn setup_ws_build_config_handler(
        test_work_dir: &str,
        json_settings: &str,
        json_build_config: &str,
    ) -> WsBuildConfigHandler {
        let work_dir: PathBuf = PathBuf::from(test_work_dir);
        let mut settings: WsSettingsHandler =
            WsSettingsHandler::new(work_dir, Helper::setup_ws_settings(json_settings), None);
        let result: Result<WsBuildConfigHandler, BError> =
            WsBuildConfigHandler::from_str(json_build_config, &mut settings);
        match result {
            Ok(ws_config) => ws_config,
            Err(e) => {
                eprintln!("Error parsing build config: {}", e);
                panic!();
            }
        }
    }

    pub fn setup_ws(
        test_work_dir: &str,
        json_settings: &str,
        json_build_config: &str,
    ) -> Workspace {
        let work_dir: PathBuf = PathBuf::from(test_work_dir);
        let mut settings: WsSettingsHandler = WsSettingsHandler::new(
            work_dir.clone(),
            Self::setup_ws_settings(json_settings),
            None,
        );
        let config: WsBuildConfigHandler =
            WsBuildConfigHandler::from_str(json_build_config, &mut settings)
                .expect("Failed to parse build config");
        Workspace::new(Some(work_dir), Some(settings), Some(config))
            .expect("Failed to setup workspace")
    }

    pub fn setup_build_data(
        work_dir: &PathBuf,
        json_build_config: Option<&str>,
        json_settings: Option<&str>,
    ) -> WsBuildData {
        let json_default_settings: &str = r#"
        {
            "version": "6"
        }"#;
        let json_default_build_config = r#"
        {
            "version": "6"
        }"#;
        let ws_settings: WsSettingsHandler = WsSettingsHandler::from_str(
            &work_dir,
            json_settings.unwrap_or(json_default_settings),
            None,
        )
        .unwrap_or_else(|err| panic!("Error parsing JSON settings: {}", err));

        let data: WsBuildData = WsBuildData::from_str(
            json_build_config.unwrap_or(json_default_build_config),
            &ws_settings,
        )
        .unwrap_or_else(|err| panic!("Error parsing JSON build config: {}", err));

        data
    }

    pub fn parse(json_string: &str) -> Result<Value, BError> {
        match serde_json::from_str(json_string) {
            Ok(data) => Ok(data),
            Err(err) => Err(BError::ParseError(format!("Failed to parse JSON: {}", err))),
        }
    }

    pub fn setup_collector_test_ws(
        _work_dir: &PathBuf,
        task_build_dir: &PathBuf,
        files: &Vec<PathBuf>,
        build_data: &WsBuildData,
        json_artifacts_config: &str,
    ) -> WsArtifactsHandler {
        Helper::create_test_files(files);
        let artifacts: WsArtifactsHandler =
            WsArtifactsHandler::from_str(json_artifacts_config, &task_build_dir, build_data)
                .expect("Failed to parse config");
        artifacts
    }

    pub fn assert_hashmap(hash: &HashMap<String, String>, verify: &HashMap<String, String>) {
        assert!(!hash.is_empty());
        assert!(!verify.is_empty());
        hash.iter().for_each(|(key, value)| {
            println!("Verify key {}={}", key, value);
            assert_eq!(value, &verify[key]);
        });
    }

    pub fn env_home() -> String {
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

    pub fn docker_pull_string(image: &DockerImage) -> Vec<String> {
        let mut cmd_line: Vec<String> = vec![String::from("docker"), String::from("pull")];
        cmd_line.push(format!("{}", image));
        cmd_line
    }

    pub fn docker_bootstrap_string(
        interactive: bool,
        args: &Vec<String>,
        volumes: &Vec<String>,
        top_dir: &PathBuf,
        work_dir: &PathBuf,
        image: &DockerImage,
        cmd: &Vec<String>,
    ) -> Vec<String> {
        let mut cmd_line: Vec<String> = vec![
            String::from("docker"),
            String::from("run"),
            String::from("--name"),
            format!("bakery-workspace-{}", std::process::id()),
            String::from("-t"),
            String::from("--rm"),
        ];
        if interactive {
            cmd_line.push("-i".to_string());
        }
        let cache: users::UsersCache = users::UsersCache::new();
        cmd_line.append(&mut vec![
            String::from("--group-add"),
            cache.get_group_by_name("docker").unwrap().gid().to_string(),
        ]);
        if !volumes.is_empty() {
            volumes.iter().for_each(|v| {
                cmd_line.append(&mut vec![String::from("-v"), v.to_string()]);
            })
        }
        cmd_line.append(&mut vec![
            String::from("-v"),
            String::from(format!(
                "{}:{}:ro",
                BkryConstants::BKRY_BIN,
                BkryConstants::BKRY_BIN
            )),
            String::from("-v"),
            String::from(format!(
                "{}:{}:ro",
                BkryConstants::BKRY_CFG_DIR,
                BkryConstants::BKRY_CFG_DIR
            )),
            String::from("-v"),
            String::from(format!(
                "{}:{}:ro",
                BkryConstants::BKRY_OPT_DIR,
                BkryConstants::BKRY_OPT_DIR
            )),
            String::from("-v"),
            String::from("/etc/passwd:/etc/passwd:ro"),
            String::from("-v"),
            String::from("/etc/group:/etc/group:ro"),
            String::from("-v"),
            String::from("/var/run/docker.sock:/var/run/docker.sock"),
            String::from("-u"),
            format!("{}:{}", users::get_current_uid(), users::get_current_gid()),
            String::from("-v"),
            format!("{}:{}", top_dir.display(), top_dir.display()),
        ]);
        cmd_line.append(&mut vec![
            String::from("-w"),
            format!("{}", work_dir.display()),
        ]);
        if !args.is_empty() {
            cmd_line.append(&mut args.clone());
        }
        cmd_line.push(format!("{}", image));
        cmd_line.append(&mut cmd.clone());
        //println!("cmd_line {:?}", cmd_line);
        cmd_line
    }

    pub fn docker_cmdline_string(
        interactive: bool,
        work_dir: &PathBuf,
        image: &DockerImage,
        cmd: &Vec<String>,
        env_file: &PathBuf,
    ) -> Vec<String> {
        let mut cmd_line: Vec<String> = vec![
            String::from("docker"),
            String::from("run"),
            String::from("-u"),
            format!("{}:{}", users::get_current_uid(), users::get_current_gid()),
            String::from("-v"),
            String::from("/etc/passwd:/etc/passwd:ro"),
            String::from("-v"),
            String::from("/etc/group:/etc/group:ro"),
            String::from("-v"),
            format!("{}:{}", Helper::env_home(), Helper::env_home()),
            String::from("-w"),
            format!("{}", work_dir.display()),
            String::from("-t"),
            String::from("--rm"),
        ];
        if interactive {
            cmd_line.push("-i".to_string());
        }
        let cache: users::UsersCache = users::UsersCache::new();
        cmd_line.append(&mut vec![
            String::from("--group-add"),
            cache.get_group_by_name("docker").unwrap().gid().to_string(),
        ]);
        cmd_line.append(&mut vec![
            String::from("--env-file"),
            env_file.to_string_lossy().to_string(),
        ]);
        cmd_line.push(format!("{}", image));
        cmd_line.append(&mut cmd.clone());
        //println!("cmd_line {:?}", cmd_line);
        cmd_line
    }
}
