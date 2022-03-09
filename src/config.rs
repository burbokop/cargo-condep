
use std::collections::LinkedList;
use std::convert::identity;
use std::fmt;
use std::{collections::BTreeMap, borrow::Cow};
use std::env::{self, VarError};
use std::os::unix;
use std::path::{Path, PathBuf};
use std::process::{Command};
use regex::{Regex, Captures};
use serde::{Serialize, Deserialize};

pub mod print {
    use std::process::exit;

    use termion::{color::{self, Reset, Fg, LightGreen, LightYellow, LightRed}, style::{self, Bold}};

    pub fn info(header: &str, str: String) {
        println!("{}{}{}{}{} {}", Bold, Fg(LightGreen), header, style::Reset{}, Reset{}.fg_str(), str)
    }
    
    pub fn warning(header: &str, str: String) {
        println!("{}{}{}{}{} {}", Bold, Fg(LightYellow), header, style::Reset{}, color::Reset{}.fg_str(), str)    
    }
    
    pub fn fatal(header: &str, str: String) {
        println!("{}{}{}{}{} {}", Bold, Fg(LightRed), header, style::Reset{}, Reset{}.fg_str(), str);
        exit(-1)
    }    

    pub const ADDING_TO_ENV:       &str = "  Adding env";
    pub const SETTING_TO_ENV:      &str = " Setting env";
    pub const ENV_DUMPED:          &str = "  Env dumped";
    pub const SETTING_ENV_FAILED:  &str = "  Env failed";
    pub const LINK_CREATED:        &str = "Link created";
    pub const CAN_NOT_CREATE_LINK: &str = " Link failed";
    pub const ENV_DUMPING_FAILED:  &str = " Dump failed";                                             
    pub const BAD_STATUS:          &str = "  Bad status";                                             
    
    
}



#[derive(Debug, Clone)]
pub struct EnvStr {
    str: String
}
impl From<String> for EnvStr {
    fn from(s: String) -> Self { EnvStr { str: s } }
}
impl From<&str> for EnvStr {
    fn from(s: &str) -> Self { EnvStr { str: String::from(s) } }
}

impl Serialize for EnvStr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        serializer.serialize_str(self.str.as_str())
    }
}

pub struct EnvStrVisitor {}

impl<'de> serde::de::Visitor<'de> for EnvStrVisitor {
    type Value = EnvStr;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result { write!(formatter, "a string") }
    fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<Self::Value, E> { Ok(s.to_owned().into()) }
}

impl<'de> Deserialize<'de> for EnvStr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> {
        deserializer.deserialize_string(EnvStrVisitor{})
    }
}

    
impl EnvStr {
    /// Converting all $ABC occurances into it's env values
    /// Examples:
    /// ```
    /// std::env::set_var("AAA", "_aaa_");
    /// std::env::set_var("BBB", "_bbb_");
    /// std::env::set_var("CCC", "_ccc_");
    /// use cargo_generate::config::EnvStr;
    /// {
    ///     let value: EnvStr = "$AAA".into();
    ///     assert_eq!(value.to_str(), "_aaa_");
    /// }
    /// {
    ///     let value: EnvStr = "$AAA/some_text/$BBB".into();
    ///     assert_eq!(value.to_str(), "_aaa_/some_text/_bbb_");
    /// }
    /// {
    ///     let value: EnvStr = "$AAA$BBB$CCC".into();
    ///     assert_eq!(value.to_str(), "_aaa__bbb__ccc_");
    /// }
    /// 
    /// 
    /// ```
    pub fn to_str(&self) -> Cow<'_, str> {
        Regex::new(r"\$[_a-zA-Z][_a-zA-Z0-9]*").unwrap().replace_all(&self.str, |caps: &Captures| -> String {
            match env::var(caps[0][1..].to_string()) {
                Ok(res) => res,
                Err(_) => caps[0].to_string(),
            }
        })
    }

    pub fn to_path(&self) -> std::io::Result<PathBuf> { std::fs::canonicalize(self.to_str().into_owned()) }

    pub fn to_string(&self) -> String {
        self.to_str().to_string()
    }
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
    pub fn convert(&self, key: String, value: String) -> (String, String) {
        let a = match self {
            VarAction::Set => (key, value),
            VarAction::Append => {
                match env::var(&key) {
                    Ok(path) => {
                        let mut paths = env::split_paths(&path).collect::<LinkedList<_>>();
                        paths.push_front(PathBuf::from(&value));
                        match env::join_paths(paths) {
                            Ok(new_path) => Ok((key.clone(), String::from(new_path.to_str().unwrap()))),
                            Err(_) => Err(())
                        }
                    },
                    Err(_) => Err(()),
                }.unwrap_or((key, value))
            } 
        };
        env::set_var(&a.0, &a.1);
        a
    }

}

