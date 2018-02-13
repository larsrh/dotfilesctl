#!/bin/bash

set -e

PWD="$(readlink -f "$(dirname "$0")")"
DOTFILESCTL="$PWD/target/debug/dotfilesctl"

cargo build

init_script()
{
  "$DOTFILESCTL" completions bash
  echo "alias dotfilesctl='$DOTFILESCTL'"
}

bash --init-file <(init_script)
