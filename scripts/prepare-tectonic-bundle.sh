#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 <tectonic-executable> <output-bundle-dir>" >&2
  exit 2
fi

tectonic_executable=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
output_bundle_input=$2
repo_root=$(cd "$(dirname "$0")/.." && pwd)

if [[ -e "$output_bundle_input" ]]; then
  echo "output bundle already exists: $output_bundle_input" >&2
  exit 2
fi
mkdir -p "$(dirname "$output_bundle_input")"
output_bundle=$(cd "$(dirname "$output_bundle_input")" && pwd)/$(basename "$output_bundle_input")

build_root=$(mktemp -d)
cleanup() {
  rm -rf "$build_root"
}
trap cleanup EXIT

mkdir -p "$build_root/source" "$build_root/cache"
cp "$repo_root/resources/tectonic/warmup.tex" "$build_root/source/"
cp "$repo_root/resources/tectonic/warmup-official.tex" "$build_root/source/"
cp "$repo_root/resources/tectonic/warmup.bib" "$build_root/source/"

export TECTONIC_CACHE_DIR="$build_root/cache"
(
  cd "$build_root/source"
  "$tectonic_executable" -X compile warmup.tex
  "$tectonic_executable" -X compile warmup-official.tex
)

data_root="$build_root/cache/bundles/data"
bundle_source=$(find "$data_root" -mindepth 1 -maxdepth 1 -type d -print -quit)
if [[ -z "$bundle_source" ]]; then
  echo "unable to locate Tectonic's resolved bundle cache" >&2
  exit 1
fi

mkdir -p "$output_bundle"
cp -R "$bundle_source/." "$output_bundle/"

# A directory bundle's fingerprint identifies the exact curated contents, not
# the much larger upstream bundle from which these files were resolved.
bundle_digest=$(
  cd "$output_bundle"
  find . -type f -print0 \
    | LC_ALL=C sort -z \
    | xargs -0 sha256sum \
    | sha256sum \
    | cut -d ' ' -f 1
)
printf '%s\n' "$bundle_digest" > "$output_bundle/SHA256SUM"

bundle_argument=$output_bundle
case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*)
    bundle_argument="file:///$(cygpath -m "$output_bundle")"
    ;;
esac

# Prove that the curated directory bundle is complete without consulting the
# network cache populated above.
mkdir -p "$build_root/empty-cache"
export TECTONIC_CACHE_DIR="$build_root/empty-cache"
(
  cd "$build_root/source"
  rm -f warmup.aux warmup.bbl warmup.blg warmup.log warmup.pdf warmup.toc \
    warmup-official.aux warmup-official.log warmup-official.pdf
  "$tectonic_executable" -X compile \
    --bundle "$bundle_argument" \
    --only-cached \
    --untrusted \
    warmup.tex
  "$tectonic_executable" -X compile \
    --bundle "$bundle_argument" \
    --only-cached \
    --untrusted \
    warmup-official.tex
)

echo "prepared offline Tectonic bundle: $output_bundle"
