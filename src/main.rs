
use std::{collections::BTreeMap, env, path::Path};
use cargo_generate::{BuildConfigProvider, BuildConfiguration, ValueAlternatives, LinkSource, EnvStr, LinkSourceType};



use clap::{Parser, Subcommand};
use termion::color;

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
#[clap(author, version, about, long_about = None)]
struct Config {
    #[clap(long, parse(from_str))]
    target: Option<String>

}

impl Config {
    fn exec(self) {
        println!("generate config: target={:?}", self.target);
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



    //let args = Args::parse();
/*
    if args.cmd == "s" {
        println!("Hello {} {}!", args.cmd, args.target);

        //env_perm::set("GG", "AAAAAAA").unwrap();

        return;
    }
*/

    for argument in env::args() {
        println!("arg: {}", argument);
    }

    println!("cargo:warning=$PB_SDK_DIR/usr/bin/$PB_SUSTEM_PATH/config -> {}", EnvStr::from("$PB_SDK_DIR/usr/bin/$PB_SYSTEM_PATH/config").str().into_owned());


    let cfg = BuildConfigProvider::new(BTreeMap::from([
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
            vec![LinkSource::new(LinkSourceType::Env, String::from("$PB_SYSTEM_PATH"))]
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
        vec![LinkSource::new(LinkSourceType::Env, String::from("$PB_SYSTEM_PATH"))]
    ));

    /*

    if env::args().len() > 2 {
        let args: Vec<_> = env::args().collect();


        cfg
        .get(&args[2])
        .into_env(&|s: &String| Path::new(s).exists());

    } else {
        cfg
        .get_from_env(&String::from("TARGET"))
        .into_env(&|s: &String| Path::new(s).exists());

    }
*/

    //let o = cargo_find_target::dump_environment(&String::from("/home/ivan/workspace/projects/conf_plugin/some.sh")).unwrap();


    //for (k, v) in o {
    //    println!("{}env var: {} -> {}{}{}", color::Fg(color::Magenta), k, color::Fg(color::Yellow), v, color::Reset{}.fg_str())
    //}

    let cc = env::var("SSS").unwrap();

    let out_dir = env::var_os("OUT_DIR").unwrap();
    
    println!("gogadoda3");
    println!("cargo:warning={} {}", "output dir:", out_dir.into_string().unwrap());
    println!("cargo:warning={} {}", "SSS:", cc);

    
    println!("cargo:rerun-if-changed=build.rs");
}
