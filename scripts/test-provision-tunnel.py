#!/usr/bin/env python3
"""Targeted unit tests for scripts/provision-tunnel.py with in-memory mocks."""

from __future__ import annotations

import contextlib
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import sys
import unittest
from unittest.mock import patch

# Dynamically load scripts/provision-tunnel.py without altering production files
SCRIPT_PATH = Path(__file__).resolve().parent / "provision-tunnel.py"
spec = importlib.util.spec_from_file_location("provision_tunnel", SCRIPT_PATH)
if spec is None or spec.loader is None:
    raise ImportError(f"Cannot load spec for {SCRIPT_PATH}")
provision_tunnel = importlib.util.module_from_spec(spec)
spec.loader.exec_module(provision_tunnel)


class MockKeychain:
    """In-memory simulation of tunnel-keychain swift helper."""

    def __init__(self, api_token: str = "cf-api-token-test-12345") -> None:
        self.api_token: str | None = api_token
        self.credentials: dict | None = None
        self.pending: dict | None = None
        self.history: list[tuple[list[str], bytes | None]] = []

    def handle_subprocess_run(self, cmd: list[str], *args, **kwargs) -> subprocess.CompletedProcess:
        input_data = kwargs.get("input")
        self.history.append((list(cmd), input_data))
        subcommand = cmd[1] if len(cmd) > 1 else None

        if subcommand == "get-api-token":
            if self.api_token:
                return subprocess.CompletedProcess(
                    args=cmd, returncode=0, stdout=self.api_token.encode("utf-8"), stderr=b""
                )
            return subprocess.CompletedProcess(
                args=cmd, returncode=2, stdout=b"", stderr=b"error: item not found"
            )

        elif subcommand == "get-pending":
            if self.pending is not None:
                return subprocess.CompletedProcess(
                    args=cmd, returncode=0, stdout=json.dumps(self.pending).encode("utf-8"), stderr=b""
                )
            return subprocess.CompletedProcess(
                args=cmd, returncode=2, stdout=b"", stderr=b"error: item not found"
            )

        elif subcommand == "put-pending":
            if self.pending is not None:
                return subprocess.CompletedProcess(
                    args=cmd, returncode=3, stdout=b"", stderr=b"error: duplicate item"
                )
            self.pending = json.loads(input_data.decode("utf-8")) if input_data else {}
            return subprocess.CompletedProcess(
                args=cmd, returncode=0, stdout=b"", stderr=b""
            )

        elif subcommand == "delete-pending":
            if self.pending is not None:
                self.pending = None
                return subprocess.CompletedProcess(
                    args=cmd, returncode=0, stdout=b"", stderr=b""
                )
            return subprocess.CompletedProcess(
                args=cmd, returncode=2, stdout=b"", stderr=b"error: item not found"
            )

        elif subcommand == "get":
            if self.credentials is not None:
                return subprocess.CompletedProcess(
                    args=cmd, returncode=0, stdout=json.dumps(self.credentials).encode("utf-8"), stderr=b""
                )
            return subprocess.CompletedProcess(
                args=cmd, returncode=2, stdout=b"", stderr=b"error: item not found"
            )

        elif subcommand == "put-new":
            if self.credentials is not None:
                return subprocess.CompletedProcess(
                    args=cmd, returncode=3, stdout=b"", stderr=b"error: duplicate item"
                )
            self.credentials = json.loads(input_data.decode("utf-8")) if input_data else {}
            return subprocess.CompletedProcess(
                args=cmd, returncode=0, stdout=b"", stderr=b""
            )

        return subprocess.CompletedProcess(
            args=cmd, returncode=1, stdout=b"", stderr=b"unknown command"
        )


