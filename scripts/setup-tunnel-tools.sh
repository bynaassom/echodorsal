#!/usr/bin/env bash
# scripts/setup-tunnel-tools.sh — macOS arm64 setup wholly in repo .cache (F0-04)
#
# Downloads official cloudflared 2026.10.3 darwin arm64 archive from GitHub release,
# verifies SHA-256 checksum, extracts binary to .cache/bin, and compiles
# scripts/tunnel-keychain.swift to .cache/bin/tunnel-keychain with swiftc.

set -euo pipefail

OS="$(uname -s)"
ARCH="$(uname -m)"
if [[ "${OS}" != "Darwin" || "${ARCH}" != "arm64" ]]; then
  echo "Error: This script is intended for macOS arm64 (Darwin arm64), detected ${OS} ${ARCH}." >&2
  exit 1
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CACHE_DIR="${REPO_ROOT}/.cache"
BIN_DIR="${CACHE_DIR}/bin"
SWIFT_MODULE_CACHE_DIR="${CACHE_DIR}/swift-module-cache"
CLANG_MODULE_CACHE_DIR="${CACHE_DIR}/clang-module-cache"
TMP_DIR="${CACHE_DIR}/tmp"
KEYCHAIN_SRC="${REPO_ROOT}/scripts/tunnel-keychain.swift"
KEYCHAIN_BIN="${BIN_DIR}/tunnel-keychain"
CLOUDFLARED_BIN="${BIN_DIR}/cloudflared"

CLOUDFLARED_VERSION="2026.10.3"
CLOUDFLARED_URL="https://github.com/cloudflare/cloudflared/releases/download/${CLOUDFLARED_VERSION}/cloudflared-darwin-arm64.tgz"
EXPECTED_SHA256="1165d14b6f6360a3b8c283d7d1100024d6ec7cd9bc9e32519e96150645f3ec55"
TGZ_PATH="${CACHE_DIR}/cloudflared-darwin-arm64.tgz"

mkdir -p "${BIN_DIR}" "${SWIFT_MODULE_CACHE_DIR}" "${CLANG_MODULE_CACHE_DIR}" "${TMP_DIR}"

for tool in curl shasum tar swiftc; do
  if ! command -v "${tool}" >/dev/null 2>&1; then
    echo "Error: Required tool '${tool}' is not installed or not available in PATH." >&2
    exit 1
  fi
done

if [[ ! -f "${KEYCHAIN_SRC}" ]]; then
  echo "Error: Source file ${KEYCHAIN_SRC} not found." >&2
  exit 1
fi

# 1. Download and verify official cloudflared archive
NEED_DOWNLOAD=1
if [[ -f "${TGZ_PATH}" ]]; then
  EXISTING_SHA256="$(shasum -a 256 "${TGZ_PATH}" | awk '{print $1}')"
  if [[ "${EXISTING_SHA256}" == "${EXPECTED_SHA256}" ]]; then
    echo "Archive ${TGZ_PATH} already exists and matches expected SHA-256."
    NEED_DOWNLOAD=0
  else
    echo "Existing archive SHA-256 mismatch (${EXISTING_SHA256}). Re-downloading..."
  fi
fi

if [[ "${NEED_DOWNLOAD}" -eq 1 ]]; then
  echo "Downloading cloudflared ${CLOUDFLARED_VERSION} from GitHub releases..."
  TMP_TGZ="$(mktemp "${CACHE_DIR}/cloudflared.XXXXXX.tgz")"
  if ! curl -fSL "${CLOUDFLARED_URL}" -o "${TMP_TGZ}"; then
    echo "Error: Failed to download cloudflared archive from ${CLOUDFLARED_URL}." >&2
    rm -f "${TMP_TGZ}"
    exit 1
  fi

  DOWNLOAD_SHA256="$(shasum -a 256 "${TMP_TGZ}" | awk '{print $1}')"
  if [[ "${DOWNLOAD_SHA256}" != "${EXPECTED_SHA256}" ]]; then
    echo "Error: SHA-256 checksum mismatch for downloaded cloudflared archive." >&2
    echo "Expected: ${EXPECTED_SHA256}" >&2
    echo "Actual:   ${DOWNLOAD_SHA256}" >&2
    rm -f "${TMP_TGZ}"
    exit 1
  fi

  mv "${TMP_TGZ}" "${TGZ_PATH}"
