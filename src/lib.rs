
    use std::convert::identity;
    use std::error::Error;
    use std::fmt::{format, Display};
    use std::{collections::BTreeMap, borrow::Cow};
    use std::env;
    use std::os::unix;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use regex::{Regex, Captures};
    use serde::{Serialize, Deserialize};



#[derive(Serialize, Deserialize)]
pub struct EnvStr {
    str: String
}
impl From<String> for EnvStr {
    fn from(s: String) -> Self { EnvStr { str: s } }
}
impl From<&str> for EnvStr {
    fn from(s: &str) -> Self { EnvStr { str: String::from(s) } }
}
    
impl EnvStr {
    pub fn str(&self) -> Cow<'_, str> {
        Regex::new(r"\$[_a-zA-Z][_a-zA-Z0-9]*").unwrap().replace_all(&self.str, |caps: &Captures| -> String {
            match env::var(caps[0][1..].to_string()) {
                Ok(res) => res,
                Err(_) => {
                    println!("cargo:warning=Env var: `{:?}` not present", &caps[0][1..]);
                    caps[0].to_string()
                },
            }
        })
    }

    pub fn path(&self) -> std::io::Result<PathBuf> { std::fs::canonicalize(self.str().into_owned()) }
}


#[derive(Serialize, Deserialize)]
pub struct ValueAlternatives {
    alt: Vec<EnvStr>,
}

impl ValueAlternatives {
    pub fn new(alt: Vec<EnvStr>) -> Self {
        ValueAlternatives { alt: alt }
    }

    pub fn into_env<F: Fn(&String) -> bool>(self, key: &String, predicate: &F) -> bool {
        self
            .alt
            .into_iter().map(|x| x.str().into_owned())
            .find(predicate)
            .map(|v| env::set_var(key, &v))
            .is_some()
    }
}

impl From<EnvStr> for ValueAlternatives {
    fn from(s: EnvStr) -> Self {
        ValueAlternatives::new(vec![s])
    }
}
impl From<&str> for ValueAlternatives {
    fn from(s: &str) -> Self {
        ValueAlternatives::from(EnvStr::from(s))
    }
}


#[derive(Serialize, Deserialize)]
pub enum LinkSourceType {
    Direct,
    Env
}

#[derive(Serialize, Deserialize)]
pub struct LinkSource {
    source_type: LinkSourceType,
    value: String 
}

impl LinkSource {
    pub fn new(source_type: LinkSourceType, value: String) -> Self {
        LinkSource { source_type: source_type, value: value }
    }
    /// link to current working directory
    pub fn link_to<Q: AsRef<Path>>(self, link: Q) -> std::io::Result<()> {
        match self.source_type {
            LinkSourceType::Direct => unix::fs::symlink(self.value, link),
            LinkSourceType::Env => match env::var(&self.value) {
                Ok(o) => unix::fs::symlink(o, link),
                Err(_) => {
                    Ok(println!("cargo:warning=Env var `{}` not found", self.value))
                },
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct BuildConfiguration {
    env: BTreeMap<String, ValueAlternatives>,
    sources: Vec<EnvStr>,
    links: Vec<LinkSource>
}

#[derive(Debug)]
pub enum DumpEnvError {
    IOError(std::io::Error),
    FromUtf8Error(std::string::FromUtf8Error),
    NotImplemented
}

#[cfg(target_os = "linux")]
pub fn dump_environment(bash_file: &String) -> Result<BTreeMap<String, String>, DumpEnvError> {
    Command::new("/bin/bash")
        .arg("-c")
        .arg(format!(". {} && env", bash_file))
        .output()
        .map_err(|err| DumpEnvError::IOError(err))
        .and_then(|r| String::from_utf8(r.stdout)
            .map_err(|err| DumpEnvError::FromUtf8Error(err))
            .map(|out| out
                .split('\n')
                .map(|pair| pair.split_once('='))
                .filter_map(identity)
                .map(|(k, v)| (String::from(k), String::from(v)))
                .collect::<BTreeMap<String, String>>()
            )
        )
}

#[cfg(not(target_os = "linux"))]
pub fn dump_environment(_bash_file: &String) -> Result<BTreeMap<String, String>, DumpEnvError> {
    DumpEnvError::NotImplemented
}

pub fn merge_environment(top: BTreeMap<String, String>) {
    for (k, v) in top {
        env::set_var(k, v)
    }
}

impl BuildConfiguration {
    pub fn new(env: BTreeMap<String, ValueAlternatives>, sources: Vec<EnvStr>, links: Vec<LinkSource>) -> Self {
        BuildConfiguration { env: env, sources: sources, links: links }
    }
    pub fn into_env<F: Fn(&String) -> bool>(self, predicate: &F) {
        for src in self.sources.into_iter() {
            let cmd = src.path().unwrap();
            println!("cargo:warning=source: `{:?}`", &cmd);
            merge_environment(dump_environment(&String::from(cmd
                .as_os_str()
                .to_str().unwrap())
            ).unwrap());
        }
        for (k, v) in self.env.into_iter() {
            if !v.into_env(&k, predicate) {
                println!("cargo:warning=No value alternative exist for key `{}`", k)
            }
        }
        for l in self.links.into_iter() {
            l.link_to(env::current_dir().unwrap().as_path()).unwrap()
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct BuildConfigProvider {
    targets: BTreeMap<String, BuildConfiguration>,
    default: BuildConfiguration
}

impl BuildConfigProvider {
    pub fn new(targets: BTreeMap<String, BuildConfiguration>, default: BuildConfiguration) -> Self {
        BuildConfigProvider { targets: targets, default: default }
    }


    pub fn get(self, target_triple: &String) -> BuildConfiguration {
        match self.targets.into_iter().find(|(k, _)| k == target_triple) {
            Some((_, v)) => v,
            None => self.default,
        }
    } 

    pub fn get_from_env(self, target_triple_key: &String) -> BuildConfiguration {
        self.get(&env::var(target_triple_key).map_err(|_| format!("can not find env variable: {}", target_triple_key)).unwrap())
    }
}
