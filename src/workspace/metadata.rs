/*
 * The idea with the WsBuildMetadataHandler is to manage the workspace build data mainly
 * determine if a developer is switching the variant or the build revision because if they
 * do that going from user to userdebug for example then they have to clean the workspace.
 * Or at least that is how the HLOS builds works. A developer can setup multiple workspaces
 * and in theory they can have the same directory name so to make sure that an unique meta
 * data file is created the path to the workspace will be hased and used as part of the name
 * of the workspace build meta data file.
 */

use crate::commands::Variant;
use crate::configs::Config;
use crate::error::BError;
use crate::fs::Manifest;
use crate::workspace::WsId;

use serde_json::{self, json, Value};
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Clone)]
pub struct WsBuildMetadataHandler {
    workspace_dir: PathBuf,
    bkry_home_dir: PathBuf,
    metadata_path: PathBuf,
}

impl Config for WsBuildMetadataHandler {}

impl WsBuildMetadataHandler {
    pub fn new(workspace_dir: &PathBuf, bkry_home_dir: &PathBuf, wsid: Option<String>) -> Self {
        let path: PathBuf = bkry_home_dir.join(PathBuf::from(format!(
            "workspaces/{}/metadata.json",
            wsid.unwrap_or(WsId::get())
        )));

        WsBuildMetadataHandler {
            workspace_dir: workspace_dir.clone(),
            bkry_home_dir: bkry_home_dir.clone(),
            metadata_path: path,
        }
    }

    pub fn write(
        &self,
        config: &str,
        bbmachine: &str,
        bbdistro: &str,
        variant: &Variant,
    ) -> Result<(), BError> {
        let ws_build_metadata: Manifest = Manifest::new(self.path())?;
        let json_object: Value = json!({
            "config": config.to_string(),
            "machine": bbmachine.to_string(),
            "distro": bbdistro.to_string(),
            "variant": variant.to_string(),
            "directory": self.workspace_dir.clone(),
        });
        let json_string: String = serde_json::to_string_pretty(&json_object).map_err(|e| {
            BError::ParseError(format!("Error parsing JSON workspace stamp file: {}", e))
        })?;
        ws_build_metadata.write(json_string.as_str())?;

        Ok(())
    }

    pub fn path(&self) -> &PathBuf {
        &self.metadata_path
    }

    pub fn directory(&self) -> Result<String, BError> {
        /*
         * If the workspace build meta data file is missing then we cant determine the meta data
         * has changed so just return true
         */
        if self.metadata_path.exists() {
            let metadata_content: String =
                fs::read_to_string(&self.metadata_path).map_err(|e| {
                    BError::IOError(format!(
                        "Error reading workspace build meta-data file: {}",
                        e
                    ))
                })?;

            let metadata_object: Value = serde_json::from_str(&metadata_content).map_err(|e| {
                BError::ParseError(format!(
                    "Error parsing JSON workspace build meta-data file: {}",
                    e
                ))
            })?;

            let dir: String = Self::get_str_value("directory", &metadata_object, None)?;
            return Ok(dir);
        }

        return Err(BError::ParseError(format!(
            "Error parsing workspace build meta-data file '{}'!",
            self.metadata_path.display()
        )));
    }

    pub fn reset(&self) -> Result<(), BError> {
        if self.metadata_path.exists() {
            fs::remove_file(&self.metadata_path).map_err(|e| {
                BError::IOError(format!(
                    "Error when removing build meta data file '{}': {}",
                    self.metadata_path.display(),
                    e
                ))
            })?;
        }
        Ok(())
    }

    pub fn _read_data(&self, name: &str, default: Option<String>) -> Result<String, BError> {
        /*
         * If the workspace build meta data file is missing then we cant determine if the meta data
         * has changed so just return true
         */
        if self.metadata_path.exists() {
            let metadata_content: String =
                fs::read_to_string(&self.metadata_path).map_err(|e| {
                    BError::IOError(format!(
                        "Error reading workspace build meta-data file: {}",
                        e
                    ))
                })?;

            let metadata_object: Value = serde_json::from_str(&metadata_content).map_err(|e| {
                BError::ParseError(format!(
                    "Error parsing JSON workspace build meta-data file: {}",
                    e
                ))
            })?;

            let data: String = Self::get_str_value(name, &metadata_object, default)?;
            return Ok(data);
        }

        return Err(BError::ParseError(format!(
            "Error parsing workspace build meta-data file '{}'!",
            self.metadata_path.display()
        )));
    }

