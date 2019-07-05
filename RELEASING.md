# Releasing

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in [BCP 14](https://tools.ietf.org/html/bcp14) [RFC2119](https://tools.ietf.org/html/rfc2119) [RFC8174](https://tools.ietf.org/html/rfc8174) when, and only when, they appear in all capitals, as shown here.

This document is licensed under [The Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0.html).

When using the name 'version' we mean the versioning scheme described in [VERSIONING.md](VERSIONING.md)

## Introduction

This document is to describe the release pipeline, which is taking the result of the artifacts created according to [BUILDING.md](BUILDING.md) and publish a release to the various release targets for the project.

We propose:
 - a set of release targets that are allowable
 - a pipeline for handling the release folder's artifacts

It is NOT the purpose of this document to describe how a project might create a build, NOR is it describing a strcture in which projects MUST write build artifacts to. It is describing the structure of the releases themselves.

## Release Pipeline

Each Pristine project MUST provide a `bin/release.sh` script which will make a release to the various targets.

Each target may be scripted directly into the `bin/release.sh` shell script, or it may be broken down into files following the pattern:`./bin/release.{target}.sh`.

While the `.sh` extension is mandatory, the scripts may be written with one of the following headers:
 - `#!bin/sh`
 - `#!bin/node`
 - `#!/usr/bin/env node`

### Create a build from current branch

Process is outlined in [BUILDING.md](BUILDING.md)

1. Clean the build directory
2. run: `bin/build.{target}.{ext}`

### Bump the version of the project

Projects SHOULD automate the version bump following [CONVENTIONAL_COMMITS.md](CONVENTIONAL_COMMITS.md).

### Generate Changelog

Projects SHOULD use generated changelogs from following [CONVENTIONAL_COMMITS.md](CONVENTIONAL_COMMITS.md).

### Commit the bump + changelog update

A project MUST generate a commit with the changes.

### Tag the commit with the bumped version

A project MUST be tagged with the semantic versioning scheme from [VERSIONING.md](VERSIONING.md).

### Sign the releases.

 - MUST be a pgp signature
 - MUST be the same pgp key as is registered with Github
 - MUST be a detached ascii-armored (.asc) signature 
 - All files in the build folder MUST have an associated signature file

### Push changelog & version bump

### Run Release Targets

For each of the desired release targets, prepare and push the release.

#### Example Release Targets

1. Github
2. Docker Hub

## Resources

- [semantic-release](https://github.com/semantic-release/semantic-release)
- [Conventional Commits](https://conventionalcommits.org/)
