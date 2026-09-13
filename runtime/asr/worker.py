import json
import sys
import time
import wave
import array
import os
from pathlib import Path

import sherpa_onnx

BASE_DIR = Path(__file__).resolve().parent
MODEL_DIR = Path(os.environ.get("VOICEFLOW_ASR_MODEL_DIR", BASE_DIR))
MODEL_PATH = MODEL_DIR / "model.int8.onnx"
TOKENS_PATH = MODEL_DIR / "tokens.txt"


def send(message):
    sys.stdout.write(json.dumps(message, ensure_ascii=False) + "\n")
    sys.stdout.flush()


def log(message):
    sys.stderr.write(message + "\n")
    sys.stderr.flush()


def load_wav(filename: Path):
    t0 = time.perf_counter()
    with wave.open(str(filename), "rb") as f:
        sample_rate = f.getframerate()
        num_samples = f.getnframes()
        audio = f.readframes(num_samples)
    t1 = time.perf_counter()

    samples = array.array("h", audio)
    samples = [s / 32768 for s in samples]
    t2 = time.perf_counter()

    return samples, sample_rate, {
        "wav_read_ms": round((t1 - t0) * 1000, 2),
        "pcm_to_float_ms": round((t2 - t1) * 1000, 2),
    }


def main():
    try:
        if hasattr(sys.stdout, "reconfigure"):
            sys.stdout.reconfigure(encoding="utf-8", line_buffering=True)
        if hasattr(sys.stderr, "reconfigure"):
            sys.stderr.reconfigure(encoding="utf-8", line_buffering=True)

        t0 = time.perf_counter()
        recognizer = sherpa_onnx.OfflineRecognizer.from_sense_voice(
            model=str(MODEL_PATH),
            tokens=str(TOKENS_PATH),
        )
        t1 = time.perf_counter()

        log("[SenseVoice Worker] model loaded")
        send(
            {
                "type": "ready",
                "ok": True,
                "model_load_ms": round((t1 - t0) * 1000, 2),
            }
        )
    except Exception as e:
        send({"type": "ready", "ok": False, "error": str(e)})
        raise

    for raw_line in sys.stdin:
        line = raw_line.strip()
        if not line:
            continue

        try:
            req = json.loads(line)
        except Exception as e:
            send({"type": "error", "ok": False, "error": f"invalid_json: {e}"})
            continue

        cmd = req.get("cmd")
        req_id = req.get("id")

        if cmd == "shutdown":
            send({"type": "shutdown", "ok": True})
            return

        if cmd == "ping":
            send({"type": "pong", "ok": True, "id": req_id})
            continue

        if cmd != "transcribe":
            send(
                {
                    "type": "error",
                    "ok": False,
                    "id": req_id,
                    "error": f"unsupported_cmd: {cmd}",
                }
            )
            continue

        wav_path = req.get("wav_path")
        if not wav_path:
            send(
                {
                    "type": "transcript",
                    "ok": False,
                    "id": req_id,
                    "error": "missing_wav_path",
                }
            )
            continue

        try:
            total_start = time.perf_counter()
            samples, sample_rate, wav_timings = load_wav(Path(wav_path))

            stream_prep_start = time.perf_counter()
            stream = recognizer.create_stream()
            stream.accept_waveform(sample_rate, samples)
            stream_prep_end = time.perf_counter()

            infer_start = time.perf_counter()
            recognizer.decode_stream(stream)
            infer_end = time.perf_counter()

            result_start = time.perf_counter()
            text = stream.result.text
            result_end = time.perf_counter()

            send(
                {
                    "type": "transcript",
                    "ok": True,
                    "id": req_id,
                    "transcript": text,
                    "tokens": list(stream.result.tokens),
                    "timestamps": list(stream.result.timestamps),
                    "timings": {
                        **wav_timings,
                        "stream_prepare_ms": round((stream_prep_end - stream_prep_start) * 1000, 2),
                        "inference_ms": round((infer_end - infer_start) * 1000, 2),
                        "result_extract_ms": round((result_end - result_start) * 1000, 2),
                        "total_ms": round((result_end - total_start) * 1000, 2),
                    },
                }
            )
        except Exception as e:
            send(
                {
                    "type": "transcript",
                    "ok": False,
                    "id": req_id,
                    "error": str(e),
                }
            )


if __name__ == "__main__":
    main()
