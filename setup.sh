#!/usr/bin/env bash
set -euo pipefail

APP="downloads_organiser"
BIN_SRC="target/release/${APP}"
BIN_DST="${HOME}/.local/bin/${APP}"
UNIT_DIR="${HOME}/.config/systemd/user"
UNIT_FILE="${UNIT_DIR}/${APP}.service"

cmd="${1:-install}"

ensure_path_hint() {
  if ! echo ":$PATH:" | grep -q ":$HOME/.local/bin:"; then
    echo "Note: $HOME/.local/bin is not on your PATH in this shell."
    echo "Add this to your shell rc file (e.g. ~/.bashrc, ~/.zshrc):"
    echo '  export PATH="$HOME/.local/bin:$PATH"'
  fi
}

write_unit_file() {
  mkdir -p "$UNIT_DIR"
  cat > "$UNIT_FILE" <<EOF
[Unit]
Description=Downloads Organiser (user daemon)

[Service]
ExecStart=%h/.local/bin/${APP}
Restart=on-failure

[Install]
WantedBy=default.target
EOF
}

case "$cmd" in
  install|update)
    echo "==> Building ${APP} (release)…"
    cargo build --release

    if [[ ! -f "$BIN_SRC" ]]; then
      echo "ERROR: Expected binary not found: $BIN_SRC"
      exit 1
    fi

    echo "==> Installing binary to ${BIN_DST}…"
    mkdir -p "${HOME}/.local/bin"
    install -m 0755 "$BIN_SRC" "$BIN_DST"

    echo "==> Installing/updating systemd user service…"
    write_unit_file

    echo "==> Reloading systemd user manager…"
    systemctl --user daemon-reload

    echo "==> Enabling + restarting ${APP}.service…"
    systemctl --user enable "${APP}.service" >/dev/null 2>&1 || true
    systemctl --user restart "${APP}.service" >/dev/null 2>&1 || systemctl --user start "${APP}.service"

    echo
    echo "Done."
    echo "Status:"
    systemctl --user --no-pager status "${APP}.service" || true
    echo
    ensure_path_hint
    echo "Logs: journalctl --user -u ${APP}.service -f"
    ;;

  status)
    systemctl --user --no-pager status "${APP}.service"
    ;;

  logs)
    journalctl --user -u "${APP}.service" -f
    ;;

  stop)
    systemctl --user stop "${APP}.service"
    ;;

  uninstall)
    echo "==> Stopping + disabling ${APP}.service…"
    systemctl --user stop "${APP}.service" >/dev/null 2>&1 || true
    systemctl --user disable "${APP}.service" >/dev/null 2>&1 || true

    echo "==> Removing unit file…"
    rm -f "$UNIT_FILE"
    systemctl --user daemon-reload

    echo "==> Removing binary…"
    rm -f "$BIN_DST"

    echo "Done."
    ;;

  *)
    echo "Usage: $0 {install|update|status|logs|stop|uninstall}"
    exit 2
    ;;
esac
