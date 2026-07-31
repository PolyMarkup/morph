#!/bin/sh
set -eu

demo_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_dir=$(CDPATH= cd -- "$demo_dir/.." && pwd)
source_file="$demo_dir/source/all-elements.html"
generated_dir="$demo_dir/generated"
morph_bin="$repo_dir/target/debug/morph"
mode=${1:-write}

if [ "$mode" != "write" ] && [ "$mode" != "--check" ]; then
  echo "usage: ./generate.sh [--check]" >&2
  exit 2
fi

cargo build --quiet --locked --manifest-path "$repo_dir/Cargo.toml" -p morph-cli

if [ "$mode" = "--check" ]; then
  output_dir=$(mktemp -d "${TMPDIR:-/tmp}/morph-static-demo.XXXXXX")
  trap 'rm -rf "$output_dir"' EXIT HUP INT TERM
else
  output_dir="$generated_dir"
  mkdir -p "$output_dir"
fi

generate() {
  extension=$1
  "$morph_bin" "$source_file" "$output_dir/all-elements.$extension"
}

generate md
generate adoc
generate rst
generate typ
generate tex
generate dj
generate org
generate textile
generate html
generate dbk

if [ "$mode" = "--check" ]; then
  status=0
  for generated_file in "$output_dir"/*; do
    checked_file="$generated_dir/$(basename "$generated_file")"
    if [ ! -f "$checked_file" ]; then
      echo "missing generated file: $checked_file" >&2
      status=1
    elif ! cmp -s "$generated_file" "$checked_file"; then
      echo "stale generated file: $checked_file" >&2
      status=1
    fi
  done
  if [ "$status" -ne 0 ]; then
    exit "$status"
  fi
  echo "Morph static demo is up to date (10 formats)."
else
  echo "Generated 10 formats in $generated_dir"
fi
