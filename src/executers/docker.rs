use regex::Regex;
use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempdir::TempDir;
use users::Groups;

use crate::cli::Cli;
use crate::constants::BkryConstants;
use crate::error::BError;

pub struct Docker {
    image: DockerImage,
    _interactive: bool,
}

#[derive(Clone)]
pub struct DockerImage {
    pub image: String,
    pub tag: String,
    pub registry: String,
}

impl fmt::Display for DockerImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}:{}", self.registry, self.image, self.tag)
    }
}

impl DockerImage {
    pub fn new(image_str: &str) -> Result<Self, BError> {
        let mut split: Vec<&str> = image_str.split('/').collect();

        if split.len() < 2 {
            return Err(BError::DockerImageError(format!(
                "Invalid image format: {}",
                image_str
            )));
        }

        let tag_split: Vec<&str> = split.pop().unwrap().split(':').collect();

        if tag_split.len() != 2 {
            return Err(BError::DockerImageError(format!(
                "Invalid image format: {}",
                image_str
            )));
        }

        let tag: String = tag_split[1].to_string();
        let registry: String = split[0].to_string();
        let mut image: String = split.split_off(1).join("/").to_string();
        if !image.is_empty() {
            image.push_str("/");
        }
        image.push_str(tag_split[0]);

        Ok(DockerImage {
            registry,
            image,
            tag,
        })
    }
}

/*
 * TODO: we should convert Docker into a TaskExecuter
 */
impl Docker {
    fn replace_env_variables(&self, input: &str, env: &HashMap<String, String>) -> String {
        // Regular expression to match ${VAR}
        let re: Regex = Regex::new(r"\$\{(\w+)\}").unwrap();

        // Replace all occurrences of ${VAR} with the corresponding value from the env map
        let result = re.replace_all(input, |caps: &regex::Captures| {
            let var_name: &str = &caps[1]; // Get the variable name without ${}
            env.get(var_name).unwrap_or(&"".to_string()).to_string() // Return "" if not found
        });

        result.to_string()
    }

