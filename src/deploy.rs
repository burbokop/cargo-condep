use std::{path::{Path, PathBuf}, fmt::{Display, Debug}};




pub struct DeploySource {
    pub execs: Vec<PathBuf>,
    pub libs: Vec<PathBuf>,
    pub config_files: Vec<PathBuf>,
    pub user_files: Vec<PathBuf>
}
pub struct DeployConfig {
    pub execs_path: PathBuf,
    pub libs_path: PathBuf,
    pub config_path: PathBuf,
    pub user_path: PathBuf
}

#[derive(Debug)]
pub enum ErrorKind {
    CopyFiles
}

pub struct DeployError {
    kind: ErrorKind,
    cause: Box<dyn Display>
}

impl Display for DeployError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: ", self.kind)?;
        self.cause.fmt(f)
    }
}

impl Debug for DeployError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: ", self.kind)?;
        self.cause.fmt(f)
    }
}

impl DeployError {
    pub fn new(kind: ErrorKind, cause: Box<dyn Display>) -> Self {
        return DeployError { kind: kind, cause: cause };
    }
    pub fn new_copy_err(cause: Box<dyn Display>) -> Self {
        return DeployError { kind: ErrorKind::CopyFiles, cause: cause };
    }
}

pub type DeployResult<T> = std::result::Result<T, DeployError>;

impl DeployConfig {
    pub fn copy_files<F>(self, src: DeploySource, f: &mut F) -> DeployResult<()>
    where
        F: FnMut(&Path, &Path) -> DeployResult<()> {
            for exe in src.execs { f(exe.as_path(), self.execs_path.as_path())? }
            for lib in src.libs { f(lib.as_path(), self.libs_path.as_path())? }
            for cfg in src.config_files { f(cfg.as_path(), self.config_path.as_path())? }
            for usr in src.user_files { f(usr.as_path(), self.user_path.as_path())? }
            Ok(())
    }
}

pub trait Deploy {
    fn deploy(&mut self, src: DeploySource, conf: DeployConfig) -> DeployResult<()>;
}

pub trait CallRemote {
    fn call_remote(&mut self, cmd: &[u8]) -> DeployResult<()>;
}


pub struct Noop {}

impl Deploy for Noop {
    fn deploy(&mut self, _: DeploySource, _: DeployConfig) -> DeployResult<()> {
        unimplemented!()
    }
}

impl CallRemote for Noop {
    fn call_remote(&mut self, _: &[u8]) -> DeployResult<()> {
        unimplemented!()
    }
}