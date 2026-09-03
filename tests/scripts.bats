#!/usr/bin/env bats

setup() {
  export DRY_RUN=1
}

@test "install.sh runs cleanly in dry-run mode" {
  run ./scripts/install.sh --dry-run
  [ "$status" -eq 0 ]
  [[ "$output" =~ "Building release binary..." ]]
  [[ "$output" =~ "[DRY RUN] cargo build --release" ]]
}

@test "uninstall.sh runs cleanly in dry-run mode" {
  run ./scripts/uninstall.sh --dry-run
  [ "$status" -eq 0 ]
  [[ "$output" =~ "Stopping and unloading launchd service..." ]]
  [[ "$output" =~ "[DRY RUN]" ]]
}

@test "install.sh fails gracefully when cargo is missing" {
  PATH="/usr/bin:/bin" run ./scripts/install.sh --dry-run
  [ "$status" -eq 1 ]
  [[ "$output" =~ "Error: Cargo/Rust toolchain not found." ]]
}