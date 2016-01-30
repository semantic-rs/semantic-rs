# semantic-rs

[![Build Status](https://travis-ci.org/semantic-rs/semantic-rs.svg?branch=master)](https://travis-ci.org/semantic-rs/semantic-rs)

The purpose of this tool is to help people to publish crates following the [semver](http://semver.org/) specification.
Right now publishing crates manually is fairly error prone and a high amount of work.

## Development

Requirements:
- cmake
- OpenSSL development package
  - Ubuntu: `libssl-dev`
  - Mac Homebrew: `openssl`
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

## Run semantic-rs in CI environment

Make sure to set the `CI=true` environment variable to disable dry-run mode.

## Contributing

Bug reports and pull requests are welcome on [GitHub](https://github.com/semantic-rs/semantic-rs).
You can find more information about contributing in the [CONTRIBUTING.md](https://github.com/semantic-rs/semantic-rs/blob/master/CONTRIBUTING.md).
This project is intended to be a safe, welcoming space for collaboration and discussion, and contributors are expected to adhere to the [Contributor Covenant](http://contributor-covenant.org/) code of conduct.

## License

This project is licensed under the MIT license.