class MockCloudflareAPI:
    """In-memory simulation of Cloudflare API responses."""

    def __init__(self, dns_records: list[dict] | None = None, tunnel_error: Exception | None = None) -> None:
        self.dns_records: list[dict] = dns_records if dns_records is not None else []
        self.tunnel_error: Exception | None = tunnel_error
        self.calls: list[dict] = []

    def handle_request(self, url: str, method: str, token: str, payload: dict | None = None) -> dict:
        self.calls.append({
            "url": url,
            "method": method,
            "token": token,
            "payload": payload,
        })
        if "/dns_records?" in url and method == "GET":
            return {"success": True, "result": self.dns_records}
        elif "/cfd_tunnel" in url and method == "POST":
            if self.tunnel_error:
                raise self.tunnel_error
            # Extract account_id from URL: https://api.cloudflare.com/client/v4/accounts/{account_id}/cfd_tunnel
            parts = url.split("/accounts/")
            account_id = parts[1].split("/cfd_tunnel")[0] if len(parts) > 1 else "default-tag"
            return {
                "success": True,
                "result": {
                    "id": "11111111-2222-3333-4444-555555555555",
                    "account_tag": account_id,
                    "config_src": "local",
                },
            }
        elif "/dns_records" in url and method == "POST":
            return {
                "success": True,
                "result": {
                    "id": "dns-record-cname-123",
                },
            }
        return {"success": False, "error": f"Unhandled mock request: {method} {url}"}


