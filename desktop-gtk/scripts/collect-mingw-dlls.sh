#!/usr/bin/env bash
set -euo pipefail

target="$(cygpath -u "$1")"
dest="$(cygpath -u "$2")"
mkdir -p "$dest"

queue=("$target")
declare -A seen=()
while ((${#queue[@]})); do
  current="${queue[0]}"
  queue=("${queue[@]:1}")
  while IFS= read -r dll; do
    [[ -f "$dll" ]] || continue
    name="$(basename "$dll")"
    key="${name,,}"
    [[ -z "${seen[$key]:-}" ]] || continue
    seen[$key]=1
    cp -f "$dll" "$dest/$name"
    queue+=("$dll")
  done < <(ldd "$current" 2>/dev/null | awk '
    /=> \/mingw64\/bin\/.*\.dll/ { print $3 }
    /^[[:space:]]*\/mingw64\/bin\/.*\.dll/ { print $1 }
  ')
done

printf 'copied %d runtime DLLs\n' "${#seen[@]}"
