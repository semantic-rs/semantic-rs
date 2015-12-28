# semantic-rs

[![Build Status](https://travis-ci.org/semantic-rs/semantic-rs.svg?branch=master)](https://travis-ci.org/semantic-rs/semantic-rs)

The purpose of this tool is to help people to publish crates following the [semver](http://semver.org/) specification.
Right now publishing crates manually is fairly error prone and a high amount of work.

## Development

Requirements:
- cmake
- Rust 1.5

Clone this project:

```bash
$ git clone git@github.com:semantic-rs/semantic-rs.git
```

You can run semantic-rs by calling:

```bash
$ cargo run
```

This analyzes the current git repository and updates the project's `Cargo.toml`.