    fn env_home(&self) -> String {
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

    fn user(&self) -> Vec<String> {
        vec![format!(
            "-u {}:{}",
            users::get_current_uid(),
            users::get_current_gid()
        )]
    }

    fn etc_files(&self) -> Vec<String> {
        vec![
            String::from("-v /etc/passwd:/etc/passwd:ro"),
            String::from("-v /etc/group:/etc/group:ro"),
        ]
    }

    fn hidden_home_files(&self) -> Vec<String> {
        vec![
            format!(
                "-v {}/.gitconfig:{}/.gitconfig:rw",
                self.env_home(),
                self.env_home()
            ),
            format!("-v {}/.ssh:{}/.ssh:rw", self.env_home(), self.env_home()),
            format!("-v {}/.docker:{}/.docker", self.env_home(), self.env_home()),
            format!("-v {}/.bkry:{}/.bkry", self.env_home(), self.env_home()),
        ]
    }

    fn bkry_volumes(&self) -> Vec<String> {
        vec![
            format!(
                "-v {}:{}:ro",
                BkryConstants::BKRY_BIN,
                BkryConstants::BKRY_BIN
            ),
            format!(
                "-v {}:{}:ro",
                BkryConstants::BKRY_CFG_DIR,
                BkryConstants::BKRY_CFG_DIR
            ),
            format!(
                "-v {}:{}:ro",
                BkryConstants::BKRY_OPT_DIR,
                BkryConstants::BKRY_OPT_DIR
            ),
        ]
    }

    fn home_dir(&self) -> Vec<String> {
        vec![format!("-v {}:{}", self.env_home(), self.env_home())]
    }

    fn work_dir(&self, dir: &PathBuf) -> Vec<String> {
        vec![format!("-w {}", dir.display())]
    }

    fn docker_sock(&self) -> Vec<String> {
        vec![String::from("-v /var/run/docker.sock:/var/run/docker.sock")]
    }

    fn group(&self) -> Vec<String> {
        let cache: users::UsersCache = users::UsersCache::new();
        vec![format!(
            "--group-add {}",
            cache.get_group_by_name("docker").unwrap().gid().to_string()
        )]
    }

    fn env_variables(&self) -> Vec<String> {
        vec![format!("-e HOME={}", self.env_home())]
    }

    fn env_file(&self, env_file: &PathBuf) -> Vec<String> {
        vec![format!(
            "--env-file {}",
            env_file.to_string_lossy().to_string()
        )]
    }

    fn volumes(&self, volumes: &Vec<String>) -> Vec<String> {
        let mut v: Vec<String> = Vec::new();
        volumes.iter().for_each(|e| {
            v.append(&mut vec![format!("-v {}", e.to_string())]);
        });
        v.append(&mut self.etc_files());
        v.append(&mut self.docker_sock());
        v
    }

    fn container_name(&self, name: &str) -> Vec<String> {
        vec![format!("--name {}-{}", name, std::process::id())]
    }

    fn top_dir(&self, dir: &PathBuf) -> Vec<String> {
        vec![format!("-v {}:{}", dir.display(), dir.display())]
    }

    pub fn inside_docker() -> bool {
        let path: PathBuf = PathBuf::from("/.dockerenv");
        // Potentially it would be better to use try_exists
        // for now lets just use exists
        path.exists()
    }

    pub fn image(&self) -> &DockerImage {
        &self.image
    }

    pub fn new(image: DockerImage, interactive: bool) -> Self {
        Docker {
            image,
            _interactive: interactive,
        }
    }

    pub fn bootstrap_cmd_line(
        &self,
        cmd_line: &Vec<String>,
        work_dir: &PathBuf,
        args: &mut Vec<String>,
        volumes: &mut Vec<String>,
    ) -> Vec<String> {
        let mut docker_cmd: Vec<String> = vec!["docker".to_string(), "run".to_string()];
        docker_cmd.append(&mut self.container_name("bakery-workspace"));
        docker_cmd.append(&mut vec!["-t".to_string(), "--rm".to_string()]);
        if self._interactive {
            docker_cmd.push("-i".to_string());
        }
        docker_cmd.append(&mut self.group());
        docker_cmd.append(&mut self.user());
        docker_cmd.append(&mut self.env_variables());
        docker_cmd.append(&mut self.work_dir(work_dir));
        docker_cmd.append(args);
        docker_cmd.append(volumes);
        docker_cmd.push(format!("{}", self.image));
        docker_cmd.append(&mut cmd_line.clone());
        //println!("bootstrap cmd line: {:?}", docker_cmd);
        docker_cmd
    }

    pub fn cmd_line(
        &self,
        cmd_line: &Vec<String>,
        env_file: &PathBuf,
        dir: &PathBuf,
    ) -> Vec<String> {
        let mut docker_cmd: Vec<String> = vec!["docker".to_string(), "run".to_string()];
        docker_cmd.append(&mut self.user());
        docker_cmd.append(&mut self.etc_files());
        docker_cmd.append(&mut self.home_dir());
        docker_cmd.append(&mut self.work_dir(dir));
        docker_cmd.append(&mut vec!["-t".to_string(), "--rm".to_string()]);
        if self._interactive {
            docker_cmd.push("-i".to_string());
        }
        docker_cmd.append(&mut self.group());
        docker_cmd.append(&mut self.env_file(env_file));
        docker_cmd.push(format!("{}", self.image));
        docker_cmd.append(&mut cmd_line.clone());
        docker_cmd
    }

    pub fn setup_env_file(
        &self,
        temp_dir: &Path,
        env: &HashMap<String, String>,
    ) -> Result<PathBuf, BError> {
        let env_file_path: PathBuf = PathBuf::from(temp_dir).join("bakery-docker.env");
        let mut env_file: File = File::create(env_file_path.clone())?;

        for (key, value) in env.iter() {
            writeln!(env_file, "{}={}", key, value)?;
        }

        Ok(env_file_path)
    }

    pub fn pull(&self, cli: &Cli) -> Result<(), BError> {
        let cmd_line: Vec<String> = vec![
            "docker".to_string(),
            "pull".to_string(),
            format!("{}", self.image),
        ];
        cli.check_call(&cmd_line, &HashMap::new(), true)?;
        Ok(())
    }

    pub fn validate_volumes(&self, cli: &Cli, volumes: &mut Vec<String>) {
        /*
         * The user can define a set of docker arguments in the workspace.json in the
         * args json node. If the --volume/-v containes a bad format then docker could create
         * a dir that will be owned by root. We need to validate all --volume/-v entries
         * to avoid it. The user can also set a --volume/-v using an env variable e.g.
         * -v ${ENV_VARIABLE}/test:${ENV_VARIABLE}/test this will be expanded and if
         * ENV_VARIABLE is not defined then docker call will break. We need to support this
         * even if it is not ideal and simply skip the --volume/-v entry in that case.
         * We will print out a warning message and remove it from the list of volumes.
         */
        volumes.retain(|v| {
            cli.debug(format!("Validate docker volume '{}'", v));
            let parts: Vec<&str> = v.split_whitespace().collect();
            if parts.len() < 2 {
                cli.warn(format!("invalid docker volume '{}'", v));
                return false;
            }

            let volume_part: &str = parts[1];
            let v_path: &str = volume_part
                .split(':')
                .next()
                .ok_or_else(|| {
                    cli.warn(format!("invalid docker volume '{}'", v));
                    return false;
                })
                .unwrap();

            if v_path.is_empty() {
                cli.warn(format!("invalid docker volume '{}'", v));
                return false;
            }

            cli.debug(format!("Valid docker volume '{}'", v));
            true
        });
    }

    pub fn prepare_volumes(&self, cli: &Cli, volumes: &Vec<String>) -> Result<(), BError> {
        /*
         * The user can define a set of docker arguments in the workspace.json in the
         * args json node . We need to check if the volume dir exists and create
         * it to avoid docker from creating it because then it will be owned by root.
         */
        for v in volumes.iter() {
            cli.debug(format!("process docker volume arg: '{}'", v));
            let parts: Vec<&str> = v.split_whitespace().collect();
            if parts.len() < 2 {
                return Err(BError::DockerVolumeError(format!(
                    "Invalid docker volume arg '{}'",
                    v
                )));
            }

            let volume_part: &str = parts[1];
            let v_path: &str = volume_part.split(':').next().ok_or_else(|| {
                BError::DockerVolumeError(format!("Invalid docker volume arg '{}'", v))
            })?;

            if v_path.is_empty() {
                return Err(BError::DockerVolumeError(format!(
                    "Missing path to docker volume '{}'",
                    v
                )));
            }

            let volume_path: PathBuf = PathBuf::from(v_path);

            cli.debug(format!("prepare docker volume path: {:?}", volume_path));

            if !cli.exists(&volume_path) {
                let result: Result<(), BError> = cli.mkdir(&volume_path);
                if result.is_err() {
                    return Err(BError::DockerVolumeError(format!(
                        "Failed to mkdir docker volume with err '{:?}'",
                        result.err()
                    )));
                }
            }
        }

        Ok(())
    }

    pub fn copy_volumes(&self, args: &mut Vec<String>, volumes: &mut Vec<String>) {
        /*
         * The docker args can be extended from workspace.json and any --volume/-v needs to be
         * added to the docker volume args and then be removed from the args list.
         */
        args.retain(|e| {
            if e.contains("-v") || e.contains("--volume") {
                volumes.push(e.to_string());
                return false;
            }
            true
        });
    }

    pub fn bootstrap_volumes(
        &self,
        docker_top_dir: &PathBuf,
        volumes: &Vec<String>,
    ) -> Vec<String> {
        let mut docker_args: Vec<String> = vec![];
        docker_args.append(&mut self.top_dir(docker_top_dir));
        docker_args.append(&mut self.home_dir());
        docker_args.append(&mut self.volumes(volumes));
        docker_args.append(&mut self.bkry_volumes());
        docker_args
    }

    pub fn expand_env_variables(
        &self,
        cli: &Cli,
        env: &HashMap<String, String>,
        args: &mut Vec<String>,
    ) {
        for arg in args.iter_mut() {
            cli.debug(format!("expand: {}", arg));
            *arg = self.replace_env_variables(arg, env);
        }
    }

    pub fn bootstrap_bkry(
        &self,
        cmd_line: &Vec<String>,
        cli: &Cli,
        docker_top_dir: &PathBuf,
        work_dir: &PathBuf,
        docker_args: &Vec<String>,
        volumes: &Vec<String>,
        env: &HashMap<String, String>,
    ) -> Result<(), BError> {
        let mut docker_volumes: Vec<String> = vec![];
        let mut args: Vec<String> = docker_args.clone();

        cli.debug(format!("docker args: {:?}", docker_args));
        cli.debug(format!("docker volumes: {:?}", volumes));

        /*
         * Expand any env variables in docker args
         */
        self.expand_env_variables(cli, env, &mut args);
        /*
         * Copy volumes from the docker args into the docker volumes vector
         */
        self.copy_volumes(&mut args, &mut docker_volumes);
        /*
         * Validate all the docker volumes removing invalid --volumes that we
         * might have after expanding the env variables
         */
        self.validate_volumes(cli, &mut docker_volumes);
        docker_volumes.append(&mut self.bootstrap_volumes(docker_top_dir, volumes));
        /*
         * Prepare the volumes by creating the dir if not already existing
         */
        self.prepare_volumes(cli, &docker_volumes)?;

        cli.check_call(
            &self.bootstrap_cmd_line(cmd_line, work_dir, &mut args, &mut docker_volumes),
            &env,
            true,
        )?;

        Ok(())
    }

    pub fn run_cmd(
        &self,
        cmd_line: &Vec<String>,
        cli: &Cli,
        exec_dir: &PathBuf,
        _docker_args: &Vec<String>,
        _volumes: &Vec<String>,
        env: &HashMap<String, String>,
    ) -> Result<(), BError> {
        let temp_dir: TempDir = TempDir::new("bakery")?;
        let env_file_path: PathBuf = self.setup_env_file(temp_dir.path(), env)?;

        cli.check_call(
            &self.cmd_line(cmd_line, &env_file_path, exec_dir),
            &HashMap::new(),
            true,
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::Read;
    use std::path::PathBuf;
    use tempdir::TempDir;

    use crate::cli::*;
    use crate::error::BError;
    use crate::executers::{Docker, DockerImage};
    use crate::helper::Helper;

    #[test]
    fn test_dockerimage() {
        let image: DockerImage = DockerImage::new("test-registry/test/test-image:0.1")
            .expect("Invalid docker image format");
        assert_eq!(image.registry, "test-registry");
        assert_eq!(image.image, "test/test-image");
        assert_eq!(image.tag, "0.1");
    }

    #[test]
    fn test_dockerimage_long_namespace() {
        let image: DockerImage = DockerImage::new("test-registry/test/test/test-image:0.1")
            .expect("Invalid docker image format");
        assert_eq!(image.registry, "test-registry");
        assert_eq!(image.image, "test/test/test-image");
        assert_eq!(image.tag, "0.1");
    }

    #[test]
    fn test_dockerimage_short_namespace() {
        let image: DockerImage =
            DockerImage::new("test-registry/test-image:0.1").expect("Invalid docker image format");
        assert_eq!(image.registry, "test-registry");
        assert_eq!(image.image, "test-image");
        assert_eq!(image.tag, "0.1");
    }

    #[test]
    fn test_docker_bootstrap_cmdline() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = PathBuf::from(temp_dir.path());
        let docker_top_dir: PathBuf = work_dir.clone();
        let test_build_dir: PathBuf = work_dir.join(PathBuf::from("test_build_dir"));
        let test_cmd: Vec<String> = vec![
            String::from("cd"),
            format!("{}", test_build_dir.display()),
            String::from("&&"),
            String::from("test"),
        ];
        let interactive: bool = false;
        let image: DockerImage =
            DockerImage::new("test-registry/test-image:0.1").expect("Invalid docker image format");
        let docker: Docker = Docker::new(image.clone(), interactive);
        let volumes: Vec<String> = vec![];
        let mut docker_args: Vec<String> = vec![];
        let result: Vec<String> = docker.bootstrap_cmd_line(
            &test_cmd,
            &work_dir,
            &mut docker_args,
            &mut docker.bootstrap_volumes(&docker_top_dir, &volumes),
        );
        let cmd_line: Vec<String> = Helper::docker_bootstrap_string(
            interactive,
            &docker_args,
            &volumes,
            &docker_top_dir,
            &work_dir,
            &image,
            &test_cmd,
        );
        assert_eq!(result, cmd_line);
    }

    #[test]
    fn test_docker_bootstrap_cmdline_interactive() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = PathBuf::from(temp_dir.path());
        let docker_top_dir: PathBuf = work_dir.clone();
        let test_build_dir: PathBuf = work_dir.join(PathBuf::from("test_build_dir"));
        let test_cmd: Vec<String> = vec![
            String::from("cd"),
            format!("{}", test_build_dir.display()),
            String::from("&&"),
            String::from("test"),
        ];
        let interactive: bool = true;
        let image: DockerImage =
            DockerImage::new("test-registry/test-image:0.1").expect("Invalid docker image format");
        let docker: Docker = Docker::new(image.clone(), interactive);
        let volumes: Vec<String> = vec![];
        let docker_args: Vec<String> = vec![];
        let result: Vec<String> = docker.bootstrap_cmd_line(
            &test_cmd,
            &work_dir,
            &mut docker_args.clone(),
            &mut docker.bootstrap_volumes(&docker_top_dir, &volumes),
        );
        let cmd_line: Vec<String> = Helper::docker_bootstrap_string(
            interactive,
            &docker_args,
            &volumes,
            &docker_top_dir,
            &work_dir,
            &image,
            &test_cmd,
        );
        assert_eq!(result, cmd_line);
    }

    #[test]
    fn test_docker_bootstrap_args() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = PathBuf::from(temp_dir.path());
        let docker_top_dir: PathBuf = work_dir.clone();
        let test_build_dir: PathBuf = work_dir.join(PathBuf::from("test_build_dir"));
        let test_cmd: Vec<String> = vec![
            String::from("cd"),
            format!("{}", test_build_dir.display()),
            String::from("&&"),
            String::from("test"),
        ];
        let interactive: bool = false;
        let image: DockerImage =
            DockerImage::new("test-registry/test-image:0.1").expect("Invalid docker image format");
        let docker: Docker = Docker::new(image.clone(), interactive);
        let volumes: Vec<String> = vec![];
        let docker_args: Vec<String> = vec![String::from("--test"), String::from("test")];
        let result: Vec<String> = docker.bootstrap_cmd_line(
            &test_cmd,
            &work_dir,
            &mut docker_args.clone(),
            &mut docker.bootstrap_volumes(&docker_top_dir, &volumes),
        );
        let cmd_line: Vec<String> = Helper::docker_bootstrap_string(
            interactive,
            &docker_args,
            &volumes,
            &docker_top_dir,
            &work_dir,
            &image,
            &test_cmd,
        );
        assert_eq!(result, cmd_line);
    }

