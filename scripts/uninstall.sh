#!/usr/bin/env bash
set -euo pipefail

DRY_RUN="${DRY_RUN:-0}"

for arg in "$@"; do
  case $arg in
    --dry-run)
      DRY_RUN=1
      shift
        ;;
  esac
done

run_cmd() {
  if [ "$DRY_RUN" -eq 1 ]; then
    echo "[DRY RUN] $*"
  else
    "$@"
  fi
}

SERVICE_NAME="com.user.mac-audio-keepalive"
PLIST_NAME="${SERVICE_NAME}.plist"
TARGET_BIN="${TARGET_BIN:-/usr/local/bin/mac-audio-keepalive}"
LAUNCH_AGENTS_DIR="${LAUNCH_AGENTS_DIR:-$HOME/Library/LaunchAgents}"
PLIST_DEST="$LAUNCH_AGENTS_DIR/$PLIST_NAME"
USER_ID=$(id -u)

echo "Stopping and unloading launchd service..."
if [ "$DRY_RUN" -eq 1 ]; then
  run_cmd launchctl bootout "gui/$USER_ID/$SERVICE_NAME"
  run_cmd pkill -f "$TARGET_BIN"
else
  if launchctl print "gui/$USER_ID/$SERVICE_NAME" &>/dev/null; then
    run_cmd launchctl bootout "gui/$USER_ID/$SERVICE_NAME"
  fi
  if pgrep -f "$TARGET_BIN" &>/dev/null; then
    run_cmd pkill -f "$TARGET_BIN"
  fi
fi

if [ -f "$PLIST_DEST" ] || [ "$DRY_RUN" -eq 1 ]; then
  echo "Removing plist from $PLIST_DEST..."
  run_cmd rm -f "$PLIST_DEST"
fi

if [ -f "$TARGET_BIN" ] || [ "$DRY_RUN" -eq 1 ]; then
  echo "Removing binary from $TARGET_BIN..."
  run_cmd sudo rm -f "$TARGET_BIN"
fi

echo "Uninstallation complete."
