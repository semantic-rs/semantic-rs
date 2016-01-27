#!/usr/bin/env bats

setup() {
  cd $WORKSPACE
}

setup_dirs() {
  if [ -f "_Cargo.toml" ]; then
    mv _Cargo.toml Cargo.toml
  fi

  if [ -d "_git" ]; then
    mv _git .git && git reset --hard master
  fi
}

@test "it runs" {
  run semantic-rs
  [ "$status" -eq 1 ]
}

@test "fails without Cargo.toml" {
  cd empty-dir
  run semantic-rs
  [ "$status" -eq 1 ]
}

@test "fails on non-git directories" {
  cd not-a-repo
  setup_dirs

  run semantic-rs
  [ "$status" -eq 1 ]
}

@test "fails with broken Cargo.toml" {
  cd broken-cargo-toml
  setup_dirs

  run semantic-rs
  [ "$status" -eq 1 ]
}

@test "Initializes to v1.0.0" {
  cd initial-release
  setup_dirs

  run semantic-rs -w
  [ "$status" -eq 0 ]
  grep -q 'version = "1.0.0"' Cargo.toml
}

@test "Bumps to next minor" {
  cd next-minor
  setup_dirs

  run grep -q 'version = "1.0.0"' Cargo.toml
  [ "$status" -eq 0 ]

  run semantic-rs -w
  [ "$status" -eq 0 ]

  run grep -q 'version = "1.1.0"' Cargo.toml
  [ "$status" -eq 0 ]

  run git log --oneline --format=format:%s
  [ "${lines[0]}" = "Bump version to 1.1.0" ]
}

@test "No bump when no new commits" {
  cd no-bump
  setup_dirs

  run grep -q 'version = "1.1.0"' Cargo.toml
  [ "$status" -eq 0 ]

  run semantic-rs
  [ "$status" -eq 0 ]
  [[ "$output" =~ "No version bump. Nothing to do" ]]

  grep -q 'version = "1.1.0"' Cargo.toml
}

@test "No crash with malformed tags" {
  cd malformed-tag
  setup_dirs

  semantic-rs
}

@test "It creates a new tag with message" {
  cd new-tag
  setup_dirs

  run git tag -l
  [ "$output" = "v1.0.0" ]

  run semantic-rs -w
  run git tag -l
  [ "${lines[0]}" = "v1.0.0" ]
  [ "${lines[1]}" = "v1.1.0" ]
}

@test "Runs a dry-run by default" {
  cd dry-run
  setup_dirs

  run semantic-rs
  [ "$status" -eq 0 ]
  grep -q 'version = "0.1.0"' Cargo.toml
}

@test "Runs in write-mode with CI=true" {
  cd write-mode
  setup_dirs

  export CI=true
  run semantic-rs
  [ "$status" -eq 0 ]
  grep -q 'version = "1.0.0"' Cargo.toml
}
