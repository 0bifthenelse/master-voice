use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const MA_INPUTS: &[&str] = &[
    "ma/master.ma",
    "ma/constants.ma",
    "ma/tables.ma",
    "ma/measure.ma",
    "ma/render.ma",
    "ma/source.ma",
    "ma/filters.ma",
    "ma/character.ma",
    "ma/math.ma",
];

fn run(tool: &str, args: &[&str], cwd: &Path) -> Output {
    Command::new(tool)
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap_or_else(|error| panic!("failed to execute GNU {tool}: {error}"))
}

fn require_success(tool: &str, output: Output) {
    if output.status.success() {
        return;
    }
    panic!(
        "GNU {tool} failed with {}:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
}

fn main() {
    let os = env::var("CARGO_CFG_TARGET_OS").expect("CARGO_CFG_TARGET_OS is missing");
    let arch = env::var("CARGO_CFG_TARGET_ARCH").expect("CARGO_CFG_TARGET_ARCH is missing");
    if os != "linux" || arch != "x86_64" {
        panic!("master-voice-synth requires Linux x86-64; requested target is {arch}-{os}");
    }

    println!("cargo:rerun-if-env-changed=AS");
    println!("cargo:rerun-if-env-changed=AR");
    for input in MA_INPUTS {
        println!("cargo:rerun-if-changed={input}");
    }

    let manifest =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is missing"));
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is missing"));
    let object = out_dir.join("master_voice_ma.o");
    let archive = out_dir.join("libmaster_voice_ma.a");
    let assembler = env::var("AS").unwrap_or_else(|_| "as".to_owned());
    let archiver = env::var("AR").unwrap_or_else(|_| "ar".to_owned());

    require_success(
        &assembler,
        run(
            &assembler,
            &[
                "--64",
                "-o",
                object.to_str().expect("non-UTF-8 object path"),
                "ma/master.ma",
            ],
            &manifest,
        ),
    );
    require_success(
        &archiver,
        run(
            &archiver,
            &[
                "crs",
                archive.to_str().expect("non-UTF-8 archive path"),
                object.to_str().expect("non-UTF-8 object path"),
            ],
            &manifest,
        ),
    );

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=master_voice_ma");
}
