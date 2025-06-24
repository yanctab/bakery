use crate::error::BError;
use crate::executers::Docker;
use crate::global::TestMode;

use mockall::*;
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::fs::ReadDir;
use std::path::PathBuf;
use std::str;

/*
Tried using "withf" and closure when mocking the check_call for testing
but it did not work when using multiple expectations not sure why but suspect
that it had something todo with that it could not determine which
closure to use because it would always use the first closure which always
resulted in a failed test. Instead we switched to "with" and predicate::eq.
Using the predicate::eq instead required that we use a struct wrapper
for the params that inherits the PartialEq trait. Not ideal but it works
so for now we will use this solution.
*/
#[derive(Debug, PartialEq)]
pub struct CallParams {
    pub cmd_line: Vec<String>,
    pub env: HashMap<String, String>,
    pub shell: bool,
}

impl fmt::Display for CallParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut cmd: String = String::new();
        self.cmd_line.iter().for_each(|c| {
            cmd.push_str(c);
            cmd.push(' ');
        });
        write!(f, "{}", cmd.as_str().trim_end())
    }
}

#[automock]
pub trait System {
    fn check_call(&self, params: &CallParams) -> Result<(), BError>;
    fn init_env_file(
        &self,
        init_file: &PathBuf,
        build_dir: &PathBuf,
    ) -> Result<HashMap<String, String>, BError>;
    fn is_directory_empty(&self, path: &PathBuf) -> Result<bool, BError>;
    fn rmdir_all(&self, path: &PathBuf) -> Result<(), BError>;
    fn env(&self) -> HashMap<String, String>;
    fn inside_docker(&self) -> bool;
}

pub struct BSystem {}

impl BSystem {
    pub fn new() -> Self {
        BSystem {}
    }
}

impl System for BSystem {
    fn check_call(&self, params: &CallParams) -> Result<(), BError> {
        let mut cmd: String = String::new();
        params.cmd_line.iter().for_each(|c| {
            cmd.push_str(c);
            cmd.push(' ');
        });

        if TestMode::is_test_mode() {
            println!("{}", cmd.as_str().trim_end());
            return Ok(());
        }

        //println!("params: '{}'", params);

        // TODO: we should consider how to handle different shells for now we
        // will stick to bash since that is what OE/Yocto requires
        let mut child: std::process::Child = std::process::Command::new("/bin/bash")
            .arg("-c")
            .arg(cmd.as_str().trim_end())
            .stdout(os_pipe::dup_stdout()?)
            .stderr(os_pipe::dup_stderr()?)
            .env_clear()
            .envs(&params.env)
            .spawn()?;

        /*
        Lets keep this for now we might need this later
        let stdout_reader = std::io::BufReader::new(child.stdout.take().unwrap());
        let stderr_reader = std::io::BufReader::new(child.stderr.take().unwrap());
        let stdout_lines = stdout_reader.lines();
        let stderr_lines = stderr_reader.lines();
        let lines = stdout_lines.chain(stderr_lines);

        for line in lines {
            if let Ok(line) = line {
                println!("{}", line);
            }
        }*/

        // Wait for the command to finish
        let status: std::process::ExitStatus = child.wait()?;
        if !status.success() {
            return Err(BError::CliError(format!("{}", status)));
        }

        Ok(())
    }

    fn init_env_file(
        &self,
        init_file: &PathBuf,
        build_dir: &PathBuf,
    ) -> Result<HashMap<String, String>, BError> {
        std::fs::create_dir_all(build_dir)?;

        if !init_file.exists() {
            return Err(BError::CliError(format!(
                "Init env file {} dose not exists",
                init_file.to_string_lossy().to_string()
            )));
        }

        let command: &str = &format!(
            ". {} {} > /dev/null; env",
            init_file.to_string_lossy().to_string(),
            build_dir.to_string_lossy().to_string()
        );

        // Execute the command
        // TODO: we should consider how to handle different shells
        let output: std::process::Output = std::process::Command::new("/bin/bash")
            .arg("-c")
            .arg(command)
            .output()?;

        let status: std::process::ExitStatus = output.status;
        if !status.success() {
            return Err(BError::CliError(format!("{}", status)));
        }

        // Capture the output as a string
        let output_str: &str = str::from_utf8(&output.stdout)?;

        // Split the output into key-value pairs
        let mut env_vars: HashMap<String, String> = HashMap::new();
        for line in output_str.lines() {
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() == 2 {
                env_vars.insert(parts[0].to_string(), parts[1].to_string());
            }
        }

        Ok(env_vars)
    }

    fn rmdir_all(&self, path: &PathBuf) -> Result<(), BError> {
        std::fs::remove_dir_all(path)?;
        Ok(())
    }

    fn inside_docker(&self) -> bool {
        Docker::inside_docker()
    }

    fn is_directory_empty(&self, path: &PathBuf) -> Result<bool, BError> {
        let mut entries: ReadDir = fs::read_dir(path)?;
        let is_empty: bool = entries.next().is_none();
        //println!("Entries in workspace: {}, {}", entries.count(), is_empty);
        Ok(is_empty)
    }