fi

# 2. Extract cloudflared binary to .cache/bin
echo "Extracting cloudflared to ${CLOUDFLARED_BIN}..."
tar -xzf "${TGZ_PATH}" -C "${BIN_DIR}" cloudflared
chmod +x "${CLOUDFLARED_BIN}"

# 3. Compile scripts/tunnel-keychain.swift to .cache/bin/tunnel-keychain
echo "Compiling ${KEYCHAIN_SRC} to ${KEYCHAIN_BIN}..."
mkdir -p "${SWIFT_MODULE_CACHE_DIR}" "${CLANG_MODULE_CACHE_DIR}" "${TMP_DIR}"
export SWIFT_MODULE_CACHE_PATH="${SWIFT_MODULE_CACHE_DIR}"
export CLANG_MODULE_CACHE_PATH="${CLANG_MODULE_CACHE_DIR}"
export TMPDIR="${TMP_DIR}"

COMPILE_LOG="$(mktemp "${TMP_DIR}/swift-compile.XXXXXX.log")"
cleanup() {
  rm -f "${COMPILE_LOG}"
}
trap cleanup EXIT

if swiftc -O -module-cache-path "${SWIFT_MODULE_CACHE_DIR}" -o "${KEYCHAIN_BIN}" "${KEYCHAIN_SRC}" >"${COMPILE_LOG}" 2>&1; then
  echo "Successfully compiled tunnel-keychain with default SDK."
else
  echo "Default SDK compilation failed, attempting fallback to MacOSX26.5 SDK..."
  FALLBACK_SDK=""
  if xcrun --sdk macosx26.5 --show-sdk-path >/dev/null 2>&1; then
    FALLBACK_SDK="$(xcrun --sdk macosx26.5 --show-sdk-path 2>/dev/null)"
  fi

  if [[ -z "${FALLBACK_SDK}" || ! -d "${FALLBACK_SDK}" ]]; then
    if [[ -d "/Library/Developer/CommandLineTools/SDKs/MacOSX26.5.sdk" ]]; then
      FALLBACK_SDK="/Library/Developer/CommandLineTools/SDKs/MacOSX26.5.sdk"
    elif [[ -d "/Library/Developer/CommandLineTools/SDKs/MacOSX26.sdk" ]]; then
      FALLBACK_SDK="/Library/Developer/CommandLineTools/SDKs/MacOSX26.sdk"
    fi
  fi

  if [[ -n "${FALLBACK_SDK}" && -d "${FALLBACK_SDK}" ]]; then
    echo "Using fallback SDK: ${FALLBACK_SDK}"
    if swiftc -sdk "${FALLBACK_SDK}" -O -module-cache-path "${SWIFT_MODULE_CACHE_DIR}" -o "${KEYCHAIN_BIN}" "${KEYCHAIN_SRC}" >"${COMPILE_LOG}" 2>&1; then
      echo "Successfully compiled tunnel-keychain with MacOSX26.5 SDK fallback."
    else
      echo "Error: Compilation failed with fallback SDK (${FALLBACK_SDK}):" >&2
      cat "${COMPILE_LOG}" >&2
      exit 1
    fi
  else
    echo "Error: Default SDK compilation failed and MacOSX26.5 SDK fallback was not found." >&2
    cat "${COMPILE_LOG}" >&2
    exit 1
  fi
fi

chmod +x "${KEYCHAIN_BIN}"
echo "Setup complete. Binaries available at:"
echo "  - ${CLOUDFLARED_BIN}"
echo "  - ${KEYCHAIN_BIN}"
