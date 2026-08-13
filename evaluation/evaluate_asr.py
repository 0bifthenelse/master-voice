#!/usr/bin/env python3

import argparse
import json
import math
import os
import re
import subprocess
import unicodedata
import wave
from pathlib import Path


def normalize_text(text: str) -> str:
    decomposed = unicodedata.normalize("NFKD", text)
    ascii_text = "".join(char for char in decomposed if not unicodedata.combining(char))
    return " ".join(re.findall(r"[A-Z0-9]+", ascii_text.upper()))


def edit_distance(reference: list[str], hypothesis: list[str]) -> int:
    previous = list(range(len(hypothesis) + 1))
    for ref_index, ref_item in enumerate(reference, start=1):
        current = [ref_index]
        for hyp_index, hyp_item in enumerate(hypothesis, start=1):
            current.append(
                min(
                    current[-1] + 1,
                    previous[hyp_index] + 1,
                    previous[hyp_index - 1] + (ref_item != hyp_item),
                )
            )
        previous = current
    return previous[-1]


def wav_metrics(path: Path) -> dict[str, float | int]:
    with wave.open(str(path), "rb") as wav_file:
        channels = wav_file.getnchannels()
        sample_rate = wav_file.getframerate()
        sample_width = wav_file.getsampwidth()
        frame_count = wav_file.getnframes()
        raw = wav_file.readframes(frame_count)
    if sample_width != 2:
        raise RuntimeError(f"unsupported PCM width {sample_width} in {path}")
    samples = [
        int.from_bytes(raw[index : index + 2], "little", signed=True) / 32768.0
        for index in range(0, len(raw), 2)
    ]
    if not samples:
        raise RuntimeError(f"empty WAV {path}")
    deltas = [abs(right - left) for left, right in zip(samples, samples[1:])]
    ordered_deltas = sorted(deltas)
    p999_index = min(len(ordered_deltas) - 1, math.floor(0.999 * len(ordered_deltas)))
    return {
        "channels": channels,
        "sample_rate": sample_rate,
        "frames": frame_count,
        "duration_seconds": frame_count / sample_rate,
        "peak": max(abs(sample) for sample in samples),
        "rms": math.sqrt(sum(sample * sample for sample in samples) / len(samples)),
        "dc": sum(samples) / len(samples),
        "max_delta": max(deltas, default=0.0),
        "p999_delta": ordered_deltas[p999_index] if ordered_deltas else 0.0,
        "clipped_samples": sum(abs(sample) >= 1.0 for sample in samples),
    }


def render_master(
    binary: Path,
    output: Path,
    text: str,
    language: str,
    depth: float,
    config_home: Path,
) -> None:
    environment = os.environ.copy()
    environment["XDG_CONFIG_HOME"] = str(config_home)
    command = [
        str(binary),
        "--language",
        "en-US" if language == "en" else "fr-FR",
        "--robotic",
        str(depth),
        "--output-wav",
        str(output),
        text,
    ]
    subprocess.run(command, check=True, env=environment, capture_output=True, text=True)


def render_espeak(output: Path, text: str, language: str) -> None:
    voice = "en-gb" if language == "en" else "fr+m1"
    command = ["espeak-ng", "-v", voice, "-w", str(output), text]
    subprocess.run(command, check=True, capture_output=True, text=True)


def transcribe(whisper: Path, model: Path, wav_path: Path, language: str) -> str:
    command = [
        str(whisper),
        "-m",
        str(model),
        "-l",
        language,
        "-bs",
        "5",
        "-tp",
        "0",
        "-nf",
        "-nt",
        "-np",
        str(wav_path),
    ]
    result = subprocess.run(command, check=True, capture_output=True, text=True)
    return " ".join(result.stdout.split())


def score_rows(rows: list[dict]) -> dict[str, float | int]:
    word_errors = 0
    reference_words = 0
    character_errors = 0
    reference_characters = 0
    for row in rows:
        reference = normalize_text(row["reference"])
        hypothesis = normalize_text(row["transcript"])
        reference_word_list = reference.split()
        hypothesis_word_list = hypothesis.split()
        reference_character_list = list(reference.replace(" ", ""))
        hypothesis_character_list = list(hypothesis.replace(" ", ""))
        row_word_errors = edit_distance(reference_word_list, hypothesis_word_list)
        row_character_errors = edit_distance(reference_character_list, hypothesis_character_list)
        row["normalized_reference"] = reference
        row["normalized_transcript"] = hypothesis
        row["word_errors"] = row_word_errors
        row["character_errors"] = row_character_errors
        row["wer"] = row_word_errors / max(1, len(reference_word_list))
        row["cer"] = row_character_errors / max(1, len(reference_character_list))
        word_errors += row_word_errors
        reference_words += len(reference_word_list)
        character_errors += row_character_errors
        reference_characters += len(reference_character_list)
    return {
        "sentences": len(rows),
        "word_errors": word_errors,
        "reference_words": reference_words,
        "wer": word_errors / max(1, reference_words),
        "character_errors": character_errors,
        "reference_characters": reference_characters,
        "cer": character_errors / max(1, reference_characters),
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/release/master-voice"))
    parser.add_argument("--whisper", type=Path, required=True)
    parser.add_argument("--model", type=Path, required=True)
    parser.add_argument("--corpus", type=Path, default=Path("evaluation/corpus.json"))
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--engine", choices=("master", "espeak"), default="master")
    parser.add_argument("--sets", default="canonical_en,heldout_en,heldout_fr")
    parser.add_argument("--depths", default="0,0.22")
    parser.add_argument("--config-home", type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    corpus = json.loads(args.corpus.read_text(encoding="utf-8"))
    requested_sets = [name.strip() for name in args.sets.split(",") if name.strip()]
    depths = [float(value) for value in args.depths.split(",")] if args.engine == "master" else [0.0]
    args.output.mkdir(parents=True, exist_ok=True)
    config_home = args.config_home or args.output / "empty-config"
    config_home.mkdir(parents=True, exist_ok=True)
    result = {
        "engine": args.engine,
        "whisper": str(args.whisper.resolve()),
        "model": str(args.model.resolve()),
        "beam_size": 5,
        "temperature": 0,
        "no_fallback": True,
        "sets": {},
    }
    for depth in depths:
        depth_name = "espeak" if args.engine == "espeak" else f"robotic-{depth:g}"
        for set_name in requested_sets:
            language = "fr" if set_name.endswith("_fr") else "en"
            rows = []
            wav_dir = args.output / depth_name / set_name
            wav_dir.mkdir(parents=True, exist_ok=True)
            for index, text in enumerate(corpus[set_name]):
                wav_path = wav_dir / f"{index:03d}.wav"
                if args.engine == "master":
                    render_master(args.binary.resolve(), wav_path, text, language, depth, config_home)
                else:
                    render_espeak(wav_path, text, language)
                transcript = transcribe(args.whisper.resolve(), args.model.resolve(), wav_path, language)
                rows.append(
                    {
                        "index": index,
                        "reference": text,
                        "transcript": transcript,
                        "wav": str(wav_path),
                        "audio": wav_metrics(wav_path),
                    }
                )
            result["sets"][f"{depth_name}/{set_name}"] = {
                "language": language,
                "robotic_depth": None if args.engine == "espeak" else depth,
                "score": score_rows(rows),
                "rows": rows,
            }
            args.output.joinpath("result.json").write_text(
                json.dumps(result, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
            )
    print(json.dumps({name: data["score"] for name, data in result["sets"].items()}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
