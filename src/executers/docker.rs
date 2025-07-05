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
        vec![
            String::from("-u"),
            format!("{}:{}", users::get_current_uid(), users::get_current_gid()),
        ]
    }

    fn etc_files(&self) -> Vec<String> {
        vec![
            String::from("-v"),
            String::from("/etc/passwd:/etc/passwd:ro"),
            String::from("-v"),
            String::from("/etc/group:/etc/group:ro"),
        ]
    }

    fn bakery(&self) -> Vec<String> {
        vec![
            String::from("-v"),
            String::from(format!("{}:{}:ro", BkryConstants::BIN, BkryConstants::BIN)),
            String::from("-v"),
            String::from(format!(
                "{}:{}:ro",
                BkryConstants::CFG_DIR,
                BkryConstants::CFG_DIR
            )),
            String::from("-v"),
            String::from(format!(
                "{}:{}:ro",
                BkryConstants::OPT_DIR,
                BkryConstants::OPT_DIR
            )),
        ]
    }

    fn _hidden_home_files(&self) -> Vec<String> {
        vec![
            String::from("-v"),
            format!(
                "{}/.gitconfig:{}/.gitconfig:rw",
                self.env_home(),
                self.env_home()
            ),
            String::from("-v"),
            format!("{}/.ssh:{}/.ssh:rw", self.env_home(), self.env_home()),
            String::from("-v"),
            format!("{}/.docker:{}/.docker", self.env_home(), self.env_home()),
            String::from("-v"),
            format!("{}/.bakery:{}/.bakery", self.env_home(), self.env_home()),
        ]
    }

    fn home_dir(&self) -> Vec<String> {
        vec![
            String::from("-v"),
            format!("{}:{}", self.env_home(), self.env_home()),
        ]
    }

    fn work_dir(&self, dir: &PathBuf) -> Vec<String> {
        vec![String::from("-w"), format!("{}", dir.display())]
    }

    fn docker_sock(&self) -> Vec<String> {
        vec![
            String::from("-v"),
            String::from("/var/run/docker.sock:/var/run/docker.sock"),
        ]
    }

    fn group(&self) -> Vec<String> {
        let cache: users::UsersCache = users::UsersCache::new();
        vec![
            String::from("--group-add"),
            cache.get_group_by_name("docker").unwrap().gid().to_string(),
        ]
    }

    fn env_file(&self, env_file: &PathBuf) -> Vec<String> {
        vec![
            String::from("--env-file"),
            env_file.to_string_lossy().to_string(),
        ]
    }

    fn volumes(&self, volumes: &Vec<String>) -> Vec<String> {
        let mut v: Vec<String> = Vec::new();
        volumes.iter().for_each(|e| {
            v.append(&mut vec![String::from("-v"), e.to_string()]);
        });
        v.append(&mut self.bakery());
        v.append(&mut self.etc_files());
        v.append(&mut self.docker_sock());
        v
    }

    fn container_name(&self, name: &str) -> Vec<String> {
        vec![
            String::from("--name"),
            format!("{}-{}", name, std::process::id()),
        ]
    }

    fn top_dir(&self, dir: &PathBuf) -> Vec<String> {
        vec![
            String::from("-v"),
            format!("{}:{}", dir.display(), dir.display()),
        ]
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
        docker_top_dir: &PathBuf,
        work_dir: &PathBuf,
        docker_args: &Vec<String>,
        volumes: &Vec<String>,
    ) -> Vec<String> {
        let mut docker_cmd: Vec<String> = vec!["docker".to_string(), "run".to_string()];
        docker_cmd.append(&mut self.container_name("bakery-workspace"));
        docker_cmd.append(&mut vec!["-t".to_string(), "--rm".to_string()]);
        if self._interactive {
            docker_cmd.push("-i".to_string());
        }
        docker_cmd.append(&mut self.group());
        docker_cmd.append(&mut self.volumes(volumes));
        docker_cmd.append(&mut self.user());
        docker_cmd.append(&mut self.top_dir(docker_top_dir));
        docker_cmd.append(&mut self.work_dir(work_dir));
        if !docker_args.is_empty() {
            docker_cmd.append(&mut docker_args.clone());
        }
        docker_cmd.push(format!("{}", self.image));
        docker_cmd.append(&mut cmd_line.clone());
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

    pub fn bootstrap_bakery(
        &self,
        cmd_line: &Vec<String>,
        cli: &Cli,
        docker_top_dir: &PathBuf,
        work_dir: &PathBuf,
        docker_args: &Vec<String>,
        volumes: &Vec<String>,
        env: &HashMap<String, String>,
    ) -> Result<(), BError> {
        if self.pull(cli).is_err() {
            cli.debug(format!("Failed to pull the Docker image '{}'.", self.image));
        }

        cli.check_call(
            &self.bootstrap_cmd_line(cmd_line, docker_top_dir, work_dir, docker_args, volumes),
            &env,
            true,
        )?;
        Ok(())
    }

    pub fn run_cmd(
        &self,
        cmd_line: &Vec<String>,
        env: &HashMap<String, String>,
        exec_dir: &PathBuf,
        cli: &Cli,
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
        let volumes: Vec<String> = vec![];
        let interactive: bool = false;
        let docker_args: Vec<String> = vec![];
        let image: DockerImage =
            DockerImage::new("test-registry/test-image:0.1").expect("Invalid docker image format");
        let docker: Docker = Docker::new(image.clone(), interactive);
        let result: Vec<String> = docker.bootstrap_cmd_line(
            &test_cmd,
            &docker_top_dir,
            &work_dir,
            &docker_args,
            &volumes,
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
        let volumes: Vec<String> = vec![];
        let interactive: bool = true;
        let docker_args: Vec<String> = vec![];
        let image: DockerImage =
            DockerImage::new("test-registry/test-image:0.1").expect("Invalid docker image format");
        let docker: Docker = Docker::new(image.clone(), interactive);
        let result: Vec<String> = docker.bootstrap_cmd_line(
            &test_cmd,
            &docker_top_dir,
            &work_dir,
            &docker_args,
            &volumes,
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
        let volumes: Vec<String> = vec![];
        let interactive: bool = false;
        let docker_args: Vec<String> = vec![String::from("--test"), String::from("test")];
        let image: DockerImage =
            DockerImage::new("test-registry/test-image:0.1").expect("Invalid docker image format");
        let docker: Docker = Docker::new(image.clone(), interactive);
        let result: Vec<String> = docker.bootstrap_cmd_line(
            &test_cmd,
            &docker_top_dir,
            &work_dir,
            &docker_args,
            &volumes,
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
        let volumes: Vec<String> = vec![String::from("/test/testdir:/test/testdir")];
        let interactive: bool = false;
        let docker_args: Vec<String> = vec![];
        let image: DockerImage =
            DockerImage::new("test-registry/test-image:0.1").expect("Invalid docker image format");
        let docker: Docker = Docker::new(image.clone(), interactive);
        let result: Vec<String> = docker.bootstrap_cmd_line(
            &test_cmd,
            &docker_top_dir,
            &work_dir,
            &docker_args,
            &volumes,
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
        let volumes: Vec<String> = vec![];
        let interactive: bool = false;
        let docker_args: Vec<String> = vec![];
        let image: DockerImage =
            DockerImage::new("test-registry/test-image:0.1").expect("Invalid docker image format");
        let docker: Docker = Docker::new(image.clone(), interactive);
        let result: Vec<String> = docker.bootstrap_cmd_line(
            &test_cmd,
            &docker_top_dir,
            &work_dir,
            &docker_args,
            &volumes,
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
}
