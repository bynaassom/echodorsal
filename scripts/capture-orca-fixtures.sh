#!/bin/sh
# Capture only allowlisted, sanitized fields. Raw responses never touch disk.
set -eu
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec python3 "$SCRIPT_DIR/capture_orca_fixtures.py" "$@"
