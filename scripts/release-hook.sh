#!/usr/bin/env bash
set -euo pipefail

version="${1:?missing version}"

root_dir="$(git rev-parse --show-toplevel)"
node_dir="$root_dir/crates/cryosnap-node"

if ! command -v npm >/dev/null 2>&1; then
  echo "npm not found; cannot bump cryosnap-node version" >&2
  exit 1
fi

cd "$node_dir"
current="$(node -p "require('./package.json').version")"
if [ "$current" = "$version" ]; then
  echo "cryosnap-node already at version $version; skipping npm version"
  exit 0
fi

npm version "$version" --no-git-tag-version
