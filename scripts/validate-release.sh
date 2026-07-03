#!/usr/bin/env bash

set -euo pipefail

if [ "$#" -lt 2 ]; then
  echo "Usage: $0 <tag> <release-title> [required-asset ...]" >&2
  exit 1
fi

tag="$1"
expected_title="$2"
shift 2

release_json="$(gh release view "$tag" --json name,assets)"
actual_title="$(printf '%s' "$release_json" | jq -r '.name')"

if [ "$actual_title" != "$expected_title" ]; then
  echo "ERROR: Release title mismatch for $tag" >&2
  echo "Expected: $expected_title" >&2
  echo "Actual:   $actual_title" >&2
  exit 1
fi

actual_assets="$(printf '%s' "$release_json" | jq -r '.assets[].name')"

for required_asset in "$@"; do
  if ! printf '%s\n' "$actual_assets" | grep -Fxq "$required_asset"; then
    echo "ERROR: Missing release asset: $required_asset" >&2
    exit 1
  fi
done

echo "Release $tag validated successfully"
echo "Title: $actual_title"
