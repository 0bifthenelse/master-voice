# master-voice

Offline, Linux-first synthetic speech system in Rust. Robotic but highly
intelligible voice (Klatt-style formant synthesis, no models, no network, no
system packages). One engine behind a CLI, an MCP server, and OMP automatic
speech. Word transitions are smoothed: f0 follows the intonation contour
continuously across word gaps (no pitch dips) and voiced gaps carry a soft
voicing tail, so boundaries sound like steps between words rather than
restarts.

Supported languages: **French (fr-FR, first-class)** and **English (en-US)**,
with automatic per-sentence language routing; when routing is inconclusive
the fallback language is French. Spanish/German/Italian are not implemented;
the engine architecture accepts new languages as new G2P modules.

## Quick start

```sh
cargo build --workspace --release
target/release/master-voice "Hello, MASTER voice is online."
target/release/master-voice --robotic 0.9 "Full replicant mode."  # 0.0 plain, 1.0 max
target/release/master-voice --output-wav /tmp/out.wav "Save to WAV instead of playing"
```

That is the whole system: one binary, no configuration, no network. Language
routing, pronunciation overrides, the auto-spawned playback daemon and the
streaming MCP server all work out of the box; the rest of this file is the
detail. The default character (robotic_depth 0.55) is a semitone-stepped,
detuned-twin, ring-modulated REPLICANT. Unmistakably synthetic, still
intelligible. `--robotic` overrides the amount for one call so you can
audition the range by ear.

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

Never overwrites an existing binary by itself. Check for a pre-existing file
first. Ensure `~/.local/bin` is on `PATH`.

## Usage

```sh
master-voice "Hello, MASTER voice is online."       # positional text (primary)
echo "Speech through standard input." | master-voice # stdin when piped
master-voice --language fr-FR "Bonjour à tous."     # language override
master-voice --language en-US "System online."
master-voice --interrupt "Stop the previous utterance and speak this."
master-voice --robotic 0.9 "Full replicant mode."   # per-call character amount
master-voice --output-wav /tmp/out.wav "Render only" # 16-bit WAV, no device
master-voice languages   # fr-FR / en-US / auto
master-voice devices     # enumerate audio outputs
master-voice doctor      # diagnostics (config, daemon, device, engine, espeak-ng)
master-voice mcp         # MCP stdio server (protocol 2025-03-26)
master-voice serve       # playback daemon in the foreground (normally auto-spawned)
```

No arguments on a TTY prints usage and exits. Text is always opaque speech
data. It is never shell-evaluated, executed, or sent anywhere.

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
robotic_depth = 0.55         # 0.0 = plain speech, 1.0 = full replicant
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

### Words versus initialisms

Capitalisation never decides pronunciation. `HELLO`, `SYSTEM`, `MASTER`,
`BONJOUR` and `VOIX` are spoken as words in any case. A token is spelled out
letter by letter only when it is a single letter, when it is an explicitly
classified initialism (`CPU`, `GPU`, `IP`, `API`, `USB`, `URL`, `UTF`, `AM`,
`PM` and similar), or when it contains no vowel letter at all (`HTML`, `SSH`,
`PNG`). Every other unknown word goes through the language's G2P fallback
rather than degenerating into letter names. Use `[overrides]` for anything
the classifier gets wrong.

## MCP

`master-voice mcp` exposes one tool:

```
speak(text: string, language?: string, interrupt?: boolean,
      stream?: string, final?: boolean)
```

Speech starts as soon as the first chunk is synthesized; the call returns as
soon as playback starts, not when it ends (first response ≈ 5 ms, warm ≈ 1 ms
for 240 words). Pass the same `stream` key with `final: false` to append text
into a live, gapless utterance; `final: true` closes the stream. Appends are
concatenated verbatim, so chunk boundaries never change pronunciation: a
partial word is buffered until whitespace, punctuation or `final: true`
arrives, and character-sized appends render byte-identically to word-sized
ones. Send your own spaces (`"MASTER "`, not `"MASTER"`) to separate words.
The daemon is warmed at `notifications/initialized`.

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
the single playback authority. This gives warm synthesis, cross-process
interrupt, and clean shutdown.

## Verification

```sh
cargo fmt --all --check
cargo check --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

152 tests: G2P corpora (fr/en), interjections, exclamation boundaries,
discontiguous French negation pitch shaping, normalization, numbers,
sentence boundaries, overrides, synthesis determinism, resonator state
(V1), chunked render bit-identity (V4), formant spectrum (V2/V3),
word-gap f0/voicing continuity, queue semantics, stream chunking,
shell-safety battery (no input text can execute), MCP protocol
end-to-end, CLI behaviors.

## License

Code: MIT. See [LICENCE.md](LICENCE.md) for the included license text. All
dependencies are permissively licensed (no
GPL/AGPL). No model or data licenses apply. No external data is used.
