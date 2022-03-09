
use std::{collections::BTreeMap, path::{PathBuf, Path}, string::FromUtf8Error};
use cargo_condep::{config::{BuildMultitargetConfig, BuildConfiguration, ValueAlternatives, LinkSource, EnvStr, LinkSourceType, LogLevel, VarAction, self, EnvPair}, deploy::{DeployConfig, self, Noop, DeployPaths}, ssh_deploy::{SSHDeploy, SSHUserAndHost}};



use clap::Parser;
use termion::color::{Fg, Reset, LightYellow, LightMagenta};


#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct SSHDeployConfig {
    pub paths: DeployConfig,
    pub ssh: SSHUserAndHost
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct WholeConfig {
    pub config: BuildMultitargetConfig,
    pub deploy: SSHDeployConfig
}

struct ConfigProvider {
    pub cache_path: PathBuf
}

#[derive(Debug)]
enum ConfigReadError {
    IOError(std::io::Error),
    FromUtf8Error(FromUtf8Error),
    YamlError(serde_yaml::Error)
}

#[derive(Debug)]
enum ConfigWriteError {
    IOError(std::io::Error),
    YamlError(serde_yaml::Error)
}

impl Default for ConfigProvider {
    fn default() -> Self { 
        ConfigProvider { 
            cache_path: dirs::home_dir()
                .unwrap()
                .join(".cargo")
                .join("condep") 
        } 
    }
}

impl ConfigProvider {
    pub fn install_from_bytes(&self, bytes: &[u8]) -> std::io::Result<()> {
        std::fs::create_dir_all(self.cache_path.as_path())
            .and_then(|()| std::fs::write(self.cache_path.join("config.yaml"), bytes))
    }

    pub fn install_from_path(&self, path: &Path) -> std::io::Result<()> {
        std::fs::read(path)
            .and_then(|bytes| self.install_from_bytes(bytes.as_slice()))
    }

    pub fn install_from_row_data(&self, row_data: &WholeConfig) -> Result<(), ConfigWriteError> {
        serde_yaml::to_string(row_data)
            .map_err(ConfigWriteError::YamlError)
            .and_then(|str| self.install_from_bytes(str.as_bytes())
                .map_err(ConfigWriteError::IOError)
            )
    }

    pub fn read(&self) -> Result<WholeConfig, ConfigReadError> {
        std::fs::read(self.cache_path.join("config.yaml"))
            .map_err(ConfigReadError::IOError)
            .and_then(|bytes| String::from_utf8(bytes)
            .map_err(ConfigReadError::FromUtf8Error)
                .and_then(|str| serde_yaml::from_str(str.as_str())
                    .map_err(ConfigReadError::YamlError)
                )
            )
    }
}



