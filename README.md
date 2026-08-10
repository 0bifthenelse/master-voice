# master-voice

Offline, Linux-first procedural speech system. The renderer is an original,
model-free x86-64 GNU assembly source-filter engine: phone timing, continuous
prosody, coarticulation, glottal and deterministic-noise excitation,
resonators, robotic character, limiting, and 24 kHz mono PCM emission all run
from `.ma` source. Rust owns text normalization, language routing, rule-based
G2P, pronunciation overrides, the public API, daemon, and playback.

The project contains no Piper or Sherpa source or assets, no AI model, no
recorded voice, and no network synthesis path. Its target is clear,
characterful procedural speech, not speech indistinguishable from a human
recording.

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

That is the whole runtime system: one binary, no model download and no network.
Language routing, pronunciation overrides, the auto-spawned playback daemon
and the streaming MCP server work without a configuration file. The default
character depth is `0.22`, a subtle reference-inspired detuned and
ring-modulated layer over the intelligible source-filter core. `--robotic 0`
selects plain procedural speech; `--robotic 1` selects the full designed
character.

## Build

```sh
cargo build --workspace --release
# binary: target/release/master-voice
```

Build requirements: Linux x86-64, Rust stable (rust-version 1.94), GNU
binutils (`as` and `ar`), and crates.io dependencies. Runtime playback requires
an ALSA, PulseAudio, or PipeWire output device through CPAL. WAV rendering does
not touch an audio device.

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
robotic_depth = 0.22         # 0.0 = plain speech, 1.0 = full character
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

The first non-final stream chunk waits for four complete words of
pronunciation context unless terminal punctuation or `final: true` arrives.
Later chunks retain the existing character and length limits. This prevents
early words from losing cross-word English pronunciation context while
bounding startup latency.

Pass the same `stream` key with `final: false` to append text into one live
utterance; `final: true` closes it. Partial words are buffered until
whitespace, punctuation, or final input arrives. Assembly state persists
across every chunk: phase, noise seed, resonators, formant targets, prosodic
contour, robotic oscillators, and the output join crossfade do not restart.
Send spaces explicitly (`"MASTER "`, not `"MASTER"`) when appending separate
words. The daemon is warmed at `notifications/initialized`.

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

```text
text -> Unicode cleanup -> abbreviation-aware sentence splitting
     -> per-word language routing -> normalization -> rule-based G2P
     -> pronunciation overrides -> packed assembly phone ABI
     -> .ma timing/prosody/source/filter/character/limiter
     -> 24,000 Hz mono PCM -> CPAL playback or 16-bit WAV
```

Crates: `linguistics` owns normalization, language routing, and G2P; `synth`
owns the safe Rust FFI and the `.ma` translation unit; `audio` owns CPAL and
the bounded queue; `core` owns the engine and playback daemon; `mcp` owns the
MCP server; `cli` owns the binary. No Rust PCM synthesis fallback remains.

Streaming keeps linguistics in Rust and synthesis state in assembly. The
daemon holds back the first non-final fragment until it has four complete
words or terminal input, then renders bounded chunks through one persistent
`SynthState`. CLI and MCP are clients of the single auto-spawned playback
daemon, providing warm synthesis, cross-process interruption, and clean
shutdown.

## Verification

```sh
cargo fmt --all --check
cargo check --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

The workspace suite covers fixed French, English, and mixed-language
pronunciation corpora; unknown override errors; all 54 packed phone mappings;
ABI rejection and output canaries; finite bounded output; absence of
flat-topping; phone-level audibility; formant realization; F0 and question
contours; spectral tilt; deterministic one-shot and streaming rendering;
sub-0.02 chunk joins; queue semantics; shell safety; MCP protocol behavior;
and CLI behavior.

## License

Code: MIT. See [LICENCE.md](LICENCE.md) for the included license text. All
dependencies are permissively licensed; no GPL or AGPL dependency is included.
No model, recorded-voice, or external data license applies. The synthesis
implementation is original and copies no Piper or Sherpa source or asset.
