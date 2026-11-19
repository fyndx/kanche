# SPDX-License-Identifier: MPL-2.0

"""Exercise the actual binary: startup failures, safe logs, signals, and restart."""

import json
import os
import queue
import signal
import socket
import subprocess
import threading
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BINARY = ROOT / "target/debug/kanche"
base_env = {key: value for key, value in os.environ.items() if not key.startswith("KANCHE_")}


def invalid_start(settings):
    result = subprocess.run([BINARY], env={**base_env, **settings}, capture_output=True, text=True, timeout=10)
    assert result.returncode != 0, "Invalid startup must fail"
    assert "sensitive-input" not in result.stdout + result.stderr, "Error echoed configuration value"


invalid_start({"KANCHE_BIND_ADDR": "sensitive-input"})
invalid_start({"KANCHE_LOG_LEVEL": "sensitive-input"})
with socket.socket() as occupied:
    occupied.bind(("127.0.0.1", 0))
    occupied.listen()
    invalid_start({"KANCHE_BIND_ADDR": f"127.0.0.1:{occupied.getsockname()[1]}"})


def lifecycle(bind, shutdown_signal):
    lines = []
    events = queue.Queue()
    process = subprocess.Popen(
        [BINARY], env={**base_env, "KANCHE_BIND_ADDR": bind},
        stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
    )

    def read_output():
        for line in process.stdout:
            lines.append(line)
            events.put(line)
        events.put(None)

    reader = threading.Thread(target=read_output, daemon=True)
    reader.start()
    try:
        while True:
            line = events.get(timeout=10)
            assert line is not None, "Server exited before listening"
            event = json.loads(line)
            if event["fields"].get("message") == "Kanche listening":
                address = event["fields"]["address"]
                break
        ids = []
        for path, expected in [("/healthz?token=sensitive-input", 200), ("/sensitive-input", 404)]:
            request = urllib.request.Request(
                f"http://{address}{path}",
                headers={"Authorization": "Bearer sensitive-input", "Cookie": "secret=sensitive-input", "X-Request-Id": "sensitive-input"},
            )
            try:
                response = urllib.request.urlopen(request, timeout=5)
            except urllib.error.HTTPError as error:
                response = error
            with response:
                assert response.status == expected
                ids.append(response.headers["X-Request-Id"])
                if expected == 200:
                    assert response.read() == b"ok"
        process.send_signal(shutdown_signal)
        assert process.wait(timeout=12) == 0, "Graceful shutdown must succeed"
        reader.join(timeout=2)
        assert "sensitive-input" not in "".join(lines), "Request metadata leaked into logs"
        completed = [json.loads(line) for line in lines if json.loads(line)["fields"].get("message") == "request completed"]
        assert [event["span"]["request_id"] for event in completed] == ids
        assert [event["span"]["route"] for event in completed] == ["/healthz", "unmatched"]
        return address
    finally:
        if process.poll() is None:
            process.kill()
            process.wait(timeout=5)
        reader.join(timeout=2)
        process.stdout.close()


address = lifecycle("127.0.0.1:0", signal.SIGTERM)
lifecycle(address, signal.SIGINT)
print("PASS server startup rejection, log redaction, request IDs, SIGTERM/SIGINT shutdown, and port reuse")
