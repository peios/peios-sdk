#!/usr/bin/env bash
# Verify the checked SDK headers against explicit build outputs.
set -euo pipefail
sdk_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
exec python3 "$sdk_root/tools/verify-abi.py" libpeios \
    --shared "${PEIOS_LIBRARY:-$sdk_root/bazel-bin/libpeios/libpeios.so}" \
    --static "${PEIOS_STATIC_LIBRARY:-$sdk_root/bazel-bin/libpeios/static/libpeios.a}" "$@"
