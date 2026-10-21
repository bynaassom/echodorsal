#!/usr/bin/env python3
"""Provision Cloudflare Tunnel and DNS record for Alexa skill (F0-04)."""

from __future__ import annotations

import argparse
import base64
import json
import os
from pathlib import Path
import re
import secrets
import subprocess
import sys
import urllib.error
import urllib.parse
import urllib.request
import uuid

REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_KEYCHAIN_BIN = REPO_ROOT / ".cache" / "bin" / "tunnel-keychain"

API_BASE = "https://api.cloudflare.com/client/v4"
HOSTNAME = "alexa.bynaassom.com.br"
TUNNEL_NAME = "echodorsal"
TIMEOUT_SECONDS = 15

HEX_32_REGEX = re.compile(r"^[0-9a-fA-F]{32}$")
UUID_REGEX = re.compile(
    r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$"
)

STRIPPED_ENV_VARS = (
    "TUNNEL_TOKEN",
    "TUNNEL_TOKEN_FILE",
    "TUNNEL_CRED_CONTENTS",
    "TUNNEL_CRED_FILE",
    "CLOUDFLARE_API_TOKEN",
    "CF_API_TOKEN",
)


def get_clean_env() -> dict[str, str]:
    env = dict(os.environ)
    for var in STRIPPED_ENV_VARS:
        env.pop(var, None)
    return env


def resolve_path(path: Path) -> Path:
    if path.is_absolute():
        return path.resolve()
    candidate = (REPO_ROOT / path).resolve()
    if candidate.exists():
        return candidate
    return path.resolve()


def fail_generic(message: str) -> None:
    print(f"Error: {message}", file=sys.stderr)
    sys.exit(1)


def fail_post_create(tunnel_id: str, message: str, recovery: str) -> None:
    print(f"Error: {message}", file=sys.stderr)
    print("Cloudflare may have created the tunnel; do not auto-retry.", file=sys.stderr)
    print("Pending state is kept in Keychain.", file=sys.stderr)
    print(f"Tunnel UUID: {tunnel_id}", file=sys.stderr)
    print(f"Recovery instructions:\n{recovery}", file=sys.stderr)
    sys.exit(1)


def fail_create_tunnel(account_id: str, tunnel_name: str, message: str) -> None:
    print(f"Error: {message}", file=sys.stderr)
    print("Cloudflare may have created the tunnel; do not auto-retry.", file=sys.stderr)
    print(f"Tunnel name: {tunnel_name}", file=sys.stderr)
    print("Pending state is kept in Keychain.", file=sys.stderr)
    print(
        "Recovery instructions:\n"
        "1. Check if the tunnel was created in Cloudflare:\n"
        f"   GET https://api.cloudflare.com/client/v4/accounts/{account_id}/cfd_tunnel?name={tunnel_name}\n"
        "2. If created, delete the remote tunnel.\n"
        "3. Once resolved, delete pending state from Keychain (`tunnel-keychain delete-pending`) before rerunning.",
        file=sys.stderr,
    )
    sys.exit(1)


def fetch_api_token(keychain_bin: Path) -> str:
    if not keychain_bin.is_file() or not (keychain_bin.stat().st_mode & 0o111):
        fail_generic("Keychain helper not found or not executable.")

    try:
        res = subprocess.run(
            [str(keychain_bin), "get-api-token"],
            capture_output=True,
            check=False,
            env=get_clean_env(),
        )
        if res.returncode != 0 or not res.stdout:
            fail_generic("Failed to retrieve API token from Keychain.")
        token = res.stdout.decode("utf-8").strip()
        if not token:
            fail_generic("Empty API token retrieved from Keychain.")
        return token
    except SystemExit:
        raise
    except Exception:
        fail_generic("Failed to execute Keychain helper.")


