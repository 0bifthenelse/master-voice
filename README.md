# master-voice

Offline, Linux-first synthetic speech system in Rust. Robotic but highly
intelligible voice (Klatt-style formant synthesis — no models, no network, no
system packages). One engine behind a CLI, an MCP server, and OMP automatic
speech.

Supported languages: **French (fr-FR, first-class)** and **English (en-US)**,
with automatic per-sentence language routing. Spanish/German/Italian are not
implemented; the engine architecture accepts new languages as new G2P modules.

## Build

```sh
cargo build --workspace --release
# binary: target/release/master-voice
```

Rust stable (rust-version 1.94), crates.io dependencies only. Runtime
dependencies: an ALSA/PulseAudio/PipeWire output device (via CPAL).

## Install (user, no root)

```sh
install -m 755 target/release/master-voice ~/.local/bin/master-voice
```

Never overwrites an existing binary by itself — check for a pre-existing file
first. Ensure `~/.local/bin` is on `PATH`.

## Usage

```sh
master-voice "Hello, MASTER voice is online."       # positional text (primary)
echo "Speech through standard input." | master-voice # stdin when piped
master-voice --language fr-FR "Bonjour à tous."     # language override
master-voice --language en-US "System online."
master-voice --interrupt "Stop the previous utterance and speak this."
master-voice languages   # fr-FR / en-US / auto
master-voice devices     # enumerate audio outputs
master-voice doctor      # diagnostics (config, daemon, device, engine, espeak-ng)
master-voice mcp         # MCP stdio server (protocol 2025-03-26)
master-voice serve       # playback daemon in the foreground (normally auto-spawned)
```

No arguments on a TTY prints usage and exits. Text is always opaque speech
data — it is never shell-evaluated, executed, or sent anywhere.

Exit codes: 0 ok · 1 usage · 2 config · 3 language · 4 synthesis · 5 audio ·
6 daemon · 7 queue full.

## Configuration

Optional, XDG-compatible: `$XDG_CONFIG_HOME/master-voice/config.toml`
(default `~/.config/master-voice/config.toml`). Direct speech works with no
configuration at all.

```toml
language = "auto"            # "fr-FR" | "en-US" | "auto"
rate = 1.0                   # speech rate multiplier
pitch = 1.0                  # pitch multiplier
volume = 1.0                 # output volume multiplier
robotic_depth = 0.6          # 0.0 = softer, 1.0 = more robotic
device = "default"           # optional output device name (see `master-voice devices`)
queue_limit = 16             # bounded playback FIFO
daemon_idle_timeout_secs = 300
omp_auto_speech = true       # OMP auto-speech master switch (extension reads this)

[overrides]
# pronunciation overrides without recompilation, e.g.:
# gif = "J IH F"
# ChatGPT = "CH AE T G P T"
```

The playback daemon socket lives at `$XDG_RUNTIME_DIR/master-voice.sock`
(fallback: temp dir).

## MCP

`master-voice mcp` exposes one tool:

```
speak(text: string, language?: string, interrupt?: boolean)
```

Stdio, protocol 2025-03-26; stdout carries protocol frames only; logs go to
stderr; `shutdown` exits cleanly, cancelling outstanding speech.

## OMP integration

- Explicit calls: register the server in `~/.omp/agent/mcp.json` as
  `{"command": "$HOME/.local/bin/master-voice", "args": ["mcp"], "type": "stdio"}`.
- Automatic speech of completed agent output: a separate auto-discovered
  extension (`~/.omp/agent/extensions/robotic-voice.ts`) spawns
  `master-voice` with argv (no shell). Routing, enable/disable
  (`omp_auto_speech`), and command override are documented in that file.

## Architecture

```
text → unicode cleanup → sentence split → language routing → per-language
normalization (numbers, dates, URLs…) → rule-based G2P → pronunciation
overrides → phonemes → prosody → Klatt formant synthesis → robotic DSP →
resample → CPAL playback (bounded FIFO, interrupt, one daemon per user)
```

Crates: `linguistics` (G2P/normalization), `synth` (formant synthesizer),
`audio` (CPAL + queue), `core` (engine + playback daemon), `mcp` (MCP server),
`cli` (binary). CLI and MCP are clients of the auto-spawned daemon, which is
the single playback authority — this gives warm synthesis, cross-process
interrupt, and clean shutdown.

## Verification

```sh
cargo fmt --all --check
cargo check --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

110 tests: G2P corpora (fr/en), normalization, numbers, sentence boundaries,
overrides, synthesis determinism, queue semantics, shell-safety battery
(no input text can execute), MCP protocol end-to-end, CLI behaviors.

## License

Code: MIT OR Apache-2.0. All dependencies are permissively licensed (no
GPL/AGPL; see COMPLETION_REPORT.md §17 for the full audit). No model or data
licenses apply — no external data is used.
