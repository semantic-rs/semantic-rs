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
- Based on your changes it calculates the next version number, generates a changelog, commits it and creates a new tag
- Done.

## Usage

If you run semantic-rs without any arguments, it operates on your current working directory:

```bash
$ semantic-rs
semantic.rs 🚀
Analyzing your repository
Current version: 0.5.0
Analyzing commits
Commits analyzed. Bump would be Minor
New version would be: 0.6.0
Would write the following Changelog:
====================================
## v0.6.0 (2016-01-30)


#### Features

*   Improve user interface ([497485a0](497485a0))

====================================
Would create annotated git tag
```

By default it runs in dry-run mode. This means it doens't perform changes automatically. You seed which changes would be performed and also the resulting changelog.

To perform the changes, pass `-w` as an argument:

```bash
$ semantic-rs -w
semantic.rs 🚀
Analyzing your repository
Current version: 0.5.0
Analyzing commits
Commits analyzed. Bump will be Minor
New version: 0.6.0
Writing Changelog
Creating annotated git tag
```
This performs the following operations:
- Create or update `Changelog.md` containing everything that changed
- It creates a new commit containing the following changes:
  - `Changelog.md`
  - An updated `Cargo.toml` with the new version number
- A new tag pointing to the last commit created recently

Note that commits and tags are created with your configured git user and email.

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

## Run semantic-rs in CI environment

Make sure to set the `CI=true` environment variable to disable dry-run mode.

## Contributing

Bug reports and pull requests are welcome on [GitHub](https://github.com/semantic-rs/semantic-rs).
You can find more information about contributing in the [CONTRIBUTING.md](https://github.com/semantic-rs/semantic-rs/blob/master/CONTRIBUTING.md).
This project is intended to be a safe, welcoming space for collaboration and discussion, and contributors are expected to adhere to the [Contributor Covenant](http://contributor-covenant.org/) code of conduct.

## License

This project is licensed under the MIT license.