def api_request(
    url: str,
    method: str,
    token: str,
    payload: dict | None = None,
) -> dict:
    headers = {
        "Authorization": f"Bearer {token}",
        "Accept": "application/json",
        "User-Agent": "echodorsal-provision/1.0",
    }
    data: bytes | None = None
    if payload is not None:
        headers["Content-Type"] = "application/json"
        data = json.dumps(payload).encode("utf-8")

    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=TIMEOUT_SECONDS) as resp:
            raw_body = resp.read()
    except urllib.error.HTTPError as exc:
        raise RuntimeError(f"HTTP {exc.code}") from None
    except (urllib.error.URLError, TimeoutError):
        raise RuntimeError("Network error or request timeout") from None
    except Exception:
        raise RuntimeError("API request failed") from None

    try:
        body = json.loads(raw_body.decode("utf-8"))
    except Exception:
        raise RuntimeError("Failed to parse API response as JSON") from None

    if not isinstance(body, dict):
        raise RuntimeError("Invalid API response format")

    return body


def check_pending_operation(keychain_bin: Path) -> None:
    if not keychain_bin.is_file() or not (keychain_bin.stat().st_mode & 0o111):
        fail_generic("Keychain helper not found or not executable.")

    try:
        res = subprocess.run(
            [str(keychain_bin), "get-pending"],
            capture_output=True,
            check=False,
            env=get_clean_env(),
        )
        if res.returncode == 0:
            print(
                "Error: A previous tunnel provisioning operation is pending in Keychain.\n"
                "Cloudflare may have created the tunnel; do not auto-retry (never issue another POST).\n"
                "Manual recovery instructions:\n"
                "1. Check Cloudflare dashboard or API for partially created tunnels or DNS records.\n"
                "2. Clean up remote resources if needed.\n"
                "3. Once resolved, delete pending state from Keychain (`tunnel-keychain delete-pending`) before rerunning.",
                file=sys.stderr,
            )
            sys.exit(1)
        elif res.returncode == 2:
            return
        else:
            fail_generic("Failed to verify pending state in Keychain.")
    except SystemExit:
        raise
    except Exception:
        fail_generic("Failed to execute Keychain helper.")


def store_pending_operation(
    keychain_bin: Path,
    account_id: str,
    zone_id: str,
    tunnel_name: str,
    tunnel_secret: str,
) -> None:
    pending = {
        "account_id": account_id,
        "zone_id": zone_id,
        "hostname": HOSTNAME,
        "tunnel_name": tunnel_name,
        "tunnel_secret": tunnel_secret,
    }
    pending_bytes = json.dumps(pending).encode("utf-8")

    try:
        res = subprocess.run(
            [str(keychain_bin), "put-pending"],
            input=pending_bytes,
            capture_output=True,
            check=False,
            env=get_clean_env(),
        )
        if res.returncode != 0:
            fail_generic("Failed to store pending operation in Keychain.")
    except SystemExit:
        raise
    except Exception:
        fail_generic("Failed to execute Keychain helper to store pending operation.")


def delete_pending_operation(keychain_bin: Path) -> None:
    try:
        res = subprocess.run(
            [str(keychain_bin), "delete-pending"],
            capture_output=True,
            check=False,
            env=get_clean_env(),
        )
        if res.returncode not in (0, 2):
            fail_generic("Failed to delete pending operation state from Keychain.")
    except SystemExit:
        raise
    except Exception:
        fail_generic("Failed to execute Keychain helper to delete pending state.")


def check_existing_keychain_credentials(keychain_bin: Path) -> None:
    if not keychain_bin.is_file() or not (keychain_bin.stat().st_mode & 0o111):
        fail_generic("Keychain helper not found or not executable.")

    try:
        res = subprocess.run(
            [str(keychain_bin), "get"],
            capture_output=True,
            check=False,
            env=get_clean_env(),
        )
        if res.returncode == 0:
            fail_generic(
                "Tunnel credentials already exist in Keychain. Refusing to overwrite."
            )
        elif res.returncode == 2:
            return
        else:
            fail_generic("Failed to verify existing Keychain credentials.")
    except SystemExit:
        raise
    except Exception:
        fail_generic("Failed to execute Keychain helper.")