fn pb_default_config() -> BuildMultitargetConfig {
    BuildMultitargetConfig::new(BTreeMap::from([
        (String::from("armv7-unknown-linux-gnueabi"), BuildConfiguration::new(
        vec![
            EnvPair { key: "CC".into(), value: ValueAlternatives::from("$PB_SDK_DIR/usr/bin/arm-obreey-linux-gnueabi-gcc") },
            EnvPair { key: "CXX".into(), value: ValueAlternatives::from("$PB_SDK_DIR/usr/bin/arm-obreey-linux-gnueabi-g++") },
            EnvPair { key: "QMAKE".into(), value: ValueAlternatives::from("$TOOLCHAIN_PATH/$TOOLCHAIN_PREFIX/sysroot/ebrmain/bin/qmake") },
            EnvPair { key: "QT_INCLUDE_PATH".into(), value: ValueAlternatives::from("$TOOLCHAIN_PATH/$TOOLCHAIN_PREFIX/sysroot/ebrmain/include") },
            EnvPair { key: "QT_LIBRARY_PATH".into(), value: ValueAlternatives::from("$TOOLCHAIN_PATH/$TOOLCHAIN_PREFIX/sysroot/ebrmain/lib") },
            EnvPair { key: "LD_LIBRARY_PATH".into(), value: ValueAlternatives::one_str("$QT_LIBRARY_PATH", VarAction::Append) },
            EnvPair { key: "PATH".into(), value: ValueAlternatives::one_str("$PB_SDK_DIR/usr/bin", VarAction::Append) }
            ],
        vec![EnvStr::from("$PB_SDK_DIR/../env_set.sh")],
        vec![LinkSource::new(LinkSourceType::Env, String::from("PB_SYSTEM_PATH"))],
        Some("$PB_SDK_DIR/usr/bin/arm-obreey-linux-gnueabi-g++".into()),
        vec!["$TOOLCHAIN_PATH/$TOOLCHAIN_PREFIX/sysroot/usr/local/lib".into()]
        )),
    ]),
	BuildConfiguration::new(
        vec![
        	EnvPair { key: "QMAKE".into(), value: ValueAlternatives::from("$PB_SDK_DIR/local/qt5/bin/qmake") },
        	EnvPair { key: "QT_INCLUDE_PATH".into(), value: ValueAlternatives::from("$PB_SDK_DIR/local/qt5/include") },
        	EnvPair { key: "QT_LIBRARY_PATH".into(), value: ValueAlternatives::from("$PB_SDK_DIR/local/qt5/lib") },
        	EnvPair { key: "LD_LIBRARY_PATH".into(), value: ValueAlternatives::one_str("$PB_SDK_DIR/usr/local/lib", VarAction::Append) },
            EnvPair { key: "LD_LIBRARY_PATH".into(), value: ValueAlternatives::one_str("$QT_LIBRARY_PATH", VarAction::Append) }
        ],
    	vec![],
    	vec![LinkSource::new(LinkSourceType::Env, String::from("PB_SYSTEM_PATH"))],
        None,
        vec!["$PB_SDK_DIR/usr/local/lib".into()]
    ))
}

fn pb_default_deploy_ssh_user_host() -> SSHUserAndHost {
    SSHUserAndHost {
        user: "root".into(),
        host: "192.168.205.1".into(),
    }
}

fn pb_default_deploy_config() -> DeployConfig {
    DeployConfig {
        execs_path: PathBuf::from("/ebrmain/bin"),
        libs_path: PathBuf::from("/ebrmain/lib"),
        config_path: PathBuf::from("/ebrmain/config"),
        user_path: PathBuf::from("/mnt/ext1/system")
    }
}

fn pb_whole_config() -> WholeConfig {
    let c = pb_default_config();
    let d = pb_default_deploy_config();
    let ssh = pb_default_deploy_ssh_user_host();
    
    WholeConfig { config: c, deploy: SSHDeployConfig{ paths: d, ssh: ssh } }
}


#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    cmd: String,

    #[clap(short, long)]
    target: String,
}



#[derive(Parser)]
#[clap(name = "cargo")]
#[clap(bin_name = "cargo")]
enum CargoSubCommand {
    Condep(Condep),
    SomeAction(SomeAction)
}


#[derive(clap::Args)]
#[clap(author, version, about, long_about = Some("Generate configuration for specific target"))]
struct SomeAction {
    #[clap(long, parse(from_str), default_value = "pretty")]
    log_level: LogLevel,
}

#[derive(clap::Subcommand)]
enum CondepSubCommand {
    Configure(Configure),
    Run(Run),
    Deploy(Deploy),
    Install(Install)
}

#[derive(clap::Args)]
#[clap(author, version, about, long_about = Some("Generate configuration for specific target"))]
struct Configure {
    #[clap(long, parse(from_str))]
    target: Option<String>,

    #[clap(long, parse(from_str), default_value = "pretty")]
    log_level: LogLevel,
}

