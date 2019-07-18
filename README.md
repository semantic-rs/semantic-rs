# semantic-rs

[![Build Status](https://travis-ci.org/semantic-rs/semantic-rs.svg?branch=master)](https://travis-ci.org/semantic-rs/semantic-rs)

This tool helps people to publish crates following the [semver](http://semver.org/) specification.

Right now if you're building a new crate publishing new versions includes a high amount of work. You need to decide if the new version will be either a new Major, Minor or Patch version. If that decision is made, the next step is to write a changelog with all the things changed. Then increase the version in `Cargo.toml`. Make a commit and a new tag for the new version and lastly publish it to crates.io.
If you need to repeat these steps every time, chances are high you make mistakes.
semantic-rs automates all these steps for you so you can focus more on developing new features instead.

## Pristine

This project follows the Pristine convention: to know more check out the [README_PRISTINE.md](README_PRISTINE.md) or visit [pristine core repo](https://github.com/etclabscore/pristine) 

## Workflow

### Manual

- Install semantic-rs on your machine.
- Place a [releaserc.toml](releaserc.toml) with "git", "rust" and "github" plugins enabled in the root of your repo
- Follow the [Angular.js commit message conventions](CONVENTIONAL_COMMITS.md) when you commit changes to your repository
- When you're done with development, run semantic-rs
- Based on your changes it determines the next version number, generates a changelog, commits it and creates a new tag
- It also increases the version number in `Cargo.toml` (also committed)
- Runs `cargo package` for you
- Creates a release on GitHub
- Publishes the new version to [crates.io](https://crates.io)
- Done 🚀

### Automated

- Install semantic-rs in your CI environment
- Place a [releaserc.toml](releaserc.toml) in the root of your repo
- Follow the [Angular.js commit message conventions](CONVENTIONAL_COMMITS.md) when you commit changes to your repository
- Set Environment Variables:
  * GH_TOKEN for pushing changes and uploading release artifacts
  * CARGO_TOKEN for releasing to [crates.io](https://crates.io)
- Configure release triggers in your CI configuration 
  * [CircleCI Configuration Example](.circleci/config.yml) can be found in this repo
- Enjoy your automated release pipeline 🚀 

## Usage

Static binaries are available for download on the [releases](https://github.com/etclabscore/semantic-rs/releases) page.

### Installation from git

Ubuntu:

```
$ sudo apt-get install -y cmake libssl-dev pkg-config zlib1g-dev
$ cargo install --git https://github.com/etclabscore/semantic-rs --tag VERSION
```

### Prerequisites

You need the following data beforehand:

- A GitHub application token [Get it here](https://github.com/settings/tokens/new)
- Your crates.io API key [Get it here](https://crates.io/me)

### Run it

semantic-rs plugins depend on some data being passed in via environment variables. We recommend placing them in a git-ignored `.env` file in the repo's root.

Setting `GIT_COMITTER_NAME` and `GIT_COMMITTER_EMAIL` is optional. If you omit those, we default to the settings from your (global) git configuration.

```bash
$ export GH_TOKEN=<GHTOKEN>
$ export CARGO_TOKEN=<CARGOTOKEN>
$ export GIT_COMMITTER_NAME=<Your name>
$ export GIT_COMMITTER_EMAIL=<Your email>
$ semantic-rs --dry
#...
```

By default it runs in release mode. If you want to just check the release without publishing it, use the `--dry` flag. In `dry-run` mode you can see which steps would be performed and also the resulting changelog.

```bash
$ semantic-rs
```
This would perform the steps defined in your `releaserc.toml`, see below for the description of allowed statements in this configuration file.

## Configuration

`releaserc.toml` derives the main idea of splitting execution into a set of steps from the awesome [semantic-release](https://github.com/semantic-release/semantic-release) tool.

Derived from the [semantic-release documentation](https://github.com/semantic-release/semantic-release/blob/master/README.md#release-steps):

| Step                | Description                                                                                                                     |
|---------------------|---------------------------------------------------------------------------------------------------------------------------------|
| Pre Flight          | Verify all the conditions to proceed with the release.                                                                          |
| Get last release    | Obtain the last release and the respective git revision                                                                         |
| Derive Next Version | Determine the type of release based on the changes since the last release.                                                      |
| Generate notes      | Generate release notes for the changes added since the last release.                                                            |
| Prepare             | Prepare the release (mainly generate artifacts and edit files).                                                                 |
| Verify Release      | Pre-release integrity check.                                                                                                   |
| Commit              | Commit changes, create git tag and push changed to the repository.                                                              |
| Publish             | Publish the release.                                                                                                            |
| Notify              | Notify of new releases or errors.                                                                                               |

Overall `releaserc.toml` document is structured as 3 tables: `plugins`, `steps` and `cfg`.

### Plugins Table

Plugins table describes the plugins `semantic-rs` should use for handling releases for the particular repository.
This table defines the relation of the name of the plugin to its location from where it can be retrieved (currently only built-in plugins are supported)

```toml
[plugins]
# Fully qualified definition
git = { location = "builtin" }
# Short definition
clog = "builtin"
```

Fully qualified definition is akin to `Cargo.toml` full dependency description, while the short one just defines the location,
with the idea that the fully qualified definition may be trivially derived by `semantic-rs`.

### Steps Table

Steps table defined which plugins should be used for each step (see [Built-in Plugins](#built-in-plugins))

There are three ways to declare a step:

##### A singleton step definition

The only handler for any step is called singleton:
```toml
[steps]
commit = "git"
```

Any step can be defined as a singleton step, but some steps may only be defined as singletons:
 - Get Last Version
 - Commit
 
##### A shared step definition

Non-singleton steps may use several plugins, like `Pre Flight` would perform checks on every connected plugin, 
or `Generate Changelog` would concatenate outputs of plugins it uses:
```toml
[steps]
pre_flight = ["git", "github", "rust"]
```

The order of step names being referenced in this list defines the order in which the plugins would be invoked
while running the step.

##### A discovery step definition

Since plugins API provides a way to know which methods plugin implements, there's a way to automatically
discover which plugin to run for any step (except singleton-only steps).

```toml
[steps]
notify = "discover"
```

This would make `semantic-rs` derive a Shared step definition for any step marked as `discover` based
on which methods the attached plugins advertise.

The order of plugin invocations in this case is defined by the original order in the [Plugins table](#plugins-table)

### Configuration table

Configuration table contains global key-value configuration as well as plugin-specific configuration.

```toml
# Global configuration
[cfg]
key = "value"

# Git plugin configuration
[cfg.git]
branch = "master"
remote = "origin"
```

Basically, in toml plugin configurations are just sub-tables in the global `cfg` map.

## Built-in Plugins

### Git

##### Plugins Table Example

```toml
[plugins]
git = "builtin"
```

##### Methods

| Step                | Description                                                                                                                     |
|---------------------|---------------------------------------------------------------------------------------------------------------------------------|
| Pre Flight          | Check that repo exists, derive committer name and email, perform https-forcing if the `force_https` flag is set                 |
| Get last release    | Rev-parse history to find the latest version tag, or return the initial commit revision if there are no tags                    |
| Commit              | Commit changes, create git tag and push changed to the repository.                                                              |

##### Configuration

```toml
[cfg.git]
user_name = "John Doe"          # Optional: default = $GIT_COMMITTER_NAME or derived from git config
user_email = "jd@example.com"   # Optional: default = $GIT_COMMITTER_EMAIL or derived from git config
branch = "master"               # Optional: default = "master"
remote = "origin"               # Optional: default = "origin"
# Replace git@ and git://. links with https:// links in remote
force_https = true              # Optional: default = false
```

### GitHub

##### Plugins Table Example

```toml
[plugins]
github = "builtin"
```

##### Methods

| Step                | Description                                                                                                                     |
|---------------------|---------------------------------------------------------------------------------------------------------------------------------|
| Pre Flight          | Check that GH_TOKEN is set, and verify assets list correctness                                                                  |
| Publish             | Publish the release to GitHub and upload assets                                                                                 |

##### Configuration

```toml
[cfg.github]
user = "etclabscore"        # Optional: default is derived from git remote url
repository = "semantic-rs"  # Optional: default is derived from git remote url
remote = "origin"           # Optional: default = "origin"
branch = "master"           # Optionl: default = "master"
# Optional: default = empty list
assets = [
    "Changelog.md",
    "artifacts/*"
]
```

##### Additional requirements

`GH_TOKEN` env var MUST be set if this plugin is used.

### Rust

##### Plugins Table Example

```toml
[plugins]
rust = "builtin"
```

##### Methods

| Step                | Description                                                                                                                     |
|---------------------|---------------------------------------------------------------------------------------------------------------------------------|
| Pre Flight          | Verify that CARGO_TOKEN is set                                                                                                  |
| Prepare             | Update version in Cargo.toml                                                                                                    |
| Verify Release      | Run `cargo package`                                                                                                             |
| Publish             | Publish the release to crates.io                                                                                                |

##### Configuration

None

##### Additional requirements

`CARGO_TOKEN` env var MUST be set if this plugin is used.

### Clog

##### Plugins Table Example

```toml
[plugins]
clog = "builtin"
```

##### Methods

| Step                | Description                                                                                                                     |
|---------------------|---------------------------------------------------------------------------------------------------------------------------------|
| Derive Next Version | Analyze the commits and derive a type of semver version bump (Major/Minor/Patch)                                                |
| Generate notes      | Generate release notes for commits in range PREV_RELEASE..HEAD                                                                  |
| Prepare             | Write changelog file                                                                                                            |

##### Configuration

```toml
[cfg.clog]
# Relative path from the repo root to changelog file
changelog = "Changelog.md" # Optional: default = "Changelog.md"
```

## Development

Requirements:
- cmake
- OpenSSL development package
  - Ubuntu: `libssl-dev`, `pkg-config`, `zlib1g-dev`
  - Mac Homebrew: `openssl`
- Nightly Rust
   * Requires `try_trait` and `external_doc` features

### For OS X > 10.10

Note that since OS X 10.11 Apple doesn't ship development headers for OpenSSL anymore. In order to get it working, you need to run cargo with these variables configured:

```bash
OPENSSL_INCLUDE_DIR=`brew --prefix openssl`/include \
OPENSSL_LIB_DIR=`brew --prefix openssl`/lib \
cargo build
```

### Build locally

Clone this project:

```bash
$ git clone git@github.com:etclabscore/semantic-rs.git
```

As a test project you can use this one: [https://github.com/mersinvald/semantic-rs-test-project](https://github.com/badboy/test-project).

Clone it as well:

```bash
$ git clone https://github.com/mersinvald/semantic-rs-test-project.git test-project
```

In your top level directory there should be now the following two folders:

```bash
$ ls -l
semantic-rs
test-project
```

Change into the test-project folder.
Then you can run semantic-rs against the test project:

```bash
$ cargo run --manifest-path ../semantic-rs/Cargo.toml -- --dry
    Finished dev [unoptimized + debuginfo] target(s) in 0.53s
     Running `/Users/mersinvald/dev/etclabscore/semantic-rs/target/debug/semantic-rs --dry`
semantic.rs 🚀
Resolving plugins
All plugins resolved
Starting plugins
All plugins started
>> Step 'notify' is marked for auto-discovery, but no plugin implements this method
Running step 'pre_flight'
Invoking plugin 'git'
Invoking plugin 'github'
Invoking plugin 'rust'
Running step 'get_last_release'
Invoking singleton 'git'
Running step 'derive_next_version'
Invoking plugin 'clog'
Running step 'generate_notes'
Invoking plugin 'clog'
Would write the following changelog: 
--------- BEGIN CHANGELOG ----------
## v3.0.0 (2019-07-18)


#### Features

*   configuration for semantic-rs 2.0 ([dfab5d46](dfab5d46))
*   Math mode ([24afa46f](24afa46f))

#### Breaking Changes

*   configuration for semantic-rs 2.0 ([dfab5d46](dfab5d46))

#### Bug Fixes

*   Into the void ([9e54f4bf](9e54f4bf))

---------- END CHANGELOG -----------
Running step 'prepare'
Invoking plugin 'clog'
clog(dry-run): saving original state of changelog file
clog: writing updated changelog
Invoking plugin 'rust'
rust(dry-run): saving original state of Cargo.toml
rust: setting new version '3.0.0' in Cargo.toml
Running step 'verify_release'
Invoking plugin 'rust'
rust: packaging new version, please wait...
rust: package created successfully
DRY RUN: skipping steps [Commit, Publish, Notify]
clog(dry-run): restoring original state of changelog file
rust(dry-run): restoring original state of Cargo.toml

```

Since `--dry` was passed, it only prints out what it would do. Note that if you run it on your local machine the output may differ.

## Run semantic-rs in CI environment

The example configuration for CircleCI can be found in [.circleci](.circleci)

##### Known caveats

CircleCI forces git+ssh remotes by default, so if you use GH_TOKEN for authentication, 
make sure to enable the `force_https` flag for [git plugin](#git) AND prepend the `semantic-rs` call with
```bash
git config --global --unset url.ssh://git@github.com.insteadof
```

## Contributing

Bug reports and pull requests are welcome on [GitHub](https://github.com/semantic-rs/semantic-rs).
You can find more information about contributing in the [CONTRIBUTING.md](https://github.com/semantic-rs/semantic-rs/blob/master/CONTRIBUTING.md).
This project is intended to be a safe, welcoming space for collaboration and discussion, and contributors are expected to adhere to the [Contributor Covenant](http://contributor-covenant.org/version/1/3/0/) code of conduct.

## License

This project is licensed under the MIT license.

