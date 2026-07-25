#!/usr/bin/env bash
set -euo pipefail
OUT="${1:?usage: make_release.sh <outdir> <sha>}"
SHA="${2:?}"
mkdir -p "$OUT/payload/frontend-dist"
printf '%s\n' "$SHA" > "$OUT/SHA"
cat > "$OUT/payload/todo-server" <<'EOF'
#!/bin/sh
echo fake-server
EOF
chmod +x "$OUT/payload/todo-server"
echo '<!doctype html><title>ok</title>' > "$OUT/payload/frontend-dist/index.html"
printf '%s\n' "$SHA" > "$OUT/payload/SHA"
tar -C "$OUT/payload" -czf "$OUT/todo-pi.tar.gz" todo-server frontend-dist SHA
