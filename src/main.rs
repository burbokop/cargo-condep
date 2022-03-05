
use std::{collections::BTreeMap, env, path::Path};
use cargo_generate::{BuildConfigProvider, BuildConfiguration, ValueAlternatives, LinkSource, EnvStr, LinkSourceType};



use clap::{Parser, Subcommand};
use termion::color;



fn pb_default_config() -> BuildConfigProvider {
    BuildConfigProvider::new(BTreeMap::from([
        (String::from("armv7-unknown-linux-gnueabi"), BuildConfiguration::new(
        BTreeMap::from([
            (String::from("CC"), ValueAlternatives::from("$PB_SDK_DIR/usr/bin/arm-obreey-linux-gnueabi-gcc")),
            (String::from("CXX"), ValueAlternatives::from("$PB_SDK_DIR/usr/bin/arm-obreey-linux-gnueabi-g++")),
            (String::from("QMAKE"), ValueAlternatives::from("$TOOLCHAIN_PATH/$TOOLCHAIN_PREFIX/sysroot/ebrmain/bin/qmake")),
            (String::from("QT_INCLUDE_PATH"), ValueAlternatives::from("$TOOLCHAIN_PATH/$TOOLCHAIN_PREFIX/sysroot/ebrmain/include")),
            (String::from("QT_LIBRARY_PATH"), ValueAlternatives::from("$TOOLCHAIN_PATH/$TOOLCHAIN_PREFIX/sysroot/ebrmain/lib")),
            (String::from("LD_LIBRARY_PATH"), ValueAlternatives::from("$QT_LIBRARY_PATH:$LD_LIBRARY_PATH"))
            ]),
        vec![EnvStr::from("$PB_SDK_DIR/../env_set.sh")],
        vec![LinkSource::new(LinkSourceType::Env, String::from("PB_SYSTEM_PATH"))]
        )),
    ]),
	BuildConfiguration::new(
    	BTreeMap::from([
        	(String::from("QMAKE"), ValueAlternatives::from("$PB_SDK_DIR/local/qt5/bin/qmake")),
        	(String::from("QT_INCLUDE_PATH"), ValueAlternatives::from("$PB_SDK_DIR/local/qt5/include")),
        	(String::from("QT_LIBRARY_PATH"), ValueAlternatives::from("$PB_SDK_DIR/local/qt5/lib")),
        	(String::from("LD_LIBRARY_PATH"), ValueAlternatives::from("$QT_LIBRARY_PATH:$LD_LIBRARY_PATH"))
    	]),
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

    #[clap(long)]
    hide_log: bool
}

impl Config {
    fn exec(self) {
        println!("self.verbose: {}", !&self.hide_log);
        let cfg = pb_default_config();
        match self.target {            
            Some(t) => cfg.get(&t),
            None => cfg.get_default(),
        }.into_env(&|s: &String| Path::new(s).exists(), !self.hide_log);
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
