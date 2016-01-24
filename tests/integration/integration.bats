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

  run semantic-rs
  [ "$status" -eq 0 ]
  run grep -q 'version = "1.0.0"' Cargo.toml
  [ "$status" -eq 0 ]
}

@test "Bumps to next minor" {
  cd next-minor
  setup_dirs

  run grep -q 'version = "1.0.0"' Cargo.toml
  [ "$status" -eq 0 ]

  run semantic-rs
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

  run grep -q 'version = "1.1.0"' Cargo.toml
  [ "$status" -eq 0 ]
}

@test "No crash with malformed tags" {
  cd malformed-tag
  setup_dirs

  semantic-rs
}
