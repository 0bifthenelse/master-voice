use clap::{Parser, Subcommand};
use master_voice_core::daemon::client::DaemonClient;
use master_voice_core::daemon::server::{self, DaemonConfig};
use master_voice_core::engine::{self, EngineSettings};
use master_voice_core::Error;
use master_voice_linguistics::lang::Language;
use std::io::IsTerminal;
use std::time::Duration;

mod wav;

#[derive(Parser)]
#[command(
    name = "master-voice",
    version,
    about = "Offline robotic speech synthesis (CLI, MCP, daemon)"
)]
struct Cli {
    /// Text to speak aloud. Positional text is the primary interface:
    /// master-voice "Hello, MASTER voice is online."
    #[arg(
        value_name = "TEXT",
        trailing_var_arg = true,
        allow_hyphen_values = true
    )]
    text: Vec<String>,

    /// Language override: fr-FR, en-US, or auto
    #[arg(long, value_name = "LANG")]
    language: Option<String>,

    /// Stop the previous utterance before speaking this one
    #[arg(long)]
    interrupt: bool,

    /// Character amount 0.0-1.0 for this invocation (0.0 = plain speech,
    /// 1.0 = full replicant); overrides config robotic_depth
    #[arg(long, value_name = "0.0-1.0")]
    robotic: Option<f32>,

    /// Write 16-bit PCM WAV to PATH instead of playing (no audio device
    /// touched)
    #[arg(long, value_name = "PATH")]
    output_wav: Option<std::path::PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run the MCP stdio server (protocol 2025-03-26)
    Mcp,
    /// Run the playback daemon in the foreground
    Serve {
        /// Detached quiet mode (used when auto-spawning)
        #[arg(long)]
        daemon: bool,
    },
    /// List supported languages
    Languages,
    /// List audio output devices
    Devices,
    /// Run diagnostics
    Doctor,
}

fn init_tracing(verbose: bool) {
    let level = if verbose { "info" } else { "warn" };
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| level.to_string());
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();
}

fn speak(
    text: &str,
    mut language: Option<&str>,
    interrupt: bool,
    robotic: Option<f32>,
    output_wav: Option<&std::path::Path>,
) -> Result<i32, Error> {
    if text.trim().is_empty() {
        return Err(Error::Language("no speakable text".to_string()));
    }
    if let Some(code) = language {
        if code.eq_ignore_ascii_case("auto") {
            language = None;
        } else if Language::from_code(code).is_none() {
            return Err(Error::Language(format!("unsupported language {code:?}")));
        }
    }
    if let Some(path) = output_wav {
        // Headless render: in-process synthesis, no audio device.
        let config = master_voice_core::config::load_config().map_err(Error::Config)?;
        let mut settings = EngineSettings::from_config(&config);
        if let Some(v) = robotic {
            settings.robotic_depth = v.clamp(0.0, 1.0);
        }
        let overrides = engine::overrides_from_config(&config);
        let (language_used, buffer, _synth_ms) = engine::synthesize_text(
            text,
            language.and_then(Language::from_code),
            &settings,
            &overrides,
        )?;
        wav::write_wav(path, &buffer.samples, buffer.sample_rate)
            .map_err(|e| Error::Usage(format!("cannot write {path:?}: {e}")))?;
        println!(
            "wrote {} ({} samples, {:.2} s, {})",
            path.display(),
            buffer.samples.len(),
            buffer.samples.len() as f32 / buffer.sample_rate as f32,
            language_used.code()
        );
        return Ok(0);
    }
    let mut client = DaemonClient::connect_or_spawn()?;
    let report = client.speak_with_id(1, text, language, interrupt, robotic)?;
    tracing::debug!(
        "spoke {} for {:.1}s ({}) accepted={:.1}ms synth={:.1}ms",
        report.language,
        report.duration_s,
        if report.status == master_voice_core::daemon::client::SpeakStatus::Interrupted {
            "interrupted"
        } else {
            "played"
        },
        report.accepted_ms,
        report.synth_ms
    );
    Ok(0)
}

fn read_stdin() -> Result<String, Error> {
    let mut buffer = Vec::new();
    std::io::Read::read_to_end(&mut std::io::stdin().lock(), &mut buffer)
        .map_err(|e| Error::Usage(format!("cannot read stdin: {e}")))?;
    Ok(String::from_utf8_lossy(&buffer).into_owned())
}

fn cmd_languages() -> Result<i32, Error> {
    println!("fr-FR  French (first-class)");
    println!("en-US  English (first-class)");
    println!("auto   automatic language routing (default)");
    Ok(0)
}

fn cmd_devices() -> Result<i32, Error> {
    let devices = master_voice_audio::list_devices();
    if devices.is_empty() {
        eprintln!("no audio output devices found");
        return Ok(0);
    }
    for device in devices {
        println!(
            "{}  ({} channels, {} Hz)",
            device.name, device.channels, device.sample_rate
        );
    }
    Ok(0)
}

