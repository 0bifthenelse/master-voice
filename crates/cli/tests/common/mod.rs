#![allow(dead_code)]

use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::Duration;

pub fn shutdown_daemon(runtime_dir: &std::path::Path) {
    let socket = runtime_dir.join("master-voice.sock");
    let Ok(mut stream) = std::os::unix::net::UnixStream::connect(&socket) else {
        return;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
    if std::io::Write::write_all(&mut stream, b"{\"op\":\"shutdown\",\"id\":0}\n").is_err() {
        return;
    }
    let _ = std::io::Write::flush(&mut stream);
    let mut response = String::new();
    let _ = std::io::BufRead::read_line(&mut std::io::BufReader::new(stream), &mut response);
    for _ in 0..100 {
        if !socket.exists() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

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
            "daemon_idle_timeout_secs = 2\ndevice = \"definitely-not-a-device-xyz\"\n",
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
        shutdown_daemon(&self.runtime_dir);
        let _ = std::fs::remove_dir_all(&self.runtime_dir);
    }
}

pub fn stdout_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub fn stderr_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