#[derive(Serialize, Deserialize, Debug)]
pub struct ValueAlternatives {
    alternatives: Vec<EnvStr>,
    action: VarAction
}



impl ValueAlternatives {
    pub fn new(alts: Vec<EnvStr>, action: VarAction) -> Self {
        ValueAlternatives { alternatives: alts, action: action }
    }

    pub fn action(&self) -> &VarAction { &self.action }

    pub fn one(alt: EnvStr, action: VarAction) -> Self {
        ValueAlternatives { alternatives: vec![alt], action: action }
    }

    pub fn one_str(alt: &str, action: VarAction) -> Self {
        ValueAlternatives { alternatives: vec![EnvStr::from(alt)], action: action }
    }

    pub fn into_env<F: Fn(&String) -> bool>(self, key: &String, predicate: &F) -> Option<String> {
        self
            .alternatives
            .into_iter().map(|x| x.to_string())
            .find(predicate)
            .map(|v| { self.action.do_action(key, &v); v })
    }

    pub fn setup_env<F: Fn(&String) -> bool>(&self, key: &String, predicate: &F) -> Option<String> {
        self
            .alternatives
            .iter().map(|x| x.to_string())
            .find(predicate)
            .map(|v| { self.action.do_action(key, &v); v })
    }

    pub fn get_env_pair<F: Fn(&String) -> bool>(&self, key: String, predicate: &F) -> Option<(String, String)> {
        self
            .alternatives
            .iter().map(|x| x.to_string())
            .find(predicate)
            .map(|v| self.action.convert(key, v))
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
        let new_link = link.as_ref().join(original.as_ref().file_name().unwrap());

        //if link.as_ref().exists() {
        //    fs::remove_dir_all(link.as_ref()).and_then(|_| unix::fs::symlink(original, link))
        //} else {
        unix::fs::symlink(original, &new_link)
            .map(|_| String::from(new_link.to_str().unwrap()))
        
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

#[derive(Serialize, Deserialize, Debug)]
pub struct EnvPair {
    pub key: String,
    pub value: ValueAlternatives
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BuildConfiguration {
    env: Vec<EnvPair>,
    sources: Vec<EnvStr>,
    soft_links: Vec<LinkSource>,
    linker: Option<EnvStr>,
    link_paths: Vec<EnvStr>
}

#[derive(Debug)]
pub enum CommandError {
    IOError(std::io::Error),
    FromUtf8Error(std::string::FromUtf8Error),
    NotImplemented
}



pub fn rustc_vv() -> std::result::Result<BTreeMap<String, String>, CommandError> {
    Command::new("rustc")
        .arg("-vV")
        .output()
        .map_err(|err| CommandError::IOError(err))
        .and_then(|out| {
            String::from_utf8(out.stdout)
                .map_err(|err| CommandError::FromUtf8Error(err))
                .map(|out| {
                    out
                    .split(|c: char| c == '\n')
                    .skip(1)
                    .map(|line| { 
                        let mut splitn_line = line.splitn(2,  |c: char| c == ':');
                        if let Some(one) = splitn_line.next() {
                            if let Some(two) = splitn_line.next() {
                                Some((one.trim().into(), two.trim().into()))
                            } else { None }
                        } else { None }
                    })
                    .filter_map(identity)
                    .collect()
            })
        })
}

#[cfg(target_os = "linux")]
pub fn dump_environment(bash_file: &String) -> Result<BTreeMap<String, String>, CommandError> {
    Command::new("/bin/bash")
        .arg("-c")
        .arg(format!(". {} && env", bash_file))
        .output()
        .map_err(|err| CommandError::IOError(err))
        .and_then(|r| String::from_utf8(r.stdout)
            .map_err(|err| CommandError::FromUtf8Error(err))
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

pub enum LogLevel {
    Off,
    Pretty,
    Verbose
}

impl From<&str> for LogLevel {
    fn from(s: &str) -> Self {
        match s {
            "off" => LogLevel::Off,
            "pretty" => LogLevel::Pretty,
            "verbose" => LogLevel::Verbose,
            _ => LogLevel::Pretty
        }
    }
}

impl LogLevel {
    pub fn print_pretty(&self) -> bool {
        match self {
            LogLevel::Off => false,
            LogLevel::Pretty => true,
            LogLevel::Verbose => true,
        }
    }
    pub fn print_verbose(&self) -> bool {
        match self {
            LogLevel::Off => false,
            LogLevel::Pretty => false,
            LogLevel::Verbose => true,
        }
    }
}

impl BuildConfiguration {
    pub fn new(env: Vec<EnvPair>, sources: Vec<EnvStr>, soft_links: Vec<LinkSource>, linker: Option<EnvStr>, link_paths: Vec<EnvStr>) -> Self {
        BuildConfiguration { env: env, sources: sources, soft_links: soft_links, linker: linker, link_paths: link_paths }
    }

    pub fn to_env<F: Fn(&String) -> bool>(self, predicate: &F, log_level: LogLevel) -> Vec<(String, String)> {
        for src in self.sources.into_iter() {
            let cmd = src.to_path().unwrap();
            match dump_environment(&String::from(cmd.as_os_str().to_str().unwrap())) {
                Ok(envmap) => {
                    if log_level.print_pretty() {
                        print::info(print::ENV_DUMPED, String::new());
                    }

                    if log_level.print_verbose() {
                        for (k, v) in &envmap {
                            println!(
                                "{}{}{} -> {}{}{}", 
                                termion::color::Bg(termion::color::Magenta), 
                                k, 
                                termion::color::Reset{}.bg_str(),
                                termion::color::Fg(termion::color::Green),
                                v, 
                                termion::color::Reset{}.fg_str()
                            )
                        }
                    }

                    merge_environment(envmap)
                },
                Err(err) => print::fatal(print::ENV_DUMPING_FAILED, format!("{:?}", err)),
            }
        }

        self
            .env
            .into_iter()
            .map(|env_pair|{
                let val = env_pair.value.get_env_pair(env_pair.key.clone(), predicate);

                if log_level.print_pretty() {
                    match &val {
                        Some(v) => match env_pair.value.action() {
                            VarAction::Set => print::info( print::SETTING_TO_ENV, format!("{}={}", v.0, v.1)),
                            VarAction::Append => print::info(print::ADDING_TO_ENV, format!("{}={}", v.0, v.1)),
                        },
                        None => print::warning(print::SETTING_ENV_FAILED, format!("{} (alternatives: {:?})", env_pair.key, &env_pair.value)),
                    }
                }
                val
            })
            .filter_map(identity)
            .collect()
    }

    pub fn make_links(&self) {
        for l in self.soft_links.iter() {
            match l.clone().link_to(env::current_dir().unwrap().as_path()) {
                Ok((s, l)) => print::info(print::LINK_CREATED, format!("{:?} -> {}", l ,s)),
                Err(err) => match err {
                    LinkError::IOError(err) => print::warning(print::CAN_NOT_CREATE_LINK, format!("{:?}: io error: {}", &l, err)),
                    LinkError::VarError(_) => print::warning(print::CAN_NOT_CREATE_LINK, format!("{:?}: env var not present", &l)),
                },
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BuildMultitargetConfig {
    targets: BTreeMap<String, BuildConfiguration>,
    default: BuildConfiguration
}

impl BuildMultitargetConfig {
    pub fn new(targets: BTreeMap<String, BuildConfiguration>, default: BuildConfiguration) -> Self {
        BuildMultitargetConfig { targets: targets, default: default }
    }

    pub fn get_or_default(self, target_triple: &Option<String>) -> Option<BuildConfiguration> {
        match target_triple {
            Some(tt) => { 
                self
                    .targets
                    .into_iter()
                    .find(|(k, _)| k == tt)
                    .map(|(_, v)| v)
            },
            None => Some(self.default),
        }
    }

    pub fn to_config_toml(self, target_triple: &Option<String>, log_level: LogLevel, alias: BTreeMap<String, String>) -> Option<toml::Config> {
        self.get_or_default(target_triple).map(|cunfiguration| {
            cunfiguration.make_links();

            let linker = cunfiguration.linker.clone();
            let link_paths = cunfiguration.link_paths.clone();
    
            let env_pairs = cunfiguration.to_env(&|s: &String| Path::new(s).exists(), log_level);
               
            let links_array: Vec<String> = link_paths
                .into_iter()
                .map(|link| ["-L".into(), link.to_string()]).flatten()
                .collect();

            match target_triple {
                Some(tgt) => toml::Config {
                    alias: alias,
                    build: toml::Build::with_target(tgt.clone(), links_array),             
                    target: {
                        let mut table = ::toml::map::Map::new();
                        if let Some(linker) = linker {
                            table.insert(toml::Config::LINKER.into(), linker.to_string().into());
                        }
                        BTreeMap::from([(tgt.clone(), table)])
                    },
                    env: BTreeMap::from_iter(env_pairs),
                },
                None => toml::Config {
                    alias: alias,
                    build: toml::Build::empty_target(links_array),
                    target: {
                        let current_target = rustc_vv()
                            .unwrap()
                            .into_iter()
                            .find(|(k, _)| k == "host")
                            .unwrap()
                            .1;

                        let mut table = ::toml::map::Map::new();
                        table.insert(toml::Config::RUNNER.into(), "cargo condep run".into());
                        BTreeMap::from([(current_target, table)])
                    },
                    env: BTreeMap::from_iter(env_pairs),
                },
            }
        })
    }

}




pub mod toml {
    use std::collections::BTreeMap;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Build {
        pub jobs: usize,
        pub target: Option<String>,
        pub rustflags: Vec<String>
    }

    impl Build {
        pub fn with_target(target: String, rustflags: Vec<String>) -> Self {
            Build { jobs: num_cpus::get(), target: Some(target), rustflags: rustflags }
        }
        pub fn empty_target(rustflags: Vec<String>) -> Self {
            Build { jobs: num_cpus::get(), target: None, rustflags: rustflags }
        }
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Config {
        pub alias: BTreeMap<String, String>,
        pub build: Build,
        pub env: BTreeMap<String, String>,
        pub target: BTreeMap<String, toml::value::Table>
    }

    impl Config {
        pub const RUNNER: &'static str = "runner";
        pub const LINKER: &'static str = "linker";
        pub const RUSTC_LINK_SEARCH: &'static str = "rustc-link-search";

        pub fn target_mono(target: String, key: String, value: toml::Value) -> BTreeMap<String, toml::value::Table> {
            let mut table = toml::map::Map::new();
            table.insert(key, value);
            BTreeMap::from([(target, table)])
        }

        /// returns old value
        pub fn set_target_val(&mut self, target: String, key: String, value: toml::Value) -> Option<toml::Value> {
            match self
                    .target
                    .entry(target)
                    .or_insert(toml::value::Table::new())
                    .entry(key) {
                        toml::map::Entry::Vacant(v) => { v.insert(value); None },
                        toml::map::Entry::Occupied(o) => { let mut oo = o; Some(oo.insert(value)) },
                    }
        }
        pub fn target_val(&self, target: &str, key: &str) -> Option<&str> {
            self
                .target
                .get(target)
                .and_then(|table| table.get(key))
                .and_then(|v| v.as_str())
        }
    } 

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Package {
        pub name: String,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Cargo {
        pub package: Package,
    }
}




