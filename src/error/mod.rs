use std::path::StripPrefixError;

use thiserror::Error;
use zip::result::ZipError;

#[derive(Error, Debug)]
pub enum BError {
    #[error("{0}")]
    ParseError(String),
    #[error("Invalid 'artifact' node in build config. {0}")]
    ParseArtifactsError(String),
    #[error("Invalid 'task' node in build config. {0}")]
    ParseTasksError(String),
    #[error("Invalid 'manifest' node in build config. {0}")]
    ParseManifestError(String),
    #[error("Failed to parse JSON. '{0}'")]
    JsonParseError(String),
    #[error("The build config version '{0}' is not compatible with current bakery version. Update config to match the format of version '6'")]
    InvalidBuildConfigError(String),
    #[error("Invalid bakery workspace: 'workspace.json' file not found. Expected in current directory, ~/.bakery/, or /etc/bakery/")]
    InvalidWorkspaceError(),
    #[error("{0}")]
    IOError(String),
    #[error("{0}")]
    ValueError(String),
    #[error("{0}")]
    WsError(String),
    #[error("{0}")]
    CliError(String),
    #[error("{0}")]
    ArchiverError(String),
    #[error("{0}")]
    CmdError(String),
    #[error("{0}")]
    CtxKeyError(String),
    #[error("{0}")]
    CollectorError(String),
    #[error("Docker is enabled for the workspace but cannot locate /usr/bin/docker")]
    DockerError(),
    #[error("{0}")]
    DockerImageError(String),
    #[error("The workspace '{0}' is not empty! To setup workspace run 'bakery setup' in an empty directory. To sync run 'bakery sync'")]
    WorkspaceNotEmpty(String),
}

impl std::convert::From<serde_json::Error> for BError {
    fn from(err: serde_json::Error) -> Self {
        BError::JsonParseError(err.to_string())
    }
}

impl std::convert::From<std::io::Error> for BError {
    fn from(err: std::io::Error) -> Self {
        BError::IOError(err.to_string())
    }
}

impl std::convert::From<StripPrefixError> for BError {
    fn from(err: StripPrefixError) -> Self {
        BError::ArchiverError(err.to_string())
    }
}

impl std::convert::From<ZipError> for BError {
    fn from(err: ZipError) -> Self {
        BError::ArchiverError(err.to_string())
    }
}

impl std::convert::From<glob::PatternError> for BError {
    fn from(err: glob::PatternError) -> Self {
        BError::CollectorError(err.to_string())
    }
}

impl std::convert::From<glob::GlobError> for BError {
    fn from(err: glob::GlobError) -> Self {
        BError::CollectorError(err.to_string())
    }
}

impl std::convert::From<std::str::Utf8Error> for BError {
    fn from(err: std::str::Utf8Error) -> Self {
        BError::ValueError(err.to_string())
    }
}