    #[test]
    fn test_docker_bootstrap_volumes() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = PathBuf::from(temp_dir.path());
        let docker_top_dir: PathBuf = work_dir.clone();
        let test_build_dir: PathBuf = work_dir.join(PathBuf::from("test_build_dir"));
        let test_cmd: Vec<String> = vec![
            String::from("cd"),
            format!("{}", test_build_dir.display()),
            String::from("&&"),
            String::from("test"),
        ];
        let interactive: bool = false;
        let image: DockerImage =
            DockerImage::new("test-registry/test-image:0.1").expect("Invalid docker image format");
        let docker: Docker = Docker::new(image.clone(), interactive);
        let volumes: Vec<String> = vec![String::from("/test/testdir:/test/testdir")];
        let mut docker_args: Vec<String> = vec![];
        let result: Vec<String> = docker.bootstrap_cmd_line(
            &test_cmd,
            &work_dir,
            &mut docker_args,
            &mut docker.bootstrap_volumes(&docker_top_dir, &volumes),
        );
        let cmd_line: Vec<String> = Helper::docker_bootstrap_string(
            interactive,
            &docker_args,
            &volumes,
            &docker_top_dir,
            &work_dir,
            &image,
            &test_cmd,
        );
        assert_eq!(result, cmd_line);
    }

    #[test]
    fn test_docker_bootstrap_top_dir() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = PathBuf::from(temp_dir.path());
        let docker_top_dir: PathBuf = work_dir.clone().join(PathBuf::from("../../"));
        let test_build_dir: PathBuf = work_dir.clone().join(PathBuf::from("test_build_dir"));
        let test_cmd: Vec<String> = vec![
            String::from("cd"),
            format!("{}", test_build_dir.display()),
            String::from("&&"),
            String::from("test"),
        ];
        let interactive: bool = false;
        let image: DockerImage =
            DockerImage::new("test-registry/test-image:0.1").expect("Invalid docker image format");
        let docker: Docker = Docker::new(image.clone(), interactive);
        let volumes: Vec<String> = vec![];
        let mut docker_args: Vec<String> = vec![];
        let result: Vec<String> = docker.bootstrap_cmd_line(
            &test_cmd,
            &work_dir,
            &mut docker_args,
            &mut docker.bootstrap_volumes(&docker_top_dir, &volumes),
        );
        let cmd_line: Vec<String> = Helper::docker_bootstrap_string(
            interactive,
            &docker_args,
            &volumes,
            &docker_top_dir,
            &work_dir,
            &image,
            &test_cmd,
        );
        assert_eq!(result, cmd_line);
    }

    #[test]
    fn test_docker_env_file() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let image: DockerImage =
            DockerImage::new("test-registry/test-image:0.1").expect("Invalid docker image format");
        let docker: Docker = Docker::new(image.clone(), true);
        let env: HashMap<String, String> = HashMap::from([
            (String::from("TEST_KEY1"), String::from("TEST_VALUE1")),
            (String::from("TEST_KEY2"), String::from("TEST_VALUE2")),
        ]);
        let env_str1 = r#"TEST_KEY1=TEST_VALUE1