    fn env(&self) -> HashMap<String, String> {
        let env_vars: std::env::Vars = std::env::vars();
        let mut env: HashMap<String, String> = HashMap::new();

        /*
         * When we bootstrap bakery into docker we should make sure that we pull
         * in the entire env from the parent
         */
        for (k, v) in env_vars {
            env.insert(k, v);
        }

        env
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::{fs::File, io::Write};
    use tempdir::TempDir;

    use crate::cli::{BSystem, System};
    use crate::error::BError;

    use super::CallParams;

    #[test]
    fn test_system_init_env_file() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = PathBuf::from(temp_dir.path());
        let build_dir: PathBuf = work_dir.clone().join("build");
        let test_file_path: PathBuf = work_dir.clone().join("init_env");
        let mut test_file: File =
            File::create(&test_file_path).expect("Failed to create init env test file");
        let env: &str = r#"#!/bin/sh
        export ENV1=value1
        export ENV2=value2
        export ENV3=/test/path/test"#;
        test_file
            .write_all(env.as_bytes())
            .expect("Failed to write init env test file");
        let system: BSystem = BSystem::new();
        let envs: HashMap<String, String> = system
            .init_env_file(&test_file_path, &build_dir)
            .expect("Failed to process init env test file");
        assert!(!envs.is_empty());
        let mut verify: HashMap<String, String> = HashMap::new();
        verify.insert("ENV1".to_string(), "value1".to_string());
        verify.insert("ENV2".to_string(), "value2".to_string());
        verify.insert("ENV3".to_string(), "/test/path/test".to_string());
        assert_eq!(
            &envs.get("ENV1").unwrap_or(&"error".to_string()),
            &verify.get("ENV1").unwrap()
        );
        assert_eq!(
            &envs.get("ENV2").unwrap_or(&"error".to_string()),
            &verify.get("ENV2").unwrap()
        );
        assert_eq!(
            &envs.get("ENV3").unwrap_or(&"error".to_string()),
            &verify.get("ENV3").unwrap()
        );
    }

    /*
    TODO: we need to fix this test not sure why it does not fail
    #[test]
    fn test_system_init_env_file_error() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = PathBuf::from(temp_dir.path());
        let build_dir: PathBuf = work_dir.clone().join("build");
        let test_file_path: PathBuf = work_dir.clone().join("init_env");
        let mut test_file: File = File::create(&test_file_path).expect("Failed to create init env test file");
        let env: &str = r#"#!/bin/sh
        garbage sdlkjsdklsjdlskj"#;
        test_file.write_all(env.as_bytes()).expect("Failed to write init env test file");
        let system: BSystem = BSystem::new();
        let result: Result<HashMap<String, String>, BError> = system.init_env_file(&test_file_path, &build_dir);
        match result {
            Ok(_env) => {
                panic!("Was expecting an error!");
            },
            Err(e) => {
                assert_eq!(e.to_string(), format!("Init env file {} dose not exists", test_file_path.to_string_lossy().to_string()));
            }
        }
    }
    */

    #[test]
    fn test_system_init_env_file_missing() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = PathBuf::from(temp_dir.path());
        let build_dir: PathBuf = work_dir.clone().join("build");
        let test_file_path: PathBuf = work_dir.clone().join("init_env");
        let system: BSystem = BSystem::new();
        let result: Result<HashMap<String, String>, BError> =
            system.init_env_file(&test_file_path, &build_dir);
        match result {
            Ok(_env) => {
                panic!("Was expecting an error!");
            }
            Err(e) => {
                assert_eq!(
                    e.to_string(),
                    format!(
                        "Init env file {} dose not exists",
                        test_file_path.to_string_lossy().to_string()
                    )
                );
            }
        }
    }

    #[test]
    fn test_system_check_call_error() {
        let system: BSystem = BSystem::new();
        let params: CallParams = CallParams {
            cmd_line: vec!["exit 1".to_string()],
            env: HashMap::new(),
            shell: true,
        };
        let result: Result<(), BError> = system.check_call(&params);
        match result {
            Ok(()) => {
                panic!("Expected an error!");
            }
            Err(e) => {
                assert_eq!(e.to_string(), "exit status: 1");
            }
        }
    }

    #[test]
    fn test_system_check_call() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = PathBuf::from(temp_dir.path());
        let system: BSystem = BSystem::new();
        let mut env: HashMap<String, String> = HashMap::new();
        env.insert(
            "TEST_FILE1".to_string(),
            work_dir.clone().join("test1").to_string_lossy().to_string(),
        );
        env.insert(
            "TEST_FILE2".to_string(),
            work_dir.clone().join("test2").to_string_lossy().to_string(),
        );
        env.insert(
            "TEST_FILE3".to_string(),
            work_dir.clone().join("test3").to_string_lossy().to_string(),
        );
        let params: CallParams = CallParams {
            cmd_line: vec![format!(
                "cd {}; touch $TEST_FILE1; touch $TEST_FILE2; touch $TEST_FILE3; ls -la .",
                work_dir.to_string_lossy().to_string()
            )],
            env,
            shell: true,
        };
        let _result: Result<(), BError> = system.check_call(&params);
        assert!(work_dir.clone().join("test1").exists());
        assert!(work_dir.clone().join("test2").exists());
        assert!(work_dir.clone().join("test3").exists());
    }
}
