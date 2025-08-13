# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0](https://github.com/pacman82/monte-carlo-tree-search/compare/v0.3.0...v0.4.0) - 2025-08-13

### Other

- *(deps)* bump actions/checkout from 4 to 5
- *(deps)* bump rand from 0.9.0 to 0.9.1
- extract ucb calculation in selection
- Stop storing parent index in tree
- [**breaking**] Evaluation::init_from_game_state -> Evaluation::eval_for_terminal_state
- Extract ucb into own module
- formating
- Update to edition 2024

## [0.3.0](https://github.com/pacman82/monte-carlo-tree-search/compare/v0.2.0...v0.3.0) - 2025-02-21

### Fixed

- [**breaking**] Terminal nodes are now updated for Ucb Explorer

### Other

- [**breaking**] Remove usage of is_solved_legacy
- [**breaking**] Introduce Explore::is_solved
- [**breaking**] Remove Evaluation::selection_weight
- [**breaking**] Stop using selection_weight
- [**breaking**] Remove default implementation for select_child_pos
- [**breaking**] Use select child pos
- Introduce Evaluation::selected_child_pos
- Mark accidental state in Search
- Node is private
- Tree members now private
- Store best move in search, rather than best link
- Remove further direct access to nodes array outside of tree module
- fix lints
- Introduce Tree::add
- use Tree::evaluation instead of direct access to nodes
- Assert Ucb playing TicTacToe against itself ends in draw
- flip negation in if
- if branches
- pass delta explicitly to backpropagation
- [**breaking**] Rename trait Policy -> Exploror
- [**breaking**] Move update to policy
- Introduce Tree::child_move_and_eval
- Introduce tree::new
- move child_links to Tree
- [**breaking**] Rename Tree -> Search

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