TEST_KEY2=TEST_VALUE2
"#;
        let env_str2 = r#"TEST_KEY2=TEST_VALUE2
TEST_KEY1=TEST_VALUE1
"#;
        let env_file: PathBuf = docker
            .setup_env_file(temp_dir.path(), &env)
            .expect("Failed to setup env file");
        assert!(env_file.exists());
        let mut file: File = File::open(&env_file).expect("Failed to open env file!");
        let mut contents: String = String::new();
        file.read_to_string(&mut contents)
            .expect("Failed to read env file!");
        if contents == env_str1 {
            assert_eq!(env_str1, contents);
        } else {
            assert_eq!(env_str2, contents);
        }
    }

    #[test]
    fn test_docker_cmdline() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = PathBuf::from(temp_dir.path());
        let env_file: PathBuf = work_dir.clone().join("test-docker.env");
        let test_build_dir: PathBuf = work_dir.join(PathBuf::from("test_build_dir"));
        let test_cmd: Vec<String> = vec![
            String::from("cd"),
            format!("{}", test_build_dir.display()),
            String::from("&&"),
            String::from("test"),
        ];
        let interactive: bool = false;
        let image: DockerImage =
            DockerImage::new("test-registry/test-image:0.1").expect("Invalid docker image format");
        let docker: Docker = Docker::new(image.clone(), interactive);
        let result: Vec<String> = docker.cmd_line(&test_cmd, &env_file, &work_dir);
        let cmd_line: Vec<String> =
            Helper::docker_cmdline_string(interactive, &work_dir, &image, &test_cmd, &env_file);
        assert_eq!(result, cmd_line);
    }

    #[test]
    fn test_docker_cmdline_interactive() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = PathBuf::from(temp_dir.path());
        let env_file: PathBuf = work_dir.clone().join("test-docker.env");
        let test_build_dir: PathBuf = work_dir.join(PathBuf::from("test_build_dir"));
        let test_cmd: Vec<String> = vec![
            String::from("cd"),
            format!("{}", test_build_dir.display()),
            String::from("&&"),
            String::from("test"),
        ];
        let interactive: bool = true;
        let image: DockerImage =
            DockerImage::new("test-registry/test-image:0.1").expect("Invalid docker image format");
        let docker: Docker = Docker::new(image.clone(), interactive);
        let result: Vec<String> = docker.cmd_line(&test_cmd, &env_file, &work_dir);
        let cmd_line: Vec<String> =
            Helper::docker_cmdline_string(interactive, &work_dir, &image, &test_cmd, &env_file);
        assert_eq!(result, cmd_line);
    }

    #[test]
    fn test_docker_validate_volumes() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = PathBuf::from(temp_dir.path());
        let interactive: bool = false;
        let image: DockerImage =
            DockerImage::new("test-registry/test-image:0.1").expect("Invalid docker image format");
        let docker: Docker = Docker::new(image.clone(), interactive);
        let test_volume_dir: PathBuf = work_dir.join(PathBuf::from("test_build_dir"));
        let mut volumes: Vec<String> = vec![
            String::from(format!(
                "-v {}:{}",
                test_volume_dir.to_str().unwrap(),
                test_volume_dir.to_str().unwrap()
            )),
            String::from(format!("-v :{}", test_volume_dir.to_str().unwrap())),
            String::from("-v :"),
            String::from("-v"),
            String::from(format!(
                "--volume {}:{}",
                test_volume_dir.to_str().unwrap(),
                test_volume_dir.to_str().unwrap()
            )),
            String::from(format!("--volume :{}", test_volume_dir.to_str().unwrap())),
            String::from("--volume :"),
            String::from("--volume"),
        ];
        let mut mocked_logger: MockLogger = MockLogger::new();
        mocked_logger
            .expect_warn()
            .with(mockall::predicate::eq(format!(
                "invalid docker volume '-v :{}'",
                test_volume_dir.to_str().unwrap()
            )))
            .once()
            .returning(|_x| ());
        mocked_logger
            .expect_warn()
            .with(mockall::predicate::eq(
                "invalid docker volume '-v :'".to_string(),
            ))
            .once()
            .returning(|_x| ());
        mocked_logger
            .expect_warn()
            .with(mockall::predicate::eq(
                "invalid docker volume '-v'".to_string(),
            ))
            .once()
            .returning(|_x| ());
        mocked_logger
            .expect_warn()
            .with(mockall::predicate::eq(format!(
                "invalid docker volume '--volume :{}'",
                test_volume_dir.to_str().unwrap()
            )))
            .once()
            .returning(|_x| ());
        mocked_logger
            .expect_warn()
            .with(mockall::predicate::eq(
                "invalid docker volume '--volume :'".to_string(),
            ))
            .once()
            .returning(|_x| ());
        mocked_logger
            .expect_warn()
            .with(mockall::predicate::eq(
                "invalid docker volume '--volume'".to_string(),
            ))
            .once()
            .returning(|_x| ());
        let cli: Cli = Cli::new(
            Box::new(mocked_logger),
            Box::new(BSystem::new()),
            clap::Command::new("bakery"),
            None,
        );
        docker.validate_volumes(&cli, &mut volumes);
        assert_eq!(
            volumes,
            vec![
                String::from(format!(
                    "-v {}:{}",
                    test_volume_dir.to_str().unwrap(),
                    test_volume_dir.to_str().unwrap()
                )),
                String::from(format!(
                    "--volume {}:{}",
                    test_volume_dir.to_str().unwrap(),
                    test_volume_dir.to_str().unwrap()
                )),
            ]
        )
    }

    #[test]
    fn test_docker_copy_volumes() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = PathBuf::from(temp_dir.path());
        let interactive: bool = false;
        let image: DockerImage =
            DockerImage::new("test-registry/test-image:0.1").expect("Invalid docker image format");
        let docker: Docker = Docker::new(image.clone(), interactive);
        let test_volume_dir: PathBuf = work_dir.join(PathBuf::from("test_build_dir"));
        let mut args: Vec<String> = vec![
            String::from(format!(
                "-v {}:{}",
                test_volume_dir.to_str().unwrap(),
                test_volume_dir.to_str().unwrap()
            )),
            String::from(format!(
                "--volume {}:{}",
                test_volume_dir.to_str().unwrap(),
                test_volume_dir.to_str().unwrap()
            )),
            String::from(format!("-w {}", test_volume_dir.to_str().unwrap())),
            String::from(format!("-e ENV_VAR={}", test_volume_dir.to_str().unwrap())),
        ];
        let mut volumes: Vec<String> = vec![];
        docker.copy_volumes(&mut args, &mut volumes);
        assert_eq!(
            volumes,
            vec![
                String::from(format!(
                    "-v {}:{}",
                    test_volume_dir.to_str().unwrap(),
                    test_volume_dir.to_str().unwrap()
                )),
                String::from(format!(
                    "--volume {}:{}",
                    test_volume_dir.to_str().unwrap(),
                    test_volume_dir.to_str().unwrap()
                )),
            ]
        );
        assert_eq!(
            args,
            vec![
                String::from(format!("-w {}", test_volume_dir.to_str().unwrap())),
                String::from(format!("-e ENV_VAR={}", test_volume_dir.to_str().unwrap())),
            ]
        )
    }

    #[test]
    fn test_docker_prepare_volumes() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = PathBuf::from(temp_dir.path());
        let interactive: bool = false;
        let image: DockerImage =
            DockerImage::new("test-registry/test-image:0.1").expect("Invalid docker image format");
        let docker: Docker = Docker::new(image.clone(), interactive);
        let test_volume_dir: PathBuf = work_dir.join(PathBuf::from("test_build_dir"));
        let volumes: Vec<String> = vec![String::from(format!(
            "-v {}:{}",
            test_volume_dir.to_str().unwrap(),
            test_volume_dir.to_str().unwrap()
        ))];
        let mut mocked_system: MockSystem = MockSystem::new();
        mocked_system
            .expect_exists()
            .with(mockall::predicate::eq(test_volume_dir.clone()))
            .once()
            .returning(|_x| false);
        mocked_system
            .expect_mkdir()
            .with(mockall::predicate::eq(test_volume_dir))
            .once()
            .returning(|_x| Ok(()));
        let cli: Cli = Cli::new(
            Box::new(BLogger::new()),
            Box::new(mocked_system),
            clap::Command::new("bakery"),
            None,
        );
        let _result: Result<(), BError> = docker.prepare_volumes(&cli, &volumes);
    }

    #[test]
    fn test_docker_prepare_volumes_error_1() {
        let temp_dir: TempDir =
            TempDir::new("bakery-test-dir").expect("Failed to create temp directory");
        let work_dir: PathBuf = PathBuf::from(temp_dir.path());
        let interactive: bool = false;
        let image: DockerImage =
            DockerImage::new("test-registry/test-image:0.1").expect("Invalid docker image format");
        let docker: Docker = Docker::new(image.clone(), interactive);
        let test_volume_dir: PathBuf = work_dir.join(PathBuf::from("test_build_dir"));
        /*
         * Bad format -v :/path/to/dir should result in error
         */
        let volumes: Vec<String> = vec![String::from(format!(
            "-v :{}",
            test_volume_dir.to_str().unwrap()
        ))];
        let cli: Cli = Cli::new(
            Box::new(BLogger::new()),
            Box::new(BSystem::new()),
            clap::Command::new("bakery"),
            None,
        );
        let result: Result<(), BError> = docker.prepare_volumes(&cli, &volumes);
        match result {
            Ok(_) => {
                // If it returns Ok, the test should fail
                panic!("Expected an error, but got Ok");
            }
            Err(e) => {
                // Check the error message
                assert_eq!(
                    e.to_string(),
                    format!(
                        "Missing path to docker volume '-v :{}'",
                        test_volume_dir.to_str().unwrap()
                    )
                );
            }
        }
    }

    #[test]
    fn test_docker_prepare_volumes_error_2() {
        let interactive: bool = false;
        let image: DockerImage =
            DockerImage::new("test-registry/test-image:0.1").expect("Invalid docker image format");
        let docker: Docker = Docker::new(image.clone(), interactive);
        /*
         * Bad format --volume should result in error
         */
        let volumes: Vec<String> = vec![String::from("--volume")];
        let cli: Cli = Cli::new(
            Box::new(BLogger::new()),
            Box::new(BSystem::new()),
            clap::Command::new("bakery"),
            None,
        );
        let result: Result<(), BError> = docker.prepare_volumes(&cli, &volumes);
        match result {
            Ok(_) => {
                // If it returns Ok, the test should fail
                panic!("Expected an error, but got Ok");
            }
            Err(e) => {
                // Check the error message
                assert_eq!(
                    e.to_string(),
                    "Invalid docker volume arg '--volume'".to_string()
                );
            }
        }
    }

    #[test]
    fn test_docker_expand_env_variables() {
        let interactive: bool = false;
        let image: DockerImage =
            DockerImage::new("test-registry/test-image:0.1").expect("Invalid docker image format");
        let docker: Docker = Docker::new(image.clone(), interactive);
        let cli: Cli = Cli::new(
            Box::new(BLogger::new()),
            Box::new(BSystem::new()),
            clap::Command::new("bakery"),
            None,
        );
        let mut env: HashMap<String, String> = HashMap::new();
        env.insert("TEST_ENV".to_string(), "/test/dir".to_string());
        let mut args: Vec<String> = vec![
            "-v ${TEST_ENV}:${TEST_ENV}".to_string(),
            "-e TEST_ENV=${TEST_ENV}".to_string(),
            "-v ${NA}:${NA}".to_string(),
            "-e NA=${NA}".to_string(),
        ];
        docker.expand_env_variables(&cli, &env, &mut args);
        assert_eq!(
            args,
            vec![
                "-v /test/dir:/test/dir".to_string(),
                "-e TEST_ENV=/test/dir".to_string(),
                "-v :".to_string(),
                "-e NA=".to_string(),
            ]
        )
    }
}
