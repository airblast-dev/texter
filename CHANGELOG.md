# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.1](https://github.com/airblast-dev/texter/compare/v0.2.0...v0.2.1)

### 🚜 Refactor


- Add ellided lifetime for `Text::lines` return - ([edfde3e](https://github.com/airblast-dev/texter/commit/edfde3eec66d8b222c68e875fb1d4e7ac1eaeac1))
- Remove tracing - ([ccb5277](https://github.com/airblast-dev/texter/commit/ccb5277ccefc51f51c7758d74b32b18d65c6eaca))

### 📚 Documentation


- Add release-plz config for pretty changelogs - ([4b2575c](https://github.com/airblast-dev/texter/commit/4b2575c6ed73a781bb262d3dee65de82351fe9f4))
- Replace versioned URL's with latest - ([93f1fce](https://github.com/airblast-dev/texter/commit/93f1fce882fe5847249aaa38792287af0a8f7c14))

### ⚙️ Miscellaneous Tasks


- Bump deps - ([a0a48f0](https://github.com/airblast-dev/texter/commit/a0a48f0068f23bebb8488749e6f392592dfb0ee1))
- Safety comment cleanup for fast_replace_range - ([e342c32](https://github.com/airblast-dev/texter/commit/e342c32daa15f8bb59c911658a10415751077012))


## [0.2.0](https://github.com/airblast-dev/texter/compare/v0.1.6...v0.2.0) - 2025-03-26

### Other

- [**breaking**] update tree-sitter to latest

## [0.1.6](https://github.com/airblast-dev/texter/compare/v0.1.5...v0.1.6) - 2025-03-26

### Fixed

- fix feature gated doc tests where lsp-types feature is not enabled

### Other

- use release plz in CI
- update deps
- add assertion for vec truncation for EolIndexes
- update rstest dev dep
- reduce unsafe code
