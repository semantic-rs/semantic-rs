# semantic-rs

[![Build Status](https://travis-ci.org/semantic-rs/semantic-rs.svg?branch=master)](https://travis-ci.org/semantic-rs/semantic-rs)

The purpose of this tool is to help people to publish crates following the [semver](http://semver.org/) specification.

Right now if you're building a new crate publishing new versions includes a high amount of work. You need to decide if the new version will be either a new Major, Minor or Patch version. If that decision is made, the next step is to write a changelog with all the things changed. Then increase the version in `Cargo.toml`. Make a commit and a new tag for the new version and lastly publish it to crates.io.
If you need to repeat these steps every time, chances are high you make mistakes.
semantic-rs automates all these steps for you so you can focus more on developing new features instead.

## Workflow

- Install semantic-rs on your machine.
- Follow the [Angular.js commit message conventions](https://docs.google.com/document/d/1QrDFcIiPjSLDn3EL15IJygNPiHORgU1_OOAqWjiDU5Y/edit?pref=2&pli=1) when you commit changes to your repository
- When you're done with development, run semantic-rs
- Based on your changes it determines the next version number, generates a changelog, commits it and creates a new tag
- It also increases the version number in `Cargo.toml` (also committed)
- Runs `cargo package` for you
- Creates a release on GitHub
- Publishes the new version to [crates.io](crates.io)
- Done 🚀

## Usage

### Prerequisites

You need the following data beforehand:

- A GitHub application token [Get it here](https://github.com/settings/tokens/new)
- Your crates.io API key [Get it here](https://crates.io/me)

### Run it

semantic-rs depends on some data being passed in via environment variables. In our examples we specify those variables explicitly but if you run semantic-rs frequently you may want to configure those in your shell's configuration file.

Setting `GIT_COMITTER_NAME` and `GIT_COMMITTER_EMAIL` is optional. If you omit those, we default to the settings from your (global) git configuration. 

If you run semantic-rs without any arguments, it operates on your current working directory:

```bash
$ export GH_TOKEN=<GHTOKEN>
$ export CARGO_TOKEN=<CARGOTOKEN> 
$ export GIT_COMMITTER_NAME=<Your name>
$ export GIT_COMMITTER_EMAIL=<Your email>
$ semantic-rs
#...
```

By default it runs in dry-run mode. This means it doesn't perform changes automatically. You see which steps would be performed and also the resulting changelog.

To perform the changes, pass `-w` as an argument:

```bash
$ semantic-rs -w
```
This performs the following operations:
- Create or update `Changelog.md` containing everything that changed
- Create a new commit containing the following changes:
  - `Changelog.md`
  - An updated `Cargo.toml` with the new version number
- Create a new annotated git tag pointing to the last commit created recently and including the Changelog for the new version
- A new version published to [crates.io](crates.io)
- A new release on GitHub
- Push the new commit and tag to GitHub

## Development

Requirements:
- cmake
- OpenSSL development package
  - Ubuntu: `libssl-dev`
  - Mac Homebrew: `openssl`
- Rust 1.5 or later

Clone this project:

```bash
$ git clone git@github.com:semantic-rs/semantic-rs.git
```

As a test project you can use this one: [https://github.com/badboy/test-project](https://github.com/badboy/test-project).

You can run semantic-rs by calling:

```bash
$ cargo run
```

This analyzes the current git repository and updates the project's `Cargo.toml`.

To pass in arguments you need to separate them with `--`:

```bash
cargo run -- -p ~/home/my-project -w
```

## Run semantic-rs in CI environment

Make sure to set the `CI=true` environment variable to disable dry-run mode.

## Contributing

Bug reports and pull requests are welcome on [GitHub](https://github.com/semantic-rs/semantic-rs).
You can find more information about contributing in the [CONTRIBUTING.md](https://github.com/semantic-rs/semantic-rs/blob/master/CONTRIBUTING.md).
This project is intended to be a safe, welcoming space for collaboration and discussion, and contributors are expected to adhere to the [Contributor Covenant](http://contributor-covenant.org/version/1/3/0/) code of conduct.

## License

This project is licensed under the MIT license.
