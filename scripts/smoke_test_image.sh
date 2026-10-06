#!/usr/bin/env bash
# Smoke-test the helper image: both binaries start (so every shared library
# they link, OpenSSL included, resolves in the distroless runtime) and the
# image runs as the distroless nonroot user.
#
# Usage: scripts/smoke_test_image.sh <image>
set -euo pipefail

image="${1:?usage: $0 <image>}"

# The default command prints a static usage report and exits 0.
output="$(docker run --rm "${image}")"
grep -q "TOTAL ESTIMATED USAGE" <<< "${output}"

# Without a key, test_api must start, refuse to run and exit 1. A missing
# shared library would exit 127 from the dynamic loader instead.
set +e
api_output="$(docker run --rm "${image}" test_api 2>&1)"
api_status=$?
set -e
if [[ "${api_status}" -ne 1 ]] || ! grep -q "ANTHROPIC_API_KEY not set" <<< "${api_output}"; then
  echo "test_api exited ${api_status}, expected 1 with a missing-key message:" >&2
  echo "${api_output}" >&2
  exit 1
fi

image_user="$(docker image inspect --format '{{ .Config.User }}' "${image}")"
if [[ "${image_user}" != "65532:65532" ]]; then
  echo "Image runs as '${image_user}', expected the distroless nonroot user 65532:65532" >&2
  exit 1
fi

echo "Smoke test passed for ${image}"
