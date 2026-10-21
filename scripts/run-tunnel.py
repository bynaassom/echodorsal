#!/usr/bin/env python3
"""Run Cloudflare Tunnel for Alexa skill using credentials from Keychain (F0-04)."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
from typing import Any

REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_CONFIG = REPO_ROOT / "config" / "cloudflared.example.yml"
DEFAULT_CLOUDFLARED = REPO_ROOT / ".cache" / "bin" / "cloudflared"
DEFAULT_KEYCHAIN_BIN = REPO_ROOT / ".cache" / "bin" / "tunnel-keychain"

STRIPPED_ENV_VARS = (
    "TUNNEL_TOKEN",
    "TUNNEL_TOKEN_FILE",
    "TUNNEL_CRED_CONTENTS",
    "TUNNEL_CRED_FILE",
)


def resolve_path(path: Path) -> Path:
    if path.is_absolute():
        return path.resolve()
    candidate = (REPO_ROOT / path).resolve()
    if candidate.exists():
        return candidate
    return path.resolve()


def validate_ingress(binary: Path, config: Path) -> None:
    if not binary.is_file() or not (binary.stat().st_mode & 0o111):
        sys.exit("Error: cloudflared binary not found or not executable.")
    if not config.is_file():
        sys.exit("Error: Tunnel configuration file not found.")

    try:
        res = subprocess.run(
            [str(binary), "tunnel", "--config", str(config), "ingress", "validate"],
            capture_output=True,
            check=False,
        )
        if res.returncode != 0:
            sys.exit("Error: Ingress validation failed.")
    except SystemExit:
        raise
    except Exception:
        sys.exit("Error: Ingress validation failed.")


def fetch_credentials(helper: Path) -> bytes:
    if not helper.is_file() or not (helper.stat().st_mode & 0o111):
        sys.exit("Error: Keychain helper not found or not executable.")

    try:
        res = subprocess.run(
            [str(helper), "get"],
            capture_output=True,
            check=False,
        )
        if res.returncode != 0 or not res.stdout:
            sys.exit("Error: Failed to retrieve credentials from Keychain.")
        return res.stdout
    except SystemExit:
        raise
    except Exception:
        sys.exit("Error: Failed to retrieve credentials from Keychain.")


def validate_credentials(cred_bytes: bytes, expected_uuid: str) -> None:
    try:
        data = json.loads(cred_bytes.decode("utf-8"))
    except Exception:
        sys.exit("Error: Failed to parse tunnel credentials JSON.")

    if not isinstance(data, dict):
        sys.exit("Error: Invalid tunnel credentials format.")

    tunnel_id = data.get("TunnelID")
    if not tunnel_id or not isinstance(tunnel_id, str):
        sys.exit("Error: Missing or invalid TunnelID in credentials.")

    if tunnel_id != expected_uuid and tunnel_id.lower() != expected_uuid.lower():
        sys.exit("Error: Tunnel ID mismatch.")


def get_clean_env() -> dict[str, str]:
    env = dict(os.environ)
    for var in STRIPPED_ENV_VARS:
        env.pop(var, None)
    return env


def main() -> int:
    for var in STRIPPED_ENV_VARS:
        os.environ.pop(var, None)

    parser = argparse.ArgumentParser(
        description="Run Cloudflare Tunnel passing credentials securely via anonymous stdin pipe."
    )
    parser.add_argument("uuid", help="Tunnel UUID")
    parser.add_argument(
        "--config",
        type=Path,
        default=DEFAULT_CONFIG,
        help="Path to cloudflared ingress config YAML (default: config/cloudflared.example.yml)",
    )
    parser.add_argument(
        "--bin",
        "--cloudflared",
        dest="cloudflared",
        type=Path,
        default=DEFAULT_CLOUDFLARED,
        help="Path to cloudflared binary (default: .cache/bin/cloudflared)",
    )
    parser.add_argument(
        "--keychain-bin",
        "--keychain-helper",
        dest="keychain_helper",
        type=Path,
        default=DEFAULT_KEYCHAIN_BIN,
        help="Path to tunnel-keychain helper (default: .cache/bin/tunnel-keychain)",
    )
    args = parser.parse_args()

    config_path = resolve_path(args.config)
    cloudflared_bin = resolve_path(args.cloudflared)
    keychain_helper = resolve_path(args.keychain_helper)

    validate_ingress(cloudflared_bin, config_path)
    cred_bytes = fetch_credentials(keychain_helper)
    validate_credentials(cred_bytes, args.uuid)

    env = get_clean_env()
    cmd = [
        str(cloudflared_bin),
        "tunnel",
        "--config",
        str(config_path),
        "--no-autoupdate",
        "--loglevel",
        "warn",
        "--metrics",
        "127.0.0.1:0",
        "--credentials-file",
        "/dev/fd/0",
        "run",
        args.uuid,
    ]

    try:
        proc = subprocess.Popen(
            cmd,
            stdin=subprocess.PIPE,
            env=env,
        )
    except Exception:
        sys.exit("Error: Failed to launch cloudflared.")

    try:
        if proc.stdin:
            proc.stdin.write(cred_bytes)
            proc.stdin.flush()
            proc.stdin.close()
    except (BrokenPipeError, OSError):
        pass
    except Exception:
        sys.exit("Error: Failed to write credentials to tunnel process.")

    def handle_signal(signum: int, _frame: Any) -> None:
        try:
            if proc.poll() is None:
                proc.terminate()
                try:
                    proc.wait(timeout=5)
                except Exception:
                    proc.kill()
        except Exception:
            pass
        print("Error: Tunnel process terminated by signal.", file=sys.stderr)
        sys.exit(128 + signum)

    signal.signal(signal.SIGINT, handle_signal)
    signal.signal(signal.SIGTERM, handle_signal)

    try:
        ret = proc.wait()
        if ret != 0:
            print("Error: Tunnel process exited with error.", file=sys.stderr)
            return ret if ret > 0 else 1
        return 0
    except KeyboardInterrupt:
        handle_signal(signal.SIGINT, None)
    except Exception:
        print("Error: Tunnel execution failed.", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
