#![allow(dead_code)]

use std::path::PathBuf;
use std::process::{Command, Output};

pub struct TestEnv {
    pub runtime_dir: PathBuf,
}

impl TestEnv {
    pub fn new() -> Self {
        let runtime_dir = std::env::temp_dir().join(format!(
            "mv-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&runtime_dir).unwrap();
        let config_dir = runtime_dir.join("config").join("master-voice");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(
            config_dir.join("config.toml"),
            "daemon_idle_timeout_secs = 2\n",
        )
        .unwrap();
        Self { runtime_dir }
    }

    pub fn run(&self, args: &[&str]) -> Output {
        let bin = env!("CARGO_BIN_EXE_master-voice");
        Command::new(bin)
            .args(args)
            .env("XDG_RUNTIME_DIR", &self.runtime_dir)
            .env("XDG_CONFIG_HOME", self.runtime_dir.join("config"))
            .env("XDG_CACHE_HOME", self.runtime_dir.join("cache"))
            .env("RUST_LOG", "off")
            .output()
            .expect("spawn master-voice")
    }

    pub fn run_stdin(&self, args: &[&str], stdin: &str) -> Output {
        let bin = env!("CARGO_BIN_EXE_master-voice");
        let mut child = Command::new(bin)
            .args(args)
            .env("XDG_RUNTIME_DIR", &self.runtime_dir)
            .env("XDG_CONFIG_HOME", self.runtime_dir.join("config"))
            .env("XDG_CACHE_HOME", self.runtime_dir.join("cache"))
            .env("RUST_LOG", "off")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("spawn master-voice");
        std::io::Write::write_all(child.stdin.as_mut().unwrap(), stdin.as_bytes()).unwrap();
        drop(child.stdin.take());
        child.wait_with_output().expect("wait master-voice")
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.runtime_dir);
    }
}

pub fn stdout_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub fn stderr_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
