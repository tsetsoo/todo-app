#!/usr/bin/env bash
set -euo pipefail

TODO_ROOT="${TODO_ROOT:-/opt/todo}"
TODO_RELEASE_BASE="${TODO_RELEASE_BASE:-https://github.com/tsetsoo/todo-app/releases/download/pi-latest}"
TODO_HEALTH_URL="${TODO_HEALTH_URL:-http://127.0.0.1:8080/api/sections}"
TODO_SYSTEMCTL="${TODO_SYSTEMCTL:-systemctl}"
KEEP_RELEASES="${KEEP_RELEASES:-3}"

mkdir -p "$TODO_ROOT/releases" "$TODO_ROOT/data"

remote_sha="$(curl -fsSL "$TODO_RELEASE_BASE/SHA" | tr -d '[:space:]')"
if [[ -z "$remote_sha" || ! "$remote_sha" =~ ^[0-9a-f]{7,40}$ ]]; then
  echo "invalid remote SHA: '$remote_sha'" >&2
  exit 1
fi

local_sha=""
if [[ -f "$TODO_ROOT/current/SHA" ]]; then
  local_sha="$(tr -d '[:space:]' < "$TODO_ROOT/current/SHA")"
fi

if [[ "$remote_sha" == "$local_sha" ]]; then
  echo "already at $remote_sha"
  exit 0
fi

prev=""
if [[ -L "$TODO_ROOT/current" ]]; then
  prev="$(readlink -f "$TODO_ROOT/current" || true)"
fi

workdir="$(mktemp -d)"
trap 'rm -rf "$workdir"' EXIT

curl -fsSL "$TODO_RELEASE_BASE/todo-pi.tar.gz" -o "$workdir/todo-pi.tar.gz"
dest="$TODO_ROOT/releases/$remote_sha"
rm -rf "$dest"
mkdir -p "$dest"
tar -C "$dest" -xzf "$workdir/todo-pi.tar.gz"

if [[ ! -x "$dest/todo-server" || ! -f "$dest/frontend-dist/index.html" ]]; then
  echo "release payload incomplete" >&2
  rm -rf "$dest"
  exit 1
fi
printf '%s\n' "$remote_sha" > "$dest/SHA"
chmod +x "$dest/todo-server"

ln -sfn "$dest" "$TODO_ROOT/current"
"$TODO_SYSTEMCTL" restart todo

ok=0
for _ in 1 2 3 4 5 6 7 8 9 10; do
  if curl -fsS "$TODO_HEALTH_URL" >/dev/null; then
    ok=1
    break
  fi
  sleep 1
done

if [[ "$ok" -ne 1 ]]; then
  echo "health check failed; rolling back" >&2
  if [[ -n "$prev" && -d "$prev" ]]; then
    ln -sfn "$prev" "$TODO_ROOT/current"
    "$TODO_SYSTEMCTL" restart todo || true
  fi
  exit 1
fi

# prune old releases (keep newest KEEP_RELEASES by mtime)
# shellcheck disable=SC2012
ls -1dt "$TODO_ROOT/releases"/* 2>/dev/null | tail -n +"$((KEEP_RELEASES + 1))" | while read -r old; do
  # never delete the live current target
  [[ "$(readlink -f "$TODO_ROOT/current")" == "$(readlink -f "$old")" ]] && continue
  rm -rf "$old"
done

echo "deployed $remote_sha"
