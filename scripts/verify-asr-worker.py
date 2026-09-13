"""Exercise a bundled worker without a microphone or provider request."""
import argparse
import json
import subprocess
import tempfile
import wave
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument("python")
parser.add_argument("worker")
parser.add_argument("--wav")
args = parser.parse_args()
with tempfile.TemporaryDirectory(prefix="voiceflow-worker-smoke-") as directory:
    wav_path = Path(args.wav).resolve() if args.wav else Path(directory) / "silence.wav"
    if not args.wav:
        with wave.open(str(wav_path), "wb") as audio:
            audio.setnchannels(1)
            audio.setsampwidth(2)
            audio.setframerate(16000)
            audio.writeframes(b"\x00\x00" * 4000)
    request = json.dumps({"cmd": "transcribe", "id": 1, "wav_path": str(wav_path)}) + "\n"
    request += json.dumps({"cmd": "shutdown"}) + "\n"
    result = subprocess.run(
        [str(Path(args.python).resolve()), "-B", str(Path(args.worker).resolve())],
        input=request, capture_output=True, text=True, encoding="utf-8", timeout=90,
        creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
    )
    if result.returncode:
        raise RuntimeError(f"Worker exited with code {result.returncode}: {result.stderr}")
    messages = [json.loads(line) for line in result.stdout.splitlines() if line.strip()]
    assert messages[0]["type"] == "ready" and messages[0]["ok"]
    transcript = next(message for message in messages if message["type"] == "transcript")
    assert transcript["ok"] and transcript["id"] == 1
    assert len(transcript["tokens"]) == len(transcript["timestamps"])
    if args.wav:
        assert transcript["tokens"], "speech fixture must produce timestamped tokens"
    print(json.dumps({"ready": True, "transcription_ok": True,
                      "token_count": len(transcript["tokens"]),
                      "timestamp_count": len(transcript["timestamps"])}))
