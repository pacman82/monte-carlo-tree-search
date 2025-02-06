# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0](https://github.com/pacman82/monte-carlo-tree-search/compare/v0.1.1...v0.2.0) - 2025-02-06

### Added

- [**breaking**] Ucb is now an evaluation
- [**breaking**] Implement Evaluation for Ucb, but we can not handle selecting terminal nodes, yet.
- [**breaking**] Move init_from_game_state to Evaluation trait

### Other

- [**breaking**] policy -> bias
- [**breaking**] Rename RandomPlayoutUcbSolver -> UcbSolver
- [**breaking**] Rename RandomPlayoutUcb -> Ucb
- [**breaking**] rename bias -> Policy
- [**breaking**] rename UcbSolver -> CountWdlSolved
- [**breaking**] rename Ucb -> CountWdl
- Rename Count -> Ucb
- Rename CountOrDecided -> UcbSolver
- Move Count into own submodule
- Move count_or_decided into own submodule
- Create dependabot.yml

## [0.1.1](https://github.com/pacman82/monte-carlo-tree-search/compare/v0.1.0...v0.1.1) - 2025-02-02

### Fixed

- Correct repository URL in Metadata

### Other

- rename Readme.md to uppercase
