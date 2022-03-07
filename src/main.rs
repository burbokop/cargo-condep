
use std::{collections::BTreeMap, path::{Path, PathBuf}};
use cargo_generate::{config::{BuildConfigProvider, BuildConfiguration, ValueAlternatives, LinkSource, EnvStr, LinkSourceType, LogLevel, VarAction, self}, deploy::{DeployConfig, self, Noop, DeployPaths}, ssh_deploy::SSHDeploy};

use clap::Parser;
use termion::color::{Fg, Blue, Reset, LightBlue};

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
        user_path: PathBuf::from("/mnt/ext1/system")
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
    SomeAction(SomeAction)
}


#[derive(clap::Args)]
#[clap(author, version, about, long_about = Some("Generate configuration for specific target"))]
struct SomeAction {
    #[clap(long, parse(from_str), default_value = "pretty")]
    log_level: LogLevel,
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


                let dst_config = match self.target {
                    Some(tgt) => config::toml::Config {
                        build: config::toml::Build::from_target(tgt.clone()),             
                        target: env_pairs
                            .iter()
                            .find(|(k, _)| k == "CXX")
                            .map(|(_, v)| config::toml::Config::target_mono(tgt, config::toml::Config::LINKER.into(), v.clone().into()))
                            .unwrap_or(BTreeMap::new()),
                        env: BTreeMap::from_iter(env_pairs),
                    },
                    None => config::toml::Config {
                        build: config::toml::Build::empty_target(),
                        target: BTreeMap::new(),
                        env: BTreeMap::from_iter(env_pairs)
                    },
                };
                
                std::fs::write(".cargo/config.toml", toml::to_string_pretty(&dst_config).unwrap()).unwrap();
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

        let cwd = std::env::current_dir().unwrap();

        let config_toml: config::toml::Config = toml::from_slice(std::fs::read(cwd.join(".cargo/config.toml")).unwrap().as_slice()).unwrap();
        let cargo_toml: config::toml::Cargo = toml::from_slice(std::fs::read(cwd.join("Cargo.toml")).unwrap().as_slice()).unwrap();

        let target_dir = cwd.join(PathBuf::from("target"));
        let exe = match config_toml.build.target {
            Some(tgt) => target_dir.join(tgt),
            None => target_dir,
        }
            .join("release")
            .join(cargo_toml.package.name);

        let src = DeployPaths {
            execs: vec![exe],
            libs: vec![],
            config_files: vec![],
            user_files: vec![],
        };

        println!("src: {:#?}", src);

        depl.call_remote(b"mount -o rw,remount /ebrmain").unwrap();

        let dst = depl.deploy(src, conf).unwrap();

        for exe in dst.execs {
            depl.call_remote(format!("chmod +x {:?}", exe).as_bytes()).unwrap();
        }
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






    match CargoSubCommand::parse() {
        CargoSubCommand::Generate(cmd) => cmd.exec(),
        CargoSubCommand::SomeAction(_) => {
            println!("some action");


            let tml = r#"
            
            [build]

            target = "armv7-unknown-linux-gnueabi"
            jobs = 6
            
            [target.armv7-unknown-linux-gnueabi]
            
            linker = "/home/ivan/workspace/SDK-B288/usr/bin/arm-obreey-linux-gnueabi-g++"
            
            [env]
            
            CC = "/home/ivan/workspace/SDK-B288/usr/bin/arm-obreey-linux-gnueabi-gcc"
            CXX = "/home/ivan/workspace/SDK-B288/usr/bin/arm-obreey-linux-gnueabi-g++"
            QT_INCLUDE_PATH = "/home/ivan/workspace/SDK-B288/usr/arm-obreey-linux-gnueabi/sysroot/ebrmain/include"
            QT_LIBRARY_PATH = "/home/ivan/workspace/SDK-B288/usr/arm-obreey-linux-gnueabi/sysroot/ebrmain/lib"
            
            "#;


            let doc3: toml::Value = tml.parse().unwrap();

            let mut doc: cargo_generate::config::toml::Config = toml::from_str(tml).unwrap();

            doc.set_target_val("armv7-unknown-linux-gnueabi".into(), config::toml::Config::LINKER.into(), "gogadoda".into());
            doc.set_target_val("armv5-unknown-linux-gnueabi".into(), config::toml::Config::LINKER.into(), "gogadoda_v5".into());

            println!("doc2: {}{:#?}{}", Fg(Blue), doc, Reset{}.fg_str());

            println!("{}{}{}", Fg(LightBlue), toml::to_string_pretty(&doc).unwrap(), Reset{}.fg_str());


            println!("doc3: {:#?}", doc3);

        },
    }
    
}
