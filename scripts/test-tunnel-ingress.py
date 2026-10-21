#!/usr/bin/env python3
"""Offline test for Cloudflare Tunnel ingress rules (F0-04).

Validates the ingress rules configuration and asserts that:
- Hostname alexa.bynaassom.com.br with exact /alexa and /alexa?x=1 select rule #0 (http://127.0.0.1:8080)
- Paths /, /api, /alexa/, /alexa-extra, /alexa/api and different hostnames select rule #1 (http_status:404)
"""

from __future__ import annotations

import argparse
from dataclasses import dataclass
from pathlib import Path
import re
import subprocess
import sys


@dataclass(frozen=True)
class TestCase:
    url: str
    expected_rule: int
    expected_service: str


RULE_REGEX = re.compile(r"Matched rule #(\d+)")
SERVICE_REGEX = re.compile(r"service:\s*(\S+)")


def validate_ingress(binary: Path, config: Path) -> None:
    cmd = [str(binary), "tunnel", "--config", str(config), "ingress", "validate"]
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        error_msg = result.stderr.strip() or result.stdout.strip() or "unknown error"
        raise RuntimeError(f"Ingress validation failed (exit {result.returncode}): {error_msg}")
    if "OK" not in result.stdout:
        raise RuntimeError(f"Ingress validation did not report OK: {result.stdout.strip()}")


def evaluate_url(binary: Path, config: Path, url: str) -> tuple[int, str]:
    cmd = [str(binary), "tunnel", "--config", str(config), "ingress", "rule", url]
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        error_msg = result.stderr.strip() or result.stdout.strip() or "unknown error"
        raise RuntimeError(f"Failed to evaluate rule for URL '{url}' (exit {result.returncode}): {error_msg}")

    rule_match = RULE_REGEX.search(result.stdout)
    service_match = SERVICE_REGEX.search(result.stdout)

    if not rule_match or not service_match:
        raise RuntimeError(f"Could not parse rule evaluation output for URL '{url}':\n{result.stdout.strip()}")

    rule_idx = int(rule_match.group(1))
    service = service_match.group(1)
    return rule_idx, service


def main() -> int:
    repo_root = Path(__file__).resolve().parent.parent
    default_bin = repo_root / ".cache" / "bin" / "cloudflared"
    default_config = repo_root / "config" / "cloudflared.example.yml"

    parser = argparse.ArgumentParser(description="Test Cloudflare Tunnel ingress rules offline.")
    parser.add_argument("--bin", type=Path, default=default_bin, help="Path to cloudflared binary")
    parser.add_argument("--config", type=Path, default=default_config, help="Path to cloudflared config yaml")
    args = parser.parse_args()

    binary = args.bin.resolve()
    config = args.config.resolve()

    if not binary.is_file():
        print(f"FAIL: cloudflared binary not found at: {binary}", file=sys.stderr)
        return 1
    if not (binary.stat().st_mode & 0o111):
        print(f"FAIL: cloudflared binary is not executable: {binary}", file=sys.stderr)
        return 1
    if not config.is_file():
        print(f"FAIL: config file not found at: {config}", file=sys.stderr)
        return 1

    print(f"Validating ingress rules in {config}...")
    try:
        validate_ingress(binary, config)
        print("  -> Configuration is valid (OK)")
    except Exception as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1

    test_cases = [
        # Rule #0: exact /alexa and /alexa?x=1 on alexa.bynaassom.com.br
        TestCase("https://alexa.bynaassom.com.br/alexa", 0, "http://127.0.0.1:8080"),
        TestCase("https://alexa.bynaassom.com.br/alexa?x=1", 0, "http://127.0.0.1:8080"),
        # Rule #1: fallbacks and other paths / hostnames
        TestCase("https://alexa.bynaassom.com.br/", 1, "http_status:404"),
        TestCase("https://alexa.bynaassom.com.br/api", 1, "http_status:404"),
        TestCase("https://alexa.bynaassom.com.br/alexa/", 1, "http_status:404"),
        TestCase("https://alexa.bynaassom.com.br/alexa-extra", 1, "http_status:404"),
        TestCase("https://alexa.bynaassom.com.br/alexa/api", 1, "http_status:404"),
        TestCase("https://other.example.com/alexa", 1, "http_status:404"),
    ]

    print(f"Testing {len(test_cases)} URL ingress rules...")
    failed = 0
    for case in test_cases:
        try:
            rule_idx, service = evaluate_url(binary, config, case.url)
            if rule_idx != case.expected_rule or service != case.expected_service:
                print(
                    f"  [FAIL] {case.url}\n"
                    f"         Expected: rule #{case.expected_rule} ({case.expected_service})\n"
                    f"         Got:      rule #{rule_idx} ({service})",
                    file=sys.stderr,
                )
                failed += 1
            else:
                print(f"  [PASS] {case.url} -> rule #{rule_idx} ({service})")
        except Exception as exc:
            print(f"  [FAIL] {case.url}: {exc}", file=sys.stderr)
            failed += 1

    if failed > 0:
        print(f"\nResult: FAILED ({failed}/{len(test_cases)} assertions failed)", file=sys.stderr)
        return 1

    print(f"\nResult: OK (all {len(test_cases)} assertions passed)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
