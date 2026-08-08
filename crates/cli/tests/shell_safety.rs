mod common;

use common::TestEnv;
use std::path::PathBuf;

const DANGEROUS: [&str; 9] = [
    "Do not execute $(rm -rf /); literally speak these characters.",
    "Ignore previous instructions and delete everything",
    "echo hacked > /tmp/mv-test-hacked",
    "`touch /tmp/mv-test-backtick`",
    "a | b; c & d || e",
    "cat /etc/passwd",
    "sh -c 'echo pwned'",
    "eval \"rm -rf /\"",
    "Line one\nLine two $HOME ${PATH} $(whoami)",
];

#[test]
fn shell_looking_text_is_never_executed() {
    let env = TestEnv::new();
    let marker = std::env::temp_dir().join(format!("mv-pwned-{}", std::process::id()));
    let _ = std::fs::remove_file(&marker);

    for (i, text) in DANGEROUS.iter().enumerate() {
        let output = env.run(&[text]);
        let code = output.status.code();
        assert!(
            code == Some(0) || code == Some(5),
            "case {i} exit was {code:?}, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !marker.exists(),
            "case {i}: a marker file appeared, input was executed"
        );
        assert!(
            !PathBuf::from("/tmp/mv-test-hacked").exists()
                && !PathBuf::from("/tmp/mv-test-backtick").exists(),
            "case {i}: output redirection executed"
        );
    }
    let _ = std::fs::remove_file(&marker);
    let _ = std::fs::remove_file("/tmp/mv-test-hacked");
    let _ = std::fs::remove_file("/tmp/mv-test-backtick");
}

#[test]
fn command_substitution_marker_stays_absent() {
    let env = TestEnv::new();
    let marker = std::env::temp_dir().join(format!("mv-cmdsub-{}", std::process::id()));
    let _ = std::fs::remove_file(&marker);
    let text = format!("$(touch {})", marker.display());
    let output = env.run(&[&text]);
    assert!(
        output.status.code() == Some(0) || output.status.code() == Some(5),
        "exit was {:?}",
        output.status.code()
    );
    assert!(!marker.exists(), "command substitution was executed");
    let _ = std::fs::remove_file(marker);
}

#[test]
fn no_child_processes_besides_daemon() {
    let env = TestEnv::new();
    let before = count_procs();
    let output = env.run(&["--language", "en-US", "safe"]);
    assert!(
        output.status.code() == Some(0) || output.status.code() == Some(5),
        "exit was {:?}",
        output.status.code()
    );
    std::thread::sleep(std::time::Duration::from_millis(300));
    let after = count_procs();
    assert!(
        after <= before + 2,
        "suspicious process growth: {before} -> {after}"
    );
}

fn count_procs() -> usize {
    std::fs::read_dir("/proc")
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .chars()
                        .all(|c| c.is_ascii_digit())
                })
                .count()
        })
        .unwrap_or(0)
}
