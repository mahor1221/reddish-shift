<!--
Note: In this file, do not use the hard wrap in the middle of a sentence for compatibility with GitHub comment style markdown rendering.
-->

# Changelog
All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org).

## [Unreleased]

## [0.1.2] - 2024-07-17
* Fix: Allow negative values in --location and --scheme arguments
* Fix: Apply gamma ramps without checking if it's changed in daemon mode
  to restore the desired ramps faster when an external program changes them. See
  'Why does the redness effect occasionally switch off for a few seconds?' in
  docs/redshift-readme.md
* Fix: Minor changes to the set command
  * Remove the hard requirement of providing at least one of the temperature,
    gamma or brightness cli arguments
  * Always use the default values of temperature, gamma and brightness when they
    don't exist. Don't use the values provided by the config file
* Feat: Add systemd service

## [0.1.1] - 2024-06-27
* Fix AUR and Crates.io builds

## [0.1.0] - 2024-06-27
* Initial release

[Unreleased]: https://github.com/mahor1221/reddish-shift/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/mahor1221/reddish-shift/releases/tag/v0.1.2
[0.1.1]: https://github.com/mahor1221/reddish-shift/releases/tag/v0.1.1
[0.1.0]: https://github.com/mahor1221/reddish-shift/releases/tag/v0.1.0
