from __future__ import annotations

import argparse
import base64
import json
import os
import socket
import struct
import subprocess
import sys
import time
import uuid
from dataclasses import dataclass
from pathlib import Path
from typing import Any, TextIO
from urllib import error, request

DEFAULT_REQUEST = Path(
    "fixtures/advisor_requests/sidea_2p_nature_match12_early_spirit_choice_request.json"
)


@dataclass(frozen=True)
class ServiceProcess:
    process: subprocess.Popen[str]
    stderr_log: Path
    stderr: TextIO


def find_free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as probe:
        probe.bind(("127.0.0.1", 0))
        return int(probe.getsockname()[1])


def start_service(port: int, log_dir: Path) -> ServiceProcess:
    log_dir.mkdir(parents=True, exist_ok=True)
    stderr_log = log_dir / f"service-smoke-{uuid.uuid4()}.stderr.log"
    env = os.environ.copy()
    env["HARMONIES_SERVICE_HOST"] = "127.0.0.1"
    env["HARMONIES_SERVICE_PORT"] = str(port)
    env.setdefault("HARMONIES_CATALOG", "docs/cards_database.json")
    env.setdefault("HARMONIES_WEIGHTS", "docs/weights.baseline.json")
    stderr = stderr_log.open("w", encoding="utf-8")
    process = subprocess.Popen(
        ["cargo", "run", "-q", "-p", "harmonies-service"],
        stdout=subprocess.DEVNULL,
        stderr=stderr,
        text=True,
        env=env,
    )
    return ServiceProcess(process=process, stderr_log=stderr_log, stderr=stderr)


def stop_service(service: ServiceProcess) -> None:
    if service.process.poll() is None:
        service.process.terminate()
        try:
            service.process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            service.process.kill()
            service.process.wait(timeout=10)
    service.stderr.close()


def post_json(url: str, payload: dict[str, Any], timeout: float) -> dict[str, Any]:
    encoded = json.dumps(payload).encode("utf-8")
    http_request = request.Request(
        url,
        data=encoded,
        method="POST",
        headers={"Content-Type": "application/json"},
    )
    with request.urlopen(http_request, timeout=timeout) as response:
        return json.loads(response.read().decode("utf-8"))


def get_json(url: str, timeout: float) -> dict[str, Any]:
    with request.urlopen(url, timeout=timeout) as response:
        return json.loads(response.read().decode("utf-8"))


def wait_for_health(port: int, deadline_seconds: float) -> dict[str, Any]:
    url = f"http://127.0.0.1:{port}/health"
    deadline = time.monotonic() + deadline_seconds
    last_error: Exception | None = None
    while time.monotonic() < deadline:
        try:
            health = get_json(url, timeout=1.0)
            if health.get("status") == "ok":
                return health
        except (OSError, error.URLError) as exc:
            last_error = exc
        time.sleep(0.25)
    raise RuntimeError(f"service health timeout: {last_error}")


def websocket_request(port: int, payload: dict[str, Any], timeout: float) -> list[dict[str, Any]]:
    with socket.create_connection(("127.0.0.1", port), timeout=timeout) as sock:
        sock.settimeout(timeout)
        websocket_handshake(sock, port)
        websocket_send_text(sock, json.dumps(payload))
        events: list[dict[str, Any]] = []
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            opcode, body = websocket_recv_frame(sock)
            if opcode == 8:
                break
            if opcode != 1:
                continue
            event = json.loads(body.decode("utf-8"))
            events.append(event)
            if event.get("final") is True:
                return events
        raise RuntimeError("websocket final response timeout")


def websocket_handshake(sock: socket.socket, port: int) -> None:
    key = base64.b64encode(os.urandom(16)).decode("ascii")
    request_text = (
        "GET /ws HTTP/1.1\r\n"
        f"Host: 127.0.0.1:{port}\r\n"
        "Upgrade: websocket\r\n"
        "Connection: Upgrade\r\n"
        f"Sec-WebSocket-Key: {key}\r\n"
        "Sec-WebSocket-Version: 13\r\n"
        "\r\n"
    )
    sock.sendall(request_text.encode("ascii"))
    response = sock.recv(4096).decode("iso-8859-1")
    if " 101 " not in response.split("\r\n", 1)[0]:
        raise RuntimeError(f"websocket upgrade failed: {response.splitlines()[0]}")


