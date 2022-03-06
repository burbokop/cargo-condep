
use std::{collections::BTreeMap, path::Path};
use cargo_generate::{BuildConfigProvider, BuildConfiguration, ValueAlternatives, LinkSource, EnvStr, LinkSourceType, LogLevel, CargoConfigFile};



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
            (String::from("LD_LIBRARY_PATH"), ValueAlternatives::one_str("$QT_LIBRARY_PATH", cargo_generate::VarAction::Append)),
            (String::from("PATH"), ValueAlternatives::one_str("$PB_SDK_DIR/usr/bin", cargo_generate::VarAction::Append))
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
        	(String::from("LD_LIBRARY_PATH"), ValueAlternatives::one_str("$QT_LIBRARY_PATH", cargo_generate::VarAction::Append))
    	],
    	vec![],
    	vec![LinkSource::new(LinkSourceType::Env, String::from("PB_SYSTEM_PATH"))]
    ))
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
}

#[derive(clap::Args)]
#[clap(author, version, about, long_about = Some("aaa"))]
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
            GenerateSubCommand::Config(config) => config.exec()
        }     
    }    
}

fn main() {
    let CargoSubCommand::Generate(args) = CargoSubCommand::parse();
    args.exec();
}
