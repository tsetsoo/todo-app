#!/usr/bin/env bash
set -euo pipefail
# Run on the Pi as root (or via sudo).
# Usage: sudo ./bootstrap.sh

REPO_DIR="$(cd "$(dirname "$0")" && pwd)"
# /opt/todo, /opt/todo/releases, and the updater script are root-owned;
# only /opt/todo/data (the sqlite db, owned by the todo.service user) is pi:pi.
install -d -o root -g root -m 0755 /opt/todo /opt/todo/releases
install -d -o pi -g pi -m 0755 /opt/todo/data
install -o root -g root -m 0755 "$REPO_DIR/todo-update.sh" /opt/todo/todo-update.sh
install -m 0644 "$REPO_DIR/todo.service" /etc/systemd/system/todo.service
install -m 0644 "$REPO_DIR/todo-update.service" /etc/systemd/system/todo-update.service
install -m 0644 "$REPO_DIR/todo-update.timer" /etc/systemd/system/todo-update.timer
systemctl daemon-reload
# First pull (may fail if pi-latest not published yet — OK)
/opt/todo/todo-update.sh || echo "first pull deferred until pi-latest exists"
systemctl enable --now todo.service || true
systemctl enable --now todo-update.timer
systemctl status todo.service --no-pager || true
systemctl list-timers todo-update.timer --no-pager
