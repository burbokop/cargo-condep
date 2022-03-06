
use std::{collections::BTreeMap, path::{Path, PathBuf}};
use cargo_generate::{config::{BuildConfigProvider, BuildConfiguration, ValueAlternatives, LinkSource, EnvStr, LinkSourceType, LogLevel, CargoConfigFile, VarAction}, deploy::{DeployConfig, self, Noop, DeploySource}, ssh_deploy::SSHDeploy};



use clap::Parser;



fn pb_default_config() -> BuildConfigProvider {
    BuildConfigProvider::new(BTreeMap::from([
        (String::from("armv7-unknown-linux-gnueabi"), BuildConfiguration::new(
        vec![
            (String::from("CC"), ValueAlternatives::from("$PB_SDK_DIR/usr/bin/arm-obreey-linux-gnueabi-gcc")),
            (String::from("CXX"), ValueAlternatives::from("$PB_SDK_DIR/usr/bin/arm-obreey-linux-gnueabi-g++")),
            (String::from("QMAKE"), ValueAlternatives::from("$TOOLCHAIN_PATH/$TOOLCHAIN_PREFIX/sysroot/ebrmain/bin/qmake")),
            (String::from("QT_INCLUDE_PATH"), ValueAlternatives::from("$TOOLCHAIN_PATH/$TOOLCHAIN_PREFIX/sysroot/ebrmain/include")),
            (String::from("QT_LIBRARY_PATH"), ValueAlternatives::from("$TOOLCHAIN_PATH/$TOOLCHAIN_PREFIX/sysroot/ebrmain/lib")),
            (String::from("LD_LIBRARY_PATH"), ValueAlternatives::one_str("$QT_LIBRARY_PATH", VarAction::Append)),
            (String::from("PATH"), ValueAlternatives::one_str("$PB_SDK_DIR/usr/bin", VarAction::Append))
            ],
        vec![EnvStr::from("$PB_SDK_DIR/../env_set.sh")],
        vec![LinkSource::new(LinkSourceType::Env, String::from("PB_SYSTEM_PATH"))]
        )),
    ]),
	BuildConfiguration::new(
        vec![
        	(String::from("QMAKE"), ValueAlternatives::from("$PB_SDK_DIR/local/qt5/bin/qmake")),
        	(String::from("QT_INCLUDE_PATH"), ValueAlternatives::from("$PB_SDK_DIR/local/qt5/include")),
        	(String::from("QT_LIBRARY_PATH"), ValueAlternatives::from("$PB_SDK_DIR/local/qt5/lib")),
        	(String::from("LD_LIBRARY_PATH"), ValueAlternatives::one_str("$QT_LIBRARY_PATH", VarAction::Append))
    	],
    	vec![],
    	vec![LinkSource::new(LinkSourceType::Env, String::from("PB_SYSTEM_PATH"))]
    ))
}


fn pb_default_deploy_config() -> DeployConfig {
    DeployConfig {
        execs_path: PathBuf::from("/ebrmain/bin"),
        libs_path: PathBuf::from("/ebrmain/lib"),
        config_path: PathBuf::from("/ebrmain/config"),
        user_path: PathBuf::from("/mnt/ext1/settings")
    }
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
    Generate(Generate),
}

#[derive(clap::Subcommand)]
enum GenerateSubCommand {
    Config(Config),
    Deploy(Deploy)
}

#[derive(clap::Args)]
#[clap(author, version, about, long_about = Some("Generate configuration for specific target"))]
struct Config {
    #[clap(long, parse(from_str))]
    target: Option<String>,

    #[clap(long, parse(from_str), default_value = "pretty")]
    log_level: LogLevel,
}

impl Config {
    fn exec(self) {
        let conf_provider = pb_default_config();

        match conf_provider.get_or_default(&self.target) {            
            Some(c) => {
                let env_pairs = c.into_env(&|s: &String| Path::new(s).exists(), self.log_level);

                
                (
                    self.target.map(|tt| CargoConfigFile::from_build_options(&tt) 
                        + env_pairs.iter().find(|(k, _)| k == "CXX")
                            .map(|(_, linker)| CargoConfigFile::from_target_options(&tt, linker))
                            .unwrap_or(CargoConfigFile::empty()))
                        .unwrap_or(CargoConfigFile::empty()) 
                    + CargoConfigFile::from_env_pairs(&env_pairs)
                ).save(".cargo/config.toml").unwrap();
            },
            None => println!("undefined target"),
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
    fn depl(&self) -> Box<dyn DeployAndCallRemote> {
        match self {
            DeployMethod::SSH => Box::new(SSHDeploy::connect("192.168.205.1", "root").unwrap()),
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
        let conf = pb_default_deploy_config();
        let mut depl = self.method.depl();

        let target = CargoConfigFile::read(".cargo/config.toml").unwrap().parse_default_target().unwrap();
        let exe_name = CargoConfigFile::read("Cargo.toml").unwrap().parse_name().unwrap();


        let src = DeploySource {
            execs: vec![std::env::current_dir().unwrap().join(PathBuf::from("target")).join(target).join(exe_name)],
            libs: vec![],
            config_files: vec![],
            user_files: vec![],
        };

        depl.call_remote(b"mount -o rw,remount /ebrmain").unwrap();

        depl.deploy(src, conf).unwrap()
    }
}

#[derive(clap::Args)]
#[clap(author, version, about, long_about = None)]
struct Generate {
    #[clap(long, parse(from_os_str))]
    manifest_path: Option<std::path::PathBuf>,

    #[clap(subcommand)]
    sub: GenerateSubCommand

}

impl Generate {
    fn exec(self) {
        match self.sub {
            GenerateSubCommand::Config(config) => config.exec(),
            GenerateSubCommand::Deploy(deploy) => deploy.exec()
        }     
    }    
}

fn main() {
    let CargoSubCommand::Generate(args) = CargoSubCommand::parse();
    args.exec();
}