impl Configure {
    fn exec(self) {
        let alias  = [("deploy".into(), "condep deploy".into())].into();
        match ConfigProvider::default().read() {
            Ok(config) => {
                match config.config.to_config_toml(&self.target, self.log_level, alias) {
                    Some(tml) => {
                        std::fs::create_dir_all(".cargo").unwrap();
                        std::fs::write(".cargo/config.toml", toml::to_string_pretty(&tml).unwrap()).unwrap();
                    },
                    None => println!("undefined target"),
                }
            },
            Err(err) => match err {
                ConfigReadError::IOError(_) => println!("Config not installed: use `cargo condep install`"),
                _ => println!("Config installed but broken: use `cargo condep install` to reinstall it"),
            },
        }
    }
}

#[derive(clap::Args, Debug)]
struct RunDelegate {
    pub exe: String,
    pub args: Vec<String>
}

#[derive(clap::Args)]
#[clap(author, version, about, long_about = Some("Run with configured LD_LIBRARY_PATH"))]
struct Run {
    #[clap(flatten)]
    delegate: RunDelegate,
}

impl Run {
    fn exec(self) {

        let cwd = std::env::current_dir().unwrap();
        let config_toml: config::toml::Config = toml::from_slice(std::fs::read(cwd.join(".cargo/config.toml")).unwrap().as_slice()).unwrap();

        let ldlp_key = "LD_LIBRARY_PATH";
        config_toml.env.get(ldlp_key).map(|ldlp| {
            println!("setting {} = {}", ldlp_key, ldlp);
            std::env::set_var(ldlp_key, ldlp);
        });
        println!("running: {:?} {:?}", cwd.join(&self.delegate.exe), self.delegate.args);

        if !self.delegate.exe.is_empty() {  

            let mut child = std::process::Command::new(&cwd.join(self.delegate.exe))
                .args(self.delegate.args)
                .spawn()
                .unwrap();

            let exit_status = child
                .wait()
                .unwrap();

            if !exit_status.success() {
                println!("bad status: {}", exit_status)
            }
        } else {
            panic!("exe is empty")
        }
    }
}

enum DeployMethod {
    SSH,
    No
}

trait DeployAndCallRemote: deploy::Deploy + deploy::CallRemote {}

impl<T> DeployAndCallRemote for T
where
    T: deploy::Deploy + deploy::CallRemote {}


impl DeployMethod {
    fn depl(&self, user_host: &SSHUserAndHost) -> Box<dyn DeployAndCallRemote> {
        match self {
            DeployMethod::SSH => Box::new(SSHDeploy::connect(user_host).unwrap()),
            DeployMethod::No => Box::new(Noop{}),
        }
    }
}

impl From<&str> for DeployMethod {
    fn from(s: &str) -> Self {
        match s {
            "ssh" => Self::SSH,
            _ => Self::No
        }
    }
}

#[derive(clap::Args)]
#[clap(author, version, about, long_about = Some("Deploy current turget"))]
struct Deploy {
    
    #[clap(long, parse(from_str), default_value = "pretty")]
    log_level: LogLevel,
    #[clap(long, parse(from_str), default_value = "ssh")]
    method: DeployMethod
}

