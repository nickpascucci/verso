# Changelog

Notable changes to verso|recto are tracked in this file.

The format is based on the [Keep A Changelog 1.0.0](https://keepachangelog.com/en/1.0.0/) spec.
Releases may be found on [GitHub](https://github.com/nickpascucci/verso/releases/) and are tagged with
their release number in the Git repository. Release numbers follow the [Semantic Versioning
2.0.0](https://semver.org/) format. As a reminder, this format uses major, minor, and patch numbers
with the following form:

```
v1.2.3-test
 ^ ^ ^ ^
 | | | |
 | | | pre-release tag
 | | patch
 | minor
 major
```

These are incremented according to the following rules:

- *MAJOR* versions contain *backwards-incompatible changes*.
- *MINOR* versions contain new *backwards-compatible* features.
- *PATCH* versions contain *backwards-compatible* fixes.

## Types of changes

_Added_ for new features.
_Changed_ for changes in existing functionality.
_Deprecated_ for soon-to-be removed features.
_Removed_ for now removed features.
_Fixed_ for any bug fixes.
_Security_ in case of vulnerabilities.

### A note to release managers

When creating a new release in GitHub, please copy the `[Unreleased]` section to a new versioned
section and use it for the release's notes, in addition to verifying that version numbers are
updated throughout the repository.

## [Unreleased]

### Added

- A new weave pattern, `@* <regex>`, can be used which expands to all of the fragments whose IDs
  match the given regular expression.

## v0.1.2

### Added

- Two new reference operators are now available in `recto`: `relpath` and
  `abspath`. `relpath` inserts the relative path from the current file to the
  fragment's source file while `abspath` inserts the absolute path of the
  fragment file, considering the directory in which `verso` was run as the
  root. (For example, if `verso` is run in directory `/a` and `/a/b.py` contains
  a fragment, then that fragment's `abspath` is `/b.py`.)

## v0.1.1

### Fixed

- Repaired bugs in logic that would reject valid IDs and consolidated the code.

## v0.1.0

### Added
- Initial release of `verso|recto`
- Most of the basic features work: creating annotated fragments, weaving them
  into documents, references, etc.