def check_existing_dns_record(zone_id: str, token: str) -> None:
    query = urllib.parse.urlencode({"name": HOSTNAME})
    url = f"{API_BASE}/zones/{zone_id}/dns_records?{query}"
    try:
        resp = api_request(url, "GET", token)
    except RuntimeError as exc:
        fail_generic(f"Failed to query existing DNS records ({exc}).")

    if not resp.get("success"):
        fail_generic("Cloudflare API returned error when checking DNS records.")

    result = resp.get("result")
    if not isinstance(result, list):
        fail_generic("Unexpected response when checking DNS records.")

    if len(result) > 0:
        fail_generic(
            f"DNS record for {HOSTNAME} already exists. Refusing to overwrite."
        )


def create_tunnel(
    account_id: str,
    token: str,
    tunnel_name: str,
    tunnel_secret: str,
) -> tuple[str, str]:
    url = f"{API_BASE}/accounts/{account_id}/cfd_tunnel"
    payload = {
        "name": tunnel_name,
        "tunnel_secret": tunnel_secret,
        "config_src": "local",
    }

    try:
        resp = api_request(url, "POST", token, payload=payload)
    except RuntimeError as exc:
        fail_create_tunnel(account_id, tunnel_name, f"Failed to create tunnel ({exc}).")

    if not resp.get("success"):
        fail_create_tunnel(account_id, tunnel_name, "Cloudflare API returned failure when creating tunnel.")

    result = resp.get("result")
    if not isinstance(result, dict):
        fail_create_tunnel(account_id, tunnel_name, "Unexpected response format when creating tunnel.")

    tunnel_id = result.get("id")
    account_tag = result.get("account_tag")
    config_src = result.get("config_src")

    if not tunnel_id or not isinstance(tunnel_id, str) or not UUID_REGEX.fullmatch(tunnel_id):
        fail_create_tunnel(account_id, tunnel_name, "Invalid or missing tunnel UUID in API response.")

    if not account_tag or not isinstance(account_tag, str) or account_tag.lower() != account_id.lower():
        fail_create_tunnel(account_tag or account_id, tunnel_name, "Account tag mismatch in tunnel creation response.")

    if config_src != "local":
        fail_create_tunnel(account_id, tunnel_name, "Unexpected config_src in tunnel creation response.")

    return tunnel_id, account_tag


def save_credentials_to_keychain(
    keychain_bin: Path,
    account_tag: str,
    tunnel_id: str,
    tunnel_secret: str,
    account_id: str,
) -> None:
    creds = {
        "AccountTag": account_tag,
        "TunnelSecret": tunnel_secret,
        "TunnelID": tunnel_id,
    }
    creds_bytes = json.dumps(creds).encode("utf-8")

    try:
        res = subprocess.run(
            [str(keychain_bin), "put-new"],
            input=creds_bytes,
            capture_output=True,
            check=False,
            env=get_clean_env(),
        )
        if res.returncode != 0:
            fail_post_create(
                tunnel_id,
                "Failed to store tunnel credentials in Keychain.",
                (
                    "The tunnel was created in Cloudflare, but credentials could not be stored in Keychain.\n"
                    "Do not rerun blindly. To recover:\n"
                    f"1. Delete the created tunnel from Cloudflare:\n"
                    f"   DELETE https://api.cloudflare.com/client/v4/accounts/{account_id}/cfd_tunnel/{tunnel_id}\n"
                    "2. Check Keychain permissions and rerun provision-tunnel.py.\n"
                    "3. Once resolved, delete pending state (`tunnel-keychain delete-pending`)."
                ),
            )
    except SystemExit:
        raise
    except Exception:
        fail_post_create(
            tunnel_id,
            "Exception occurred while saving credentials to Keychain.",
            (
                f"1. Delete the created tunnel from Cloudflare:\n"
                f"   DELETE https://api.cloudflare.com/client/v4/accounts/{account_id}/cfd_tunnel/{tunnel_id}\n"
                "2. Check Keychain permissions and rerun provision-tunnel.py.\n"
                "3. Once resolved, delete pending state (`tunnel-keychain delete-pending`)."
            ),
        )


