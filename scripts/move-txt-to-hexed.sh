#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TARGET_DIR="$REPO_ROOT/hexed"

mkdir -p "$TARGET_DIR"

# enable nullglob if supported
if shopt -q nullglob 2>/dev/null; then
  shopt -s nullglob
fi

files=("$REPO_ROOT"/*.txt)
if [ ${#files[@]} -eq 0 ] || [ -z "${files[0]}" ]; then
  echo "No .txt files found in $REPO_ROOT"
  exit 0
fi

for f in "${files[@]}"; do
  mv -v -- "$f" "$TARGET_DIR/"
done