    pub fn config(&self) -> Result<String, BError> {
        self._read_data("config", None)
    }

    pub fn variant(&self) -> Result<Variant, BError> {
        let variant_str: String = self._read_data("variant", None)?;
        Variant::from_str(&variant_str)
    }

    pub fn bbmachine(&self) -> Result<String, BError> {
        self._read_data("machine", Some("NA".to_string()))
    }

    pub fn bbdistro(&self) -> Result<String, BError> {
        self._read_data("distro", Some("NA".to_string()))
    }

    pub fn exists(&self) -> bool {
        self.metadata_path.exists()
    }

    pub fn verify(
        &self,
        config: &str,
        bbmachine: &str,
        bbdistro: &str,
        variant: &Variant,
    ) -> Result<bool, BError> {
        let json_object: Value = json!({
            "config": config.to_string(),
            "machine": bbmachine.to_string(),
            "distro": bbdistro.to_string(),
            "variant": variant.to_string(),
            "directory": self.workspace_dir.clone(),
        });

        /*
         * If the workspace build meta data file is missing then we cant determine the meta data
         * has changed so just return true
         */
        if self.metadata_path.exists() {
            let metadata_content: String =
                fs::read_to_string(&self.metadata_path).map_err(|e| {
                    /*
                     * We don't want to block so in case of error lets remove the
                     * meta data file
                     */
                    let _result: Result<(), BError> = self.reset();
                    BError::IOError(format!(
                        "Error reading workspace build meta-data file: {}",
                        e
                    ))
                })?;

            let metadata_object: Value = serde_json::from_str(&metadata_content).map_err(|e| {
                /*
                 * We don't want to block so in case of error lets remove the
                 * meta data file
                 */
                let _result: Result<(), BError> = self.reset();
                BError::ParseError(format!(
                    "Error parsing JSON workspace build meta-data file: {}",
                    e
                ))
            })?;

            if json_object != metadata_object {
                if self.config()? != *config {
                    return Err(BError::BuildMetadataChangeError(format!(
                        "The build config has been changed to '{}'",
                        config
                    )));
                }

                if self.bbmachine()? != *bbmachine {
                    return Err(BError::BuildMetadataChangeError(format!(
                        "The machine has been changed to '{}'",
                        bbmachine
                    )));
                }

                if self.bbdistro()? != *bbdistro {
                    return Err(BError::BuildMetadataChangeError(format!(
                        "The distro has been changed to '{}'",
                        bbdistro
                    )));
                }

                if self.variant()? != *variant {
                    return Err(BError::BuildMetadataChangeError(format!(
                        "The build variant has been changed to '{}'",
                        variant
                    )));
                }
            }
        }

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use crate::commands::Variant;
    use crate::error::BError;
    use crate::workspace::WsBuildMetadataHandler;

    use serde_json::{self, json, Value};
    use std::fs::File;
    use std::io::{Read, Write};
    use std::path::{Path, PathBuf};
    use tempdir::TempDir;

    #[test]
    fn test_workspace_metadata_write() {
        let temp_dir: TempDir =
            TempDir::new("bkry-test-dir").expect("Failed to create temp directory");
        let path: &Path = temp_dir.path();
        let workspace_dir: PathBuf = path.join("ws");
        let bkry_home_dir: PathBuf = path.join(".bkry");
        let variant: Variant = Variant::RELEASE;
        let config: &str = "test";
        let bbmachine: &str = "test-machine";
        let bbdistro: &str = "test-distro";
        let handler: WsBuildMetadataHandler =
            WsBuildMetadataHandler::new(&workspace_dir, &bkry_home_dir, None);
        let json_reference_object: Value = json!({
            "config": config.to_string(),
            "variant": variant.to_string(),
            "machine": bbmachine.to_string(),
            "distro": bbdistro.to_string(),
            "directory": workspace_dir,
        });
        let json_reference_str: String =
            serde_json::to_string_pretty(&json_reference_object).unwrap();
        handler
            .write(config, bbmachine, bbdistro, &variant)
            .expect("Failed to create workspace stamp file!");
        assert!(handler.path().exists());
        let mut file: File =
            File::open(handler.path()).expect("Failed to open workspace stamp file!");
        let mut contents: String = String::new();
        file.read_to_string(&mut contents)
            .expect("Failed to read manifes file!");
        assert_eq!(json_reference_str, contents);
    }

    #[test]
    fn test_workspace_metadata_verify() {
        let temp_dir: TempDir =
            TempDir::new("bkry-test-dir").expect("Failed to create temp directory");
        let path: &Path = temp_dir.path();
        let workspace_dir: PathBuf = path.join("ws");
        let bkry_home_dir: PathBuf = path.join(".bkry");
        let config: &str = "test";
        let bbmachine: &str = "test-machine";
        let bbdistro: &str = "test-distro";
        let handler: WsBuildMetadataHandler =
            WsBuildMetadataHandler::new(&workspace_dir, &bkry_home_dir, None);
        handler
            .write(config, bbmachine, bbdistro, &Variant::DEV)
            .expect("Failed to create workspace stamp file!");
        assert!(handler
            .verify(config, bbmachine, bbdistro, &Variant::DEV)
            .expect("Failed to verify the workspace metadata"));
    }

    #[test]
    fn test_workspace_metadata_verify_config() {
        let temp_dir: TempDir =
            TempDir::new("bkry-test-dir").expect("Failed to create temp directory");
        let path: &Path = temp_dir.path();
        let workspace_dir: PathBuf = path.join("ws");
        let bkry_home_dir: PathBuf = path.join(".bkry");
        let config: &str = "test";
        let bbmachine: &str = "test-machine";
        let bbdistro: &str = "test-distro";
        let handler: WsBuildMetadataHandler =
            WsBuildMetadataHandler::new(&workspace_dir, &bkry_home_dir, None);
        handler
            .write(config, bbmachine, bbdistro, &Variant::RELEASE)
            .expect("Failed to create workspace stamp file!");
        let result: Result<bool, BError> =
            handler.verify("test2", bbmachine, bbdistro, &Variant::DEV);
        match result {
            Ok(_) => {
                // If it returns Ok, the test should fail
                panic!("Expected an error, but got Ok");
            }
            Err(e) => {
                // Check the error message
                assert_eq!(e.to_string(), "The build config has been changed to 'test2'. If your product requires it, run bkry sync --reset (or bkry clean) before building. If you understand the implications, you can run the build with --force.".to_string());
            }
        }
    }

    #[test]
    fn test_workspace_metadata_verify_bbmachine() {
        let temp_dir: TempDir =
            TempDir::new("bkry-test-dir").expect("Failed to create temp directory");
        let path: &Path = temp_dir.path();
        let workspace_dir: PathBuf = path.join("ws");
        let bkry_home_dir: PathBuf = path.join(".bkry");
        let config: &str = "test";
        let bbmachine: &str = "test-machine";
        let bbdistro: &str = "test-distro";
        let handler: WsBuildMetadataHandler =
            WsBuildMetadataHandler::new(&workspace_dir, &bkry_home_dir, None);
        handler
            .write(config, bbmachine, bbdistro, &Variant::RELEASE)
            .expect("Failed to create workspace stamp file!");
        let result: Result<bool, BError> =
            handler.verify(config, "test-target-2", bbdistro, &Variant::DEV);
        match result {
            Ok(_) => {
                // If it returns Ok, the test should fail
                panic!("Expected an error, but got Ok");
            }
            Err(e) => {
                // Check the error message
                assert_eq!(e.to_string(), "The machine has been changed to 'test-target-2'. If your product requires it, run bkry sync --reset (or bkry clean) before building. If you understand the implications, you can run the build with --force.".to_string());
            }
        }
    }

    #[test]
    fn test_workspace_metadata_verify_bbdistro() {
        let temp_dir: TempDir =
            TempDir::new("bkry-test-dir").expect("Failed to create temp directory");
        let path: &Path = temp_dir.path();
        let workspace_dir: PathBuf = path.join("ws");
        let bkry_home_dir: PathBuf = path.join(".bkry");
        let config: &str = "test";
        let bbmachine: &str = "test-machine";
        let bbdistro: &str = "test-distro";
        let handler: WsBuildMetadataHandler =
            WsBuildMetadataHandler::new(&workspace_dir, &bkry_home_dir, None);
        handler
            .write(config, bbmachine, bbdistro, &Variant::RELEASE)
            .expect("Failed to create workspace stamp file!");
        let result: Result<bool, BError> =
            handler.verify(config, bbmachine, "test-distro-2", &Variant::DEV);
        match result {
            Ok(_) => {
                // If it returns Ok, the test should fail
                panic!("Expected an error, but got Ok");
            }
            Err(e) => {
                // Check the error message
                assert_eq!(e.to_string(), "The distro has been changed to 'test-distro-2'. If your product requires it, run bkry sync --reset (or bkry clean) before building. If you understand the implications, you can run the build with --force.".to_string());
            }
        }
    }

    #[test]
    fn test_workspace_metadata_verify_variant() {
        let temp_dir: TempDir =
            TempDir::new("bkry-test-dir").expect("Failed to create temp directory");
        let path: &Path = temp_dir.path();
        let workspace_dir: PathBuf = path.join("ws");
        let bkry_home_dir: PathBuf = path.join(".bkry");
        let config: &str = "test";
        let bbmachine: &str = "test-machine";
        let bbdistro: &str = "test-distro";
        let handler: WsBuildMetadataHandler =
            WsBuildMetadataHandler::new(&workspace_dir, &bkry_home_dir, None);
        handler
            .write(config, bbmachine, bbdistro, &Variant::RELEASE)
            .expect("Failed to create workspace stamp file!");
        let result: Result<bool, BError> =
            handler.verify(config, bbmachine, bbdistro, &Variant::DEV);
        match result {
            Ok(_) => {
                // If it returns Ok, the test should fail
                panic!("Expected an error, but got Ok");
            }
            Err(e) => {
                // Check the error message
                assert_eq!(e.to_string(), "The build variant has been changed to 'dev'. If your product requires it, run bkry sync --reset (or bkry clean) before building. If you understand the implications, you can run the build with --force.".to_string());
            }
        }
    }

    #[test]
    fn test_workspace_metadata_verify_false() {
        let temp_dir: TempDir =
            TempDir::new("bkry-test-dir").expect("Failed to create temp directory");
        let path: &Path = temp_dir.path();
        let workspace_dir: PathBuf = path.join("ws");
        let bkry_home_dir: PathBuf = path.join(".bkry");
        let config: &str = "test";
        let bbmachine: &str = "test-machine";
        let bbdistro: &str = "test-distro";
        let handler: WsBuildMetadataHandler =
            WsBuildMetadataHandler::new(&workspace_dir, &bkry_home_dir, None);
        handler
            .write(config, bbmachine, bbdistro, &Variant::RELEASE)
            .expect("Failed to create workspace stamp file!");
        let result: Result<bool, BError> =
            handler.verify(config, bbmachine, bbdistro, &Variant::DEV);
        match result {
            Ok(_) => {
                // If it returns Ok, the test should fail
                panic!("Expected an error, but got Ok");
            }
            Err(e) => {
                // Check the error message
                assert_eq!(e.to_string(), "The build variant has been changed to 'dev'. If your product requires it, run bkry sync --reset (or bkry clean) before building. If you understand the implications, you can run the build with --force.".to_string());
            }
        }
    }

    #[test]
    fn test_workspace_metadata_missing() {
        let temp_dir: TempDir =
            TempDir::new("bkry-test-dir").expect("Failed to create temp directory");
        let path: &Path = temp_dir.path();
        let workspace_dir: PathBuf = path.join("ws");
        let bkry_home_dir: PathBuf = path.join(".bkry");
        let config: &str = "test";
        let bbmachine: &str = "test-machine";
        let bbdistro: &str = "test-distro";
        let handler: WsBuildMetadataHandler =
            WsBuildMetadataHandler::new(&workspace_dir, &bkry_home_dir, None);
        assert!(handler
            .verify(config, bbmachine, bbdistro, &Variant::DEV)
            .expect("Failed to verify the workspace metadata"));
    }

    #[test]
    fn test_workspace_metadata_get() {
        let temp_dir: TempDir =
            TempDir::new("bkry-test-dir").expect("Failed to create temp directory");
        let path: &Path = temp_dir.path();
        let workspace_dir: PathBuf = path.join("ws");
        let bkry_home_dir: PathBuf = path.join(".bkry");
        let variant: Variant = Variant::RELEASE;
        let config: &str = "test";
        let bbmachine: &str = "test-machine";
        let bbdistro: &str = "test-distro";
        let handler: WsBuildMetadataHandler =
            WsBuildMetadataHandler::new(&workspace_dir, &bkry_home_dir, None);
        handler
            .write(config, bbmachine, bbdistro, &variant)
            .expect("Failed to create workspace stamp file!");
        assert_eq!(
            handler.variant().expect("Failed to read out variant"),
            variant
        );
        assert_eq!(
            handler.bbmachine().expect("Failed to read out machine"),
            bbmachine
        );
        assert_eq!(
            handler.bbdistro().expect("Failed to read out distro"),
            bbdistro
        );
    }
}