def create_dns_record(
    zone_id: str,
    tunnel_id: str,
    token: str,
) -> None:
    url = f"{API_BASE}/zones/{zone_id}/dns_records"
    payload = {
        "type": "CNAME",
        "name": HOSTNAME,
        "content": f"{tunnel_id}.cfargotunnel.com",
        "proxied": True,
        "ttl": 1,
    }

    try:
        resp = api_request(url, "POST", token, payload=payload)
    except RuntimeError as exc:
        fail_post_create(
            tunnel_id,
            f"Failed to create DNS CNAME record ({exc}).",
            (
                "The tunnel was created and credentials are saved in Keychain, but DNS creation failed.\n"
                "Do not delete the Keychain credentials. To recover:\n"
                f"1. Manually add CNAME record in Cloudflare dashboard for zone {zone_id}:\n"
                f"   Type: CNAME | Name: {HOSTNAME} | Target: {tunnel_id}.cfargotunnel.com | Proxy: Proxied\n"
                "2. Or delete the tunnel from Cloudflare and Keychain before reprovisioning.\n"
                "3. Once resolved, delete pending state (`tunnel-keychain delete-pending`)."
            ),
        )

    if not resp.get("success"):
        fail_post_create(
            tunnel_id,
            "Cloudflare API reported error while creating DNS CNAME record.",
            (
                "The tunnel was created and credentials are saved in Keychain, but DNS creation failed.\n"
                f"1. Manually add CNAME record in Cloudflare dashboard for zone {zone_id}:\n"
                f"   Type: CNAME | Name: {HOSTNAME} | Target: {tunnel_id}.cfargotunnel.com | Proxy: Proxied\n"
                "2. Or delete the tunnel from Cloudflare and Keychain before reprovisioning.\n"
                "3. Once resolved, delete pending state (`tunnel-keychain delete-pending`)."
            ),
        )


def main() -> int:
    for var in STRIPPED_ENV_VARS:
        os.environ.pop(var, None)

    parser = argparse.ArgumentParser(
        description="Provision Cloudflare Tunnel and DNS CNAME record for Alexa skill."
    )
    parser.add_argument(
        "--account-id",
        required=True,
        help="Cloudflare Account ID (32 hex characters)",
    )
    parser.add_argument(
        "--zone-id",
        required=True,
        help="Cloudflare Zone ID (32 hex characters)",
    )
    parser.add_argument(
        "--keychain-bin",
        type=Path,
        default=DEFAULT_KEYCHAIN_BIN,
        help="Path to tunnel-keychain helper binary (default: .cache/bin/tunnel-keychain)",
    )
    args = parser.parse_args()

    account_id = args.account_id.strip()
    zone_id = args.zone_id.strip()

    if not HEX_32_REGEX.fullmatch(account_id):
        fail_generic("Invalid --account-id. Must be exactly 32 hex characters.")
    if not HEX_32_REGEX.fullmatch(zone_id):
        fail_generic("Invalid --zone-id. Must be exactly 32 hex characters.")

    keychain_bin = resolve_path(args.keychain_bin)

    check_pending_operation(keychain_bin)
    token = fetch_api_token(keychain_bin)

    check_existing_keychain_credentials(keychain_bin)
    check_existing_dns_record(zone_id, token)

    op_uuid = str(uuid.uuid4())
    tunnel_name = f"echodorsal-{op_uuid}"
    tunnel_secret = base64.b64encode(secrets.token_bytes(32)).decode("ascii")

    store_pending_operation(
        keychain_bin, account_id, zone_id, tunnel_name, tunnel_secret
    )

    tunnel_id, account_tag = create_tunnel(
        account_id, token, tunnel_name, tunnel_secret
    )

    save_credentials_to_keychain(
        keychain_bin, account_tag, tunnel_id, tunnel_secret, account_id
    )

    create_dns_record(zone_id, tunnel_id, token)

    delete_pending_operation(keychain_bin)

    print(f"Tunnel UUID: {tunnel_id}")
    print("Status: provisioned successfully")
    return 0


if __name__ == "__main__":
    sys.exit(main())
