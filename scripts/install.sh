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

if ! command -v cargo &> /dev/null; then
  echo "Error: Cargo/Rust toolchain not found."
  echo "Please install Rust from https://www.rust-lang.org/tools/install and try again."
  exit 1
fi

PLIST_NAME="com.user.mac-audio-keepalive.plist"
TARGET_BIN="${TARGET_BIN:-/usr/local/bin/mac-audio-keepalive}"
LAUNCH_AGENTS_DIR="${LAUNCH_AGENTS_DIR:-$HOME/Library/LaunchAgents}"
PLIST_DEST="$LAUNCH_AGENTS_DIR/$PLIST_NAME"
USER_ID=$(id -u)

echo "Building release binary..."
run_cmd cargo build --release

echo "Ensuring target directories exist..."
run_cmd mkdir -p "$LAUNCH_AGENTS_DIR"
run_cmd sudo mkdir -p "$(dirname "$TARGET_BIN")"

echo "Installing binary to $TARGET_BIN..."
run_cmd sudo cp target/release/mac-audio-keepalive "$TARGET_BIN"
run_cmd sudo chmod 755 "$TARGET_BIN"

echo "Installing launchd plist to $PLIST_DEST..."
run_cmd cp "$PLIST_NAME" "$PLIST_DEST"

echo "Loading launchd service..."
run_cmd launchctl bootout "gui/$USER_ID/$PLIST_NAME" 2>/dev/null || true
run_cmd launchctl bootstrap "gui/$USER_ID" "$PLIST_DEST"

echo "Installation complete."
