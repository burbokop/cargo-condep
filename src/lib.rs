
use std::convert::identity;
use std::fs;
use std::{collections::BTreeMap, borrow::Cow};
use std::env::{self, VarError, JoinPathsError};
use std::os::unix;
use std::path::{Path, PathBuf};
use std::process::Command;
use regex::{Regex, Captures};
use serde::{Serialize, Deserialize};

mod print {
    use termion::color;

    pub fn info(header: &str, str: String) {
        println!("{}{}{} {}", color::Fg(color::LightGreen), header, color::Reset{}.fg_str(), str)
    }
    
    pub fn warning(header: &str, str: String) {
        println!("{}{}{} {}", color::Fg(color::LightYellow), header, color::Reset{}.fg_str(), str)    
    }
    
    pub fn fatal(header: &str, str: String) {
        panic!("{}{}{} {}", color::Fg(color::LightRed), header, color::Reset{}.fg_str(), str)
    }    
}



#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
pub enum VarAction {
    Set,
    Append
}

impl VarAction {
    pub fn do_action(&self, key: &String, value: &String) -> bool {
        match self {
            VarAction::Set => { env::set_var(key, value); true },
            VarAction::Append => {
                match env::var(key) {
                    Ok(path) => {
                        let mut paths = env::split_paths(&path).collect::<Vec<_>>();
                        paths.push(PathBuf::from(value));
                        match env::join_paths(paths) {
                            Ok(new_path) => Ok(env::set_var(key, &new_path)),
                            Err(_) => Err(())
                        }
                    },
                    Err(_) => Err(()),
                }.map_err(|_| env::set_var(key, value)).is_ok()
            } 
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ValueAlternatives {
    alt: Vec<EnvStr>,
    action: VarAction
}


impl ValueAlternatives {
    pub fn new(alt: Vec<EnvStr>, action: VarAction) -> Self {
        ValueAlternatives { alt: alt, action: action }
    }

    pub fn action(&self) -> &VarAction { &self.action }

    pub fn one(alt: EnvStr, action: VarAction) -> Self {
        ValueAlternatives { alt: vec![alt], action: action }
    }

    pub fn one_str(alt: &str, action: VarAction) -> Self {
        ValueAlternatives { alt: vec![EnvStr::from(alt)], action: action }
    }

    pub fn into_env<F: Fn(&String) -> bool>(self, key: &String, predicate: &F) -> Option<String> {
        self
            .alt
            .into_iter().map(|x| x.str().into_owned())
            .find(predicate)
            .map(|v| { self.action.do_action(key, &v); v })
    }

    pub fn setup_env<F: Fn(&String) -> bool>(&self, key: &String, predicate: &F) -> Option<String> {
        self
            .alt
            .iter().map(|x| x.str().into_owned())
            .find(predicate)
            .map(|v| { self.action.do_action(key, &v); v })
    }
}

impl From<EnvStr> for ValueAlternatives {
    fn from(s: EnvStr) -> Self {
        ValueAlternatives::new(vec![s], VarAction::Set)
    }
}
impl From<&str> for ValueAlternatives {
    fn from(s: &str) -> Self {
        ValueAlternatives::from(EnvStr::from(s))
    }
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum LinkSourceType {
    Direct,
    Env
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LinkSource {
    source_type: LinkSourceType,
    value: String 
}

pub enum LinkError {
    IOError(std::io::Error),
    VarError(VarError)
}

impl LinkSource {
    pub fn new(source_type: LinkSourceType, value: String) -> Self {
        LinkSource { source_type: source_type, value: value }
    }

    pub fn link_in_dir<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) -> std::io::Result<String> {
        let ll = link.as_ref().join(original.as_ref().file_name().unwrap());

        println!("ll: {:?}", &ll);
        //if link.as_ref().exists() {
        //    fs::remove_dir_all(link.as_ref()).and_then(|_| unix::fs::symlink(original, link))
        //} else {
        unix::fs::symlink(original, &ll)
            .map(|_| String::from(ll.to_str().unwrap()))
        
        //}
    }

    /// link to current working directory
    pub fn link_to<Q: AsRef<Path>>(self, link: Q) -> Result<(String, String), LinkError> {
        match self.source_type {
            LinkSourceType::Direct => Self::link_in_dir(&self.value, &link)
                .map_err(|err| LinkError::IOError(err))
                .map(|l| (self.value, l)),
            LinkSourceType::Env => match env::var(&self.value) {
                Ok(o) => Self::link_in_dir(&o, &link)
                    .map_err(|err| LinkError::IOError(err))
                    .map(|l| (o, l)),
                Err(err) => Err(LinkError::VarError(err)),
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct BuildConfiguration {
    env: Vec<(String, ValueAlternatives)>,
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
    pub fn new(env: Vec<(String, ValueAlternatives)>, sources: Vec<EnvStr>, links: Vec<LinkSource>) -> Self {
        BuildConfiguration { env: env, sources: sources, links: links }
    }
    pub fn into_env<F: Fn(&String) -> bool>(self, predicate: &F, verbose: bool) {
        for src in self.sources.into_iter() {
            let cmd = src.path().unwrap();
            if verbose {
                println!("Dumping env: {:?}", &cmd);
            }
            match dump_environment(&String::from(cmd.as_os_str().to_str().unwrap())) {
                Ok(envmap) => {
                    print::info("Env dumped", String::new());
                    merge_environment(envmap)
                },
                Err(err) => print::fatal("Env dumping error", format!("{:?}", err)),
            }
        }

        for (k, va) in self.env.into_iter() {
            let val = va.setup_env(&k, predicate);

            if verbose {
                match val {
                    Some(v) => match va.action() {
                        VarAction::Set => print::info("Setting env", format!("{}={}", k, v)),
                        VarAction::Append => print::info("Adding to env", format!("{}+={}", k, v)),
                    },
                    None => print::warning("Setting env failed", format!("{} (alternatives: {:?})", k, &va)),
                }
            }
        }
        for l in self.links.into_iter() {
            match l.clone().link_to(env::current_dir().unwrap().as_path()) {
                Ok((s, l)) => print::info("Link created", format!("{:?} -> {}", l ,s)),
                Err(err) => match err {
                    LinkError::IOError(err) => print::warning("Can not create link", format!("{:?}: io error: {}", &l, err)),
                    LinkError::VarError(_) => print::warning("Can not create link", format!("{:?}: env var not present", &l)),
                },
            }
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

    pub fn get_default(self) -> BuildConfiguration { self.default }

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
