mod common;

use common::{stderr_text, stdout_text, TestEnv};

#[test]
fn version_prints() {
    let env = TestEnv::new();
    let output = env.run(&["--version"]);
    assert!(output.status.success());
    assert!(stdout_text(&output).contains("master-voice"));
}

#[test]
fn help_lists_positional_usage() {
    let env = TestEnv::new();
    let output = env.run(&["--help"]);
    assert!(output.status.success());
    let help = stdout_text(&output);
    assert!(help.contains("TEXT"));
    assert!(help.contains("--language"));
    assert!(help.contains("--interrupt"));
    assert!(help.contains("mcp"));
    assert!(help.contains("serve"));
    assert!(help.contains("languages"));
    assert!(help.contains("devices"));
    assert!(help.contains("doctor"));
}

#[test]
fn no_subcommand_required_for_speech() {
    let env = TestEnv::new();
    let output = env.run(&["system", "online"]);
    assert!(
        output.status.code() == Some(0) || output.status.code() == Some(5),
        "exit was {:?}, stderr: {}",
        output.status.code(),
        stderr_text(&output)
    );
}

#[test]
fn stdin_speech_works() {
    let env = TestEnv::new();
    let output = env.run_stdin(&[], "Speech through standard input.");
    assert!(
        output.status.code() == Some(0) || output.status.code() == Some(5),
        "exit was {:?}, stderr: {}",
        output.status.code(),
        stderr_text(&output)
    );
}

#[test]
fn empty_input_is_language_error() {
    let env = TestEnv::new();
    let output = env.run_stdin(&[], "");
    assert_eq!(output.status.code(), Some(3));
    assert!(stderr_text(&output).contains("no speakable text"));
}

#[test]
fn invalid_language_is_language_error() {
    let env = TestEnv::new();
    let output = env.run(&["--language", "xx-XX", "hello"]);
    assert_eq!(output.status.code(), Some(3));
    assert!(stderr_text(&output).contains("unsupported language"));
}

#[test]
fn french_language_flag_accepted() {
    let env = TestEnv::new();
    let output = env.run(&["--language", "fr-FR", "Bonjour."]);
    assert!(
        output.status.code() == Some(0) || output.status.code() == Some(5),
        "exit was {:?}, stderr: {}",
        output.status.code(),
        stderr_text(&output)
    );
}

#[test]
fn languages_lists_french_and_english() {
    let env = TestEnv::new();
    let output = env.run(&["languages"]);
    assert!(output.status.success());
    let text = stdout_text(&output);
    assert!(text.contains("fr-FR"));
    assert!(text.contains("en-US"));
}

#[test]
fn devices_does_not_panic() {
    let env = TestEnv::new();
    let output = env.run(&["devices"]);
    assert!(output.status.success());
}

#[test]
fn doctor_runs_diagnostics() {
    let env = TestEnv::new();
    let output = env.run(&["doctor"]);
    assert!(output.status.success());
    let text = stdout_text(&output);
    assert!(text.contains("engine self-test"));
    assert!(text.contains("audio device"));
}

#[test]
fn stdout_is_quiet_on_success() {
    let env = TestEnv::new();
    let output = env.run(&["--language", "en-US", "quiet"]);
    assert!(
        output.status.code() == Some(0) || output.status.code() == Some(5),
        "exit was {:?}",
        output.status.code()
    );
    if output.status.code() == Some(0) {
        assert!(stdout_text(&output).is_empty(), "stdout must be silent");
    }
}

#[test]
fn positional_text_writes_headless_wav() {
    let env = TestEnv::new();
    let path = env.runtime_dir.join("positional.wav");
    let output = env.run(&[
        "--output-wav",
        path.to_str().unwrap(),
        "--language",
        "en-US",
        "Hello. Master voice is online.",
    ]);
    assert!(output.status.success(), "{}", stderr_text(&output));
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(&bytes[..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    assert!(bytes.len() > 44);
}

#[test]
fn stdin_text_writes_headless_wav() {
    let env = TestEnv::new();
    let path = env.runtime_dir.join("stdin.wav");
    let output = env.run_stdin(
        &[
            "--output-wav",
            path.to_str().unwrap(),
            "--language",
            "fr-FR",
        ],
        "Un bon vin blanc, mon ami.",
    );
    assert!(output.status.success(), "{}", stderr_text(&output));
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(&bytes[..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    assert!(bytes.len() > 44);
}