fn cmd_doctor() -> Result<i32, Error> {
    let config_path = master_voice_core::config::config_path();
    println!("config file: {}", config_path.display());
    match master_voice_core::config::load_config() {
        Ok(config) => {
            println!("config: parse OK");
            let settings = EngineSettings::from_config(&config);
            println!(
                "settings: language={:?} rate={} pitch={} volume={} robotic_depth={}",
                settings.language,
                settings.rate,
                settings.pitch,
                settings.volume,
                settings.robotic_depth
            );
            let overrides = master_voice_core::engine::overrides_from_config(&config);
            println!("overrides: {} entries", {
                let _ = &overrides;
                if overrides.is_empty() {
                    0
                } else {
                    1
                }
            });
        }
        Err(message) => {
            println!("config: PARSE FAILED: {message}");
        }
    }
    let mut client = match DaemonClient::connect() {
        Ok(client) => client,
        Err(_) => {
            println!("daemon: not running");
            let _ = DaemonClient::connect_or_spawn()?;
            DaemonClient::connect().map_err(|e| Error::Daemon(e.to_string()))?
        }
    };
    if client.ping() {
        println!("daemon: running and responding");
    } else {
        println!("daemon: not responding");
    }
    let _ = client;

    match master_voice_audio::default_device_info() {
        Ok(info) => println!(
            "audio device: {} ({} channels, {} Hz)",
            info.name, info.channels, info.sample_rate
        ),
        Err(e) => println!("audio device: NONE ({e})"),
    }

    let config = master_voice_core::config::load_config().unwrap_or_default();
    let settings = EngineSettings::from_config(&config);
    let overrides = engine::overrides_from_config(&config);
    match engine::synthesize_text("Master voice online.", None, &settings, &overrides) {
        Ok((language, buffer, synth_ms)) => println!(
            "engine self-test: OK ({} ms, {} samples, {} s of audio, {})",
            synth_ms as u64,
            buffer.samples.len(),
            buffer.samples.len() as f32 / buffer.sample_rate as f32,
            language.code()
        ),
        Err(e) => println!("engine self-test: FAILED ({e})"),
    }

    let espeak_ng = std::path::Path::new("/usr/bin/espeak-ng").exists()
        || std::path::Path::new("/usr/bin/espeak").exists();
    println!(
        "espeak-ng baseline: {}",
        if espeak_ng {
            "available"
        } else {
            "not installed"
        }
    );
    Ok(0)
}

fn main() {
    let cli = Cli::parse();
    let result = run(cli);
    match result {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("master-voice: {e}");
            std::process::exit(e.exit_code());
        }
    }
}

fn run(cli: Cli) -> Result<i32, Error> {
    match &cli.command {
        Some(Command::Mcp) => {
            init_tracing(false);
            master_voice_mcp::serve_stdio().map_err(|e| Error::Daemon(e.to_string()))?;
            return Ok(0);
        }
        Some(Command::Serve { daemon }) => {
            init_tracing(!daemon);
            let config = master_voice_core::config::load_config().map_err(Error::Config)?;
            let settings = EngineSettings::from_config(&config);
            let overrides = engine::overrides_from_config(&config);
            let daemon_config = DaemonConfig {
                device: config.device.clone(),
                queue_limit: config.queue_limit.unwrap_or(16),
                idle_timeout: Duration::from_secs(config.daemon_idle_timeout_secs.unwrap_or(300)),
            };
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|e| Error::Daemon(e.to_string()))?;
            runtime
                .block_on(server::run(settings, overrides, daemon_config))
                .map_err(Error::Daemon)?;
            return Ok(0);
        }
        Some(Command::Languages) => return cmd_languages(),
        Some(Command::Devices) => return cmd_devices(),
        Some(Command::Doctor) => {
            init_tracing(false);
            return cmd_doctor();
        }
        None => {}
    }

    init_tracing(false);

    if !cli.text.is_empty() {
        let text = cli.text.join(" ");
        return speak(
            &text,
            cli.language.as_deref(),
            cli.interrupt,
            cli.robotic,
            cli.output_wav.as_deref(),
        );
    }

    let stdin = std::io::stdin();
    if !stdin.is_terminal() {
        let text = read_stdin()?;
        return speak(
            &text,
            cli.language.as_deref(),
            cli.interrupt,
            cli.robotic,
            cli.output_wav.as_deref(),
        );
    }

    print_usage();
    Ok(0)
}

fn print_usage() {
    let usage = r#"master-voice: speak text aloud through the default audio output

USAGE:
  master-voice "text to speak"
  echo "text" | master-voice
  master-voice --language fr-FR "Bonjour à tous."
  master-voice --interrupt "Stop the previous utterance and speak this."
  master-voice <subcommand>

SUBCOMMANDS:
  mcp          Run the MCP stdio server
  serve        Run the playback daemon in the foreground
  languages    List supported languages
  devices      List audio output devices
  doctor       Run diagnostics

OPTIONS:
  --language <LANG>   Language override: fr-FR, en-US, or auto
  --interrupt         Stop the previous utterance before speaking
  --help              Show help
  --version           Show version

Text is treated strictly as speech data. It is never executed.
"#;
    eprint!("{usage}");
}
