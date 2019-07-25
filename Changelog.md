<a name="v2.7.0"></a>
## v2.7.0 (2019-07-25)


#### Features

* **github:**
  *  draft release flag support ([36fff0e5](36fff0e5))
  *  pre_release flag support ([8b5b669c](8b5b669c))



<a name="v2.6.0"></a>
## v2.6.0 (2019-07-21)


#### Features

* **ci:**
  *  setup docker environment ([42f73a39](42f73a39))
  *  Windows builds & tests ([89b4227b](89b4227b))
  *  cache rustup path for rust:latest container ([041abd31](041abd31))
  *  use Cargo.lock as a cache key ([9ffffaa8](9ffffaa8))
  *  treat `chore: ...` as semantic-pr ([725aabec](725aabec))
  *  run tests on MacOS ([a065669f](a065669f))
  *  more efficient job queue ([223ee033](223ee033))
  *  build MacOS binaries ([de77294e](de77294e))
* **docker:**  publishing to DockerHub ([37f49b41](37f49b41), closes [#11](11))
* **docs:**  docker plugin description ([1721dfb8](1721dfb8))



<a name="v2.5.0"></a>
## v2.5.0 (2019-07-21)


#### Features

* **clog:**  ignore list for commit segments ([c8eb580a](c8eb580a))



<a name="v2.4.0"></a>
## v2.4.0 (2019-07-21)


#### Features

* **ci:**  use rustup-components-history to get the latest usable nightly ([de9c4a3e](de9c4a3e))
* **plugin:**  pass params by reference to avoid a lot of cloning ([4a98a0a0](4a98a0a0))

#### Bug Fixes

* **ci:**  fixate nightly version for `test` stage (one that has clippy) ([f2b3d190](f2b3d190))



<a name="v2.3.0"></a>
## v2.3.0 (2019-07-20)


#### Features

*   exit early if derived version is the same is last one ([51d9f4ec](51d9f4ec))



<a name="v2.2.0"></a>
## v2.2.0 (2019-07-19)


#### Features

* **deps:**
  *  update hubcaps (0.3 -> 0.5)   removes hyper   removes hyper-native-tls   adds futures   adds tokio ([207d5e55](207d5e55), closes [#27](27), [#22](22), [#21](21))
  *  update git2 (0.7.5 -> 0.9) ([d7c6ea8d](d7c6ea8d), closes [#25](25))



<a name="v2.1.0"></a>
## v2.1.0 (2019-07-19)


#### Features

* **ci:**  upload artifacts from /workspace/bin/* ([4ba5eeae](4ba5eeae))
* **github:**  log artifacts that are gonna be published in pre_flight ([a1aedb4a](a1aedb4a))



<a name="v2.0.0"></a>
## v2.0.0 (2019-07-19)


#### Features

*   remove extra dependencies ([e08bed0a](e08bed0a))
*   dry-run flag ([0c9b02aa](0c9b02aa))
*   dotenv support ([33642c17](33642c17))
*   plugin system & releaserc.toml configuration ([06c93d73](06c93d73))
* **ci:**
  *  copy artifacts into bin/ directory ([a4c34384](a4c34384))
  *  switch to nightly rust container ([bc1fc4eb](bc1fc4eb))
* **docs:**
  *  add crate-level documentation and some descriptive comments in proto ([f74384ed](f74384ed))
  *  README.md description of releaserc.toml and plugin system ([11b56271](11b56271))
* **git:**  last release search support for new repos (with no tags) ([765b7d2d](765b7d2d))
* **log:**  print the original file if dry-run guard fails to write it ([706d0c52](706d0c52))
* **logs:**  print error messages in main.rs instead of panicking ([356cbc54](356cbc54))

#### Breaking Changes

*   plugin system & releaserc.toml configuration ([06c93d73](06c93d73))



<a name="v1.2.0"></a>
## v1.2.0 (2019-07-07)


#### Features

* **error-handling:**  transition to failure crate ([8d7d261a](8d7d261a))

#### Bug Fixes

* **github:**  rewrite asset puslishing with reqwest, now uploads are working ([d6046143](d6046143))



<a name="v1.1.0"></a>
## v1.1.0 (2019-07-07)


#### Bug Fixes

*   remove travis-specific build order checks ([09e2f867](09e2f867))
* **README.md:**  s/Typescript/Rust/g ([a0174450](a0174450))
* **cargo.toml:**  typo in authors list ([b16b71f1](b16b71f1))
* **ci:**
  *  correct cargo install invocation ([5bfe0fae](5bfe0fae))
  *  corrected invocation of semantic-rs ([095017b0](095017b0))
  *  remove build stage from release pipeline as install does the build ([51839791](51839791))
  *  install build-essential before building semantic-rs ([f7ae408d](f7ae408d))
* **docs:**
  *  remove irrelevant BUILDING.md and introduce rust-lib specific changes to the other documents ([0eb13871](0eb13871))
  *  typos in README.md ([25627a8b](25627a8b))
* **github-template:**  more rust lib oriented bug report template ([e06db885](e06db885))
* **gitignore:**  remove Cargo.lock as it's a binary project ([beece801](beece801))

#### Features

*   https remote forcing flag ([17a4abd8](17a4abd8))
*   asset upload for github ([1f291402](1f291402))
*   Add support for major zero-style initial development ([60da0684](60da0684))
* **ci:**
  *  force HTTPS remote in semantic-rs by default ([5cdbcbb4](5cdbcbb4))
  *  caching for release ([c76ecd66](c76ecd66))
  *  more platform-specific build steps for future support of windows & macos ([f8421a9d](f8421a9d))
  *  upload binary and the Changelog to the assets ([4134aacc](4134aacc))
  *  build static musl binary in install phase ([1f6145fd](1f6145fd))
  *  use current branch for dry-run ([04ae0cf8](04ae0cf8))
  *  save deps after cargo test run as opposed to the end of test job ([eda1431c](eda1431c))
  *  release-dry-run stage for PRs ([16befcf0](16befcf0))
  *  preinstall apt build deps ([e587e46d](e587e46d))
  *  merge clippy and rustfmt into test phase, introduce a separate install step ([e37b38f7](e37b38f7))
  *  circleci config with automatic releases ([25560911](25560911))
* **docs:**  merged pristine-rust ([0dad54a5](0dad54a5))
* **git:**  add circleci special word to disable CI runs for release commits ([7a0fbc57](7a0fbc57))
* **init:**
  *  add rust-toolchain defaulting to current stable ([739f5718](739f5718))
  *  initialize the cargo project ([4419b5f0](4419b5f0))
  *  add empty .rustfmt.toml ([16b2c31d](16b2c31d))
  *  add .editorconfig ([029ba110](029ba110))
  *  add .gitignore ([f72f6843](f72f6843))
* **logs:**
  *  log the remote we're using ([869d40de](869d40de))
  *  improved logging through log and env_logger ([438810ad](438810ad))
* **style:**
  *  cargo fmt --all ([c4e4ea70](c4e4ea70))
  *  transition to Rust-2018 ([1b3f9116](1b3f9116))



<a name="v1.0.0"></a>
## v1.0.0 (2018-09-16)


#### Bug Fixes

*   remove superflous to_string() ([d16a236b](d16a236b))
*   Use hyper_native_tls for GitHub interaction ([a3a30a84](a3a30a84))
*   we can only push if the gh_token is present ([7d8f1ee3](7d8f1ee3))
*   Updated hubcaps needs client directly ([99c9048f](99c9048f))
*   indentation and error message ([bbc040eb](bbc040eb))
*   Does not fail when project has no remote ([14edbaa3](14edbaa3))
*   write repo_name to repository_name instead of user ([b8235b26](b8235b26))
*   use compatible cd flag ([dc69ac67](dc69ac67))
*   make CDPATH assignement to empty explicit ([384541e0](384541e0))
*   restore script intention ([20160882](20160882))
*   warnings by shellcheck && cd problem ([9e12edd3](9e12edd3))
*   Avoid non-POSIX readlink parameter ([a3f0c9e6](a3f0c9e6), closes [#103](103))
*   Reorder commands to first commit, then package ([63b88e0b](63b88e0b))
*   Make help output consistent ([773e9a42](773e9a42))
*   Switch to using --write=yes|no ([11f69f21](11f69f21))
*   Only wait for other builds in release mode ([19d2212f](19d2212f))
*   Handle more remote urls, like ssh ([80c26341](80c26341))
*   Use release clog version ([397af709](397af709))
*   Wait a short time before creating the release ([801d6ad3](801d6ad3))
*   Do not set fixed master branch ([92bfab66](92bfab66))
*   Create commit and tag against chosen branch ([187d1d3f](187d1d3f))
*   Push correct reference ([d10b09f4](d10b09f4))
*   Use uppercased env variable ([3ab01ed1](3ab01ed1))
*   Rename it properly to CARGO_TOKEN ([b969dbc6](b969dbc6))
*   Handle Result returned from revwalk ([01fd0271](01fd0271))
*   Canonicalize path to absolute path ([5a5318fc](5a5318fc))
*   Remove printed debug line ([677fedf3](677fedf3))
*   Add Cargo.lock if not ignored ([aa3205be](aa3205be))
*   Do not take AUTHOR into account for Git information ([14ad9f03](14ad9f03))
*   Properly fetch committer name and email and show helpful error message ([08ea0927](08ea0927))
*   Better error when repository path was not found ([7d12b177](7d12b177))
*   Commit the changelog file ([51675fce](51675fce))
*   Place changelog in repository's folder ([db018f7c](db018f7c), closes [#33](33))
*   Remove typo ([d1a126d4](d1a126d4))
*   Only commit Cargo.toml ([f39477ba](f39477ba))
*   Exit with error code on failure ([6b49489a](6b49489a))
*   Check that we're in a git repo ([e7dad211](e7dad211))
*   Fail on broken Cargo.toml ([6d984514](6d984514))
* **main:**  Handle result of write_new_version ([494b7bb1](494b7bb1))
* **toml_file:**  Pass path to toml_file ([f7f1c3f6](f7f1c3f6))

#### Breaking Changes

*   Switch to using --write=yes|no ([11f69f21](11f69f21))
*   dry-run by default. Overwrite by passing -w ([3789d066](3789d066))

#### Features

*   Fix release behavior ([fc96ed12](fc96ed12))
*   Replace docopt with clap ([3e0fc684](3e0fc684))
*   print message if everything's fine ([acec3cd7](acec3cd7))
*   Check if CARGO_TOKEN is defined or not ([8e248bb4](8e248bb4))
*   validating git remote ([358154c7](358154c7))
*   Hook preflight check into main function ([c0456fcc](c0456fcc))
*   Disable release mode if write mode is disabled ([b58812cf](b58812cf))
*   Allow write mode to be disabled, even on CI ([08c2f9f9](08c2f9f9))
*   adapt user message after push ([6b4ec6c4](6b4ec6c4))
*   use callback username to fetch ssh key ([3ccc742e](3ccc742e))
*   fetch ssh key based on email ([332390ed](332390ed))
*   Check if remote is GitHub url ([3f911b58](3f911b58))
*   provide additional information for the end user ([bbcff343](bbcff343))
*   performs push and github release only if there's a remote ([c5e9aa0e](c5e9aa0e))
*   add logger::warn method ([6eaa6bb6](6eaa6bb6))
*   handle the case if project doesn't have origin remote ([bc5224aa](bc5224aa))
*   Include and enable env_logger ([916f4963](916f4963))
*   In CI mode wait for other builds to finish ([6355cc46](6355cc46))
*   Implement branch detection and abort early ([934535e0](934535e0))
*   Use Hubcaps' own std error implementation ([c3b555a8](c3b555a8))
*   Print nice final message ([22d3f887](22d3f887))
*   Introduce special release-mode for releases on GitHub and crates.io ([8c7a699f](8c7a699f))
*   Fetch tokens ([bb9a00eb](bb9a00eb))
*   Extract user and repository name from remote URL ([9666341f](9666341f))
*   Method to publish to crates.io ([9601344b](9601344b))
*   Create proper release on GitHub ([67b48690](67b48690))
*   Push new commit & tag to remote repository ([47ae6dbd](47ae6dbd))
*   Handle IoError ([d075f30a](d075f30a))
*   Make sure error messages are shown by flushing ([b8806d98](b8806d98))
*   Package crate before committing ([2aae1836](2aae1836))
*   Update lockfile before committing ([2b1ba605](2b1ba605))
*   Disable dry-run by setting CI environment variable ([fe68adef](fe68adef))
*   dry-run by default. Overwrite by passing -w ([3789d066](3789d066))
*   Print generated changelog to stdout in dry-run mode ([f6575055](f6575055))
*   Check dry-run flag and act accordingly ([e35d2e29](e35d2e29))
*   Show semantic-rs' version ([bb01b553](bb01b553))
*   Write git tag with changelog included ([acbef4f5](acbef4f5))
*   Create git tag after successful commit ([c43212dd](c43212dd))
*   add git2-commit dependency ([f6e39b68](f6e39b68))
* **commit:**
  *  write new version in commit message ([21c11855](21c11855))
  *  Commit updated Cargo.toml and Cargo.lock ([15682f32](15682f32))
* **main:**  Allow the user to pass a custom path to a repository ([ce2dd516](ce2dd516))



