
use std::{collections::BTreeMap, env, path::Path};
use cargo_find_target::{BuildConfigProvider, BuildConfiguration, ValueAlternatives, LinkSource, EnvStr, LinkSourceType};


fn main() {

    //"TARGET"

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



    let cc = env::var("CC").unwrap();

    let out_dir = env::var_os("OUT_DIR").unwrap();
    
    println!("gogadoda3");
    println!("cargo:warning={} {}", "output dir:", out_dir.into_string().unwrap());
    println!("cargo:warning={} {}", "CC:", cc);

    
    println!("cargo:rerun-if-changed=build.rs");
}