impl Deploy {
    fn exec(self) {

        match ConfigProvider::default().read() {
            Ok(config) => {
                let mut depl = self.method.depl(&config.deploy.ssh);
        
                let cwd = std::env::current_dir().unwrap();
        
                let config_toml: config::toml::Config = toml::from_slice(std::fs::read(cwd.join(".cargo/config.toml")).unwrap().as_slice()).unwrap();
                let cargo_toml: config::toml::Cargo = toml::from_slice(std::fs::read(cwd.join("Cargo.toml")).unwrap().as_slice()).unwrap();
        
                let target_dir = cwd.join(PathBuf::from("target"));
                let current_target_dir = match config_toml.build.target {
                    Some(tgt) => target_dir.join(tgt),
                    None => target_dir,
                };
        
                let exe = { 
                    let release = current_target_dir
                        .join("release")
                        .join(&cargo_toml.package.name);
                    if !release.exists() {
                        let debug = current_target_dir
                            .join("debug")
                            .join(cargo_toml.package.name);
                        if !debug.exists() {
                            panic!("neither release nor debug exist. may do `cargo build`")
                        } else {
                            debug
                        }
                    } else {
                        release
                    }
                };
        
                let src = DeployPaths {
                    execs: vec![exe],
                    libs: vec![],
                    config_files: vec![],
                    user_files: vec![],
                };
        
                println!("src: {:#?}", src);
        
                depl.call_remote(b"mount -o rw,remount /ebrmain").unwrap();
        
                let dst = depl.deploy(src, config.deploy.paths).unwrap();
        
                for exe in dst.execs {
                    depl.call_remote(format!("chmod +x {:?}", exe).as_bytes()).unwrap();
                }        
            },
            Err(err) => panic!("can not read config {:?}", err)
        }
    }
}


#[derive(clap::Args)]
#[clap(author, version, about, long_about = "Install configuration file")]
struct Install {
    #[clap(long, parse(from_str))]
    file: Option<PathBuf>,
    #[clap(long, parse(from_str))]
    url: Option<String>,
    #[clap(long)]
    hardcode: bool,
}

impl Install {
    fn exec(self) {
        let cfg_provider = ConfigProvider::default();
        if let Some(file) = self.file {            
            println!("installing from path: {:?}", file.as_path());
            cfg_provider.install_from_path(file.as_path())
                .unwrap()
        } else if let Some(_) = self.url {
            panic!("url installation not implemented yet")
        } else if self.hardcode {
            println!("installing from hardcode");
            cfg_provider.install_from_row_data(&pb_whole_config()).unwrap()
        } else {
            panic!("specify --file='some/path' or --url='https://some.url'")
        }
    }
}

#[derive(clap::Args)]
#[clap(author, version, about, long_about = None)]
struct Condep {
    #[clap(long, parse(from_os_str))]
    manifest_path: Option<std::path::PathBuf>,

    #[clap(subcommand)]
    sub: CondepSubCommand

}

impl Condep {
    fn exec(self) {
        match self.sub {
            CondepSubCommand::Configure(cmd) => cmd.exec(),
            CondepSubCommand::Run(cmd) => cmd.exec(),
            CondepSubCommand::Deploy(cmd) => cmd.exec(),
            CondepSubCommand::Install(cmd) => cmd.exec()
        }     
    }    
}


fn main() {
    match CargoSubCommand::parse() {
        CargoSubCommand::Condep(cmd) => cmd.exec(),
        CargoSubCommand::SomeAction(_) => {

            

            let c = pb_default_config();
            let d = pb_default_deploy_config();
            let ssh = pb_default_deploy_ssh_user_host();
            
            

            //let doc3: toml::Value = tml.parse().unwrap();

            //let mut doc: cargo_condep::config::toml::Config = toml::from_str(tml).unwrap();

            //doc.set_target_val("armv7-unknown-linux-gnueabi".into(), config::toml::Config::LINKER.into(), "gogadoda".into());
            //doc.set_target_val("armv5-unknown-linux-gnueabi".into(), config::toml::Config::LINKER.into(), "gogadoda_v5".into());

            //println!("doc2: {}{:#?}{}", Fg(Blue), doc, Reset{}.fg_str());
            

            let c_yaml = serde_yaml::to_string(&WholeConfig { config: c, deploy: SSHDeployConfig{ paths: d, ssh: ssh } }).unwrap();

            println!("{}{}{}", Fg(LightYellow), c_yaml, Reset{}.fg_str());

            println!("{}{:#?}{}", Fg(LightMagenta), serde_yaml::from_str::<WholeConfig>(c_yaml.as_str()).unwrap(), Reset{}.fg_str());

            
        },
    }
    
}