class ProvisionTunnelTests(unittest.TestCase):
    ACCOUNT_ID = "0123456789abcdef0123456789abcdef"
    ZONE_ID = "abcdef0123456789abcdef0123456789"

    def run_main(self, keychain: MockKeychain, cf_api: MockCloudflareAPI) -> tuple[int, str, str]:
        stdout_buf = io.StringIO()
        stderr_buf = io.StringIO()
        cli_args = [
            "provision-tunnel.py",
            "--account-id",
            self.ACCOUNT_ID,
            "--zone-id",
            self.ZONE_ID,
            "--keychain-bin",
            sys.executable,
        ]

        with patch("sys.argv", cli_args), \
             patch.object(provision_tunnel.subprocess, "run", side_effect=keychain.handle_subprocess_run), \
             patch.object(provision_tunnel, "api_request", side_effect=cf_api.handle_request), \
             contextlib.redirect_stdout(stdout_buf), \
             contextlib.redirect_stderr(stderr_buf):
            try:
                ret = provision_tunnel.main()
                exit_code = ret
            except SystemExit as exc:
                exit_code = exc.code if isinstance(exc.code, int) else 1

        return exit_code, stdout_buf.getvalue(), stderr_buf.getvalue()

    def test_successful_provision(self) -> None:
        keychain = MockKeychain()
        cf_api = MockCloudflareAPI(dns_records=[])

        exit_code, out, err = self.run_main(keychain, cf_api)

        self.assertEqual(exit_code, 0, f"Expected 0 exit, got {exit_code}. Stderr: {err}")
        self.assertIn("Tunnel UUID: 11111111-2222-3333-4444-555555555555", out)
        self.assertIn("Status: provisioned successfully", out)

        # Assert final credentials stored in Keychain
        self.assertIsNotNone(keychain.credentials)
        self.assertEqual(keychain.credentials["AccountTag"], self.ACCOUNT_ID)
        self.assertEqual(keychain.credentials["TunnelID"], "11111111-2222-3333-4444-555555555555")
        self.assertTrue(bool(keychain.credentials.get("TunnelSecret")))

        # Assert pending was deleted
        self.assertIsNone(keychain.pending)

        # Assert no secrets in output
        self.assertNotIn(keychain.credentials["TunnelSecret"], out)
        self.assertNotIn(keychain.credentials["TunnelSecret"], err)

    def test_existing_credentials_refusal(self) -> None:
        keychain = MockKeychain()
        keychain.credentials = {
            "AccountTag": self.ACCOUNT_ID,
            "TunnelSecret": "existing-secret",
            "TunnelID": "00000000-0000-0000-0000-000000000000",
        }
        cf_api = MockCloudflareAPI(dns_records=[])

        exit_code, out, err = self.run_main(keychain, cf_api)

        self.assertEqual(exit_code, 1)
        self.assertIn("Tunnel credentials already exist in Keychain. Refusing to overwrite.", err)

        # No pending written and no remote POST calls made
        self.assertIsNone(keychain.pending)
        post_calls = [c for c in cf_api.calls if c["method"] == "POST"]
        self.assertEqual(len(post_calls), 0)

    def test_existing_dns_a_refusal(self) -> None:
        keychain = MockKeychain()
        # DNS query returns an existing A record for the hostname
        existing_records = [
            {"id": "dns-a-1", "type": "A", "name": "alexa.bynaassom.com.br", "content": "192.0.2.1"}
        ]
        cf_api = MockCloudflareAPI(dns_records=existing_records)

        exit_code, out, err = self.run_main(keychain, cf_api)

        self.assertEqual(exit_code, 1)
        self.assertIn("DNS record for alexa.bynaassom.com.br already exists. Refusing to overwrite.", err)

        # No pending written and no remote tunnel POST calls made
        self.assertIsNone(keychain.pending)
        post_calls = [c for c in cf_api.calls if c["method"] == "POST"]
        self.assertEqual(len(post_calls), 0)

    def test_ambiguous_post_preserving_pending_and_rerun_refusal(self) -> None:
        keychain = MockKeychain()
        # Simulate network timeout / ambiguous failure during POST /cfd_tunnel
        cf_api = MockCloudflareAPI(
            dns_records=[],
            tunnel_error=RuntimeError("Network error or request timeout"),
        )

        # --- First run (ambiguous failure) ---
        exit_code_1, out_1, err_1 = self.run_main(keychain, cf_api)

        self.assertEqual(exit_code_1, 1)
        self.assertIn("Cloudflare may have created the tunnel; do not auto-retry.", err_1)
        self.assertIn("Pending state is kept in Keychain.", err_1)

        # Pending state MUST be preserved in Keychain
        self.assertIsNotNone(keychain.pending)
        saved_secret = keychain.pending["tunnel_secret"]
        self.assertEqual(keychain.pending["account_id"], self.ACCOUNT_ID)
        self.assertEqual(keychain.pending["zone_id"], self.ZONE_ID)
        self.assertEqual(keychain.pending["hostname"], "alexa.bynaassom.com.br")
        self.assertTrue(keychain.pending["tunnel_name"].startswith("echodorsal-"))

        # Credentials should not be saved yet
        self.assertIsNone(keychain.credentials)

        # Verify no secret leaked in output
        self.assertNotIn(saved_secret, out_1)
        self.assertNotIn(saved_secret, err_1)

        # --- Second run (rerun refusal) ---
        # With the pending state intact in Keychain, rerun provision-tunnel
        cf_api.calls.clear()
        exit_code_2, out_2, err_2 = self.run_main(keychain, cf_api)

        self.assertEqual(exit_code_2, 1)
        self.assertIn("A previous tunnel provisioning operation is pending in Keychain.", err_2)
        self.assertIn("never issue another POST", err_2)

        # Must NOT have made any remote API calls (no GET, no POST)
        self.assertEqual(len(cf_api.calls), 0)

        # Pending state is still intact
        self.assertIsNotNone(keychain.pending)

        # No secret in output
        self.assertNotIn(saved_secret, out_2)
        self.assertNotIn(saved_secret, err_2)

    def test_no_secrets_in_output(self) -> None:
        keychain = MockKeychain()
        captured_secrets: list[str] = []

        # Hook put-pending to spy on the generated secret
        orig_handle = keychain.handle_subprocess_run

        def spy_handle(cmd, *args, **kwargs):
            if len(cmd) > 1 and cmd[1] == "put-pending":
                data = json.loads(kwargs.get("input", b"{}").decode("utf-8"))
                captured_secrets.append(data.get("tunnel_secret", ""))
            return orig_handle(cmd, *args, **kwargs)

        with patch.object(keychain, "handle_subprocess_run", side_effect=spy_handle):
            cf_api = MockCloudflareAPI(dns_records=[])
            exit_code, out, err = self.run_main(keychain, cf_api)

        self.assertEqual(exit_code, 0)
        self.assertEqual(len(captured_secrets), 1)
        secret = captured_secrets[0]
        self.assertTrue(len(secret) > 20)

        # Confirm secret is neither in stdout nor stderr
        self.assertNotIn(secret, out)
        self.assertNotIn(secret, err)


if __name__ == "__main__":
    unittest.main()