def websocket_send_text(sock: socket.socket, text: str) -> None:
    payload = text.encode("utf-8")
    header = bytearray([0x81])
    if len(payload) < 126:
        header.append(0x80 | len(payload))
    elif len(payload) <= 0xFFFF:
        header.append(0x80 | 126)
        header.extend(struct.pack("!H", len(payload)))
    else:
        header.append(0x80 | 127)
        header.extend(struct.pack("!Q", len(payload)))
    mask = os.urandom(4)
    masked = bytes(byte ^ mask[index % 4] for index, byte in enumerate(payload))
    sock.sendall(bytes(header) + mask + masked)


def websocket_recv_frame(sock: socket.socket) -> tuple[int, bytes]:
    first, second = read_exact(sock, 2)
    opcode = first & 0x0F
    masked = bool(second & 0x80)
    length = second & 0x7F
    if length == 126:
        length = struct.unpack("!H", read_exact(sock, 2))[0]
    elif length == 127:
        length = struct.unpack("!Q", read_exact(sock, 8))[0]
    mask = read_exact(sock, 4) if masked else b""
    payload = read_exact(sock, length)
    if masked:
        payload = bytes(byte ^ mask[index % 4] for index, byte in enumerate(payload))
    return opcode, payload


def read_exact(sock: socket.socket, size: int) -> bytes:
    chunks = bytearray()
    while len(chunks) < size:
        chunk = sock.recv(size - len(chunks))
        if not chunk:
            raise RuntimeError("socket closed")
        chunks.extend(chunk)
    return bytes(chunks)


def not_participant_request(payload: dict[str, Any]) -> dict[str, Any]:
    cheap = json.loads(json.dumps(payload))
    snapshot = cheap["snapshot"]
    other_player = next(
        player["playerId"]
        for player in snapshot["players"]
        if player["playerId"] != snapshot["perspectivePlayerId"]
    )
    snapshot["activePlayerId"] = other_player
    cheap["timeBudgetMs"] = 100
    return cheap


def validate_ws(events: list[dict[str, Any]]) -> dict[str, Any]:
    final = events[-1]
    response = final.get("response", {})
    moves = response.get("bestMoves") or []
    first_action = (((moves[0] or {}).get("orderedActions") or [None])[0] or {})
    if response.get("status") != "ready" or not moves:
        raise RuntimeError(f"unexpected websocket response: {response}")
    if first_action.get("kind") != "chooseSpirit":
        raise RuntimeError(f"expected first action chooseSpirit, got {first_action}")
    return {
        "events": len(events),
        "status": response.get("status"),
        "elapsedMs": response.get("elapsedMs"),
        "bestMoves": len(moves),
        "firstAction": first_action.get("kind"),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description="Smoke test local Harmonies advisor service.")
    parser.add_argument("--request", type=Path, default=DEFAULT_REQUEST)
    parser.add_argument("--startup-timeout", type=float, default=60.0)
    parser.add_argument("--advisor-timeout", type=float, default=60.0)
    parser.add_argument("--log-dir", type=Path, default=Path("logs/service_smoke"))
    args = parser.parse_args()

    payload = json.loads(args.request.read_text(encoding="utf-8"))
    port = find_free_port()
    service = start_service(port, args.log_dir)
    try:
        health = wait_for_health(port, args.startup_timeout)
        http_response = post_json(
            f"http://127.0.0.1:{port}/advise",
            not_participant_request(payload),
            timeout=5.0,
        )
        if http_response.get("status") != "notParticipantTurn":
            raise RuntimeError(f"unexpected HTTP response: {http_response}")
        events = websocket_request(port, payload, args.advisor_timeout)
        output = {
            "ok": True,
            "port": port,
            "health": health,
            "httpStatus": http_response.get("status"),
            "websocket": validate_ws(events),
            "stderrLog": str(service.stderr_log),
        }
        print(json.dumps(output, indent=2))
    finally:
        stop_service(service)
    if service.process.returncode not in (0, -15, 1):
        print(f"service exited with {service.process.returncode}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
