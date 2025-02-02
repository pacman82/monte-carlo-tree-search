# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/pacman82/monte-carlo-tree-search/releases/tag/v0.1.0) - 2025-02-02

### Added

- Separate Tree from Evalution implementation
- [**breaking**] Introduce generic parameter to control bias
- translate all counts to one state in case of propagating a deterministic outcome as undecided
- [**breaking**] Do not despair, allow for opponent mistakes even in lost games
- [**breaking**] Propagate draws
- Introduce Evaluation::Draw, its not constructed yet though.
- allow to inspect number of nodes and links
- [**breaking**] prove win in one move
- Handle unexplored root children
- [**breaking**] Realize game if provable won, if initialized from terminal position
- Move tree to library
- Add simulation to library
- Introduce trait TwoPlayerGame
- Introduce count::Count

### Fixed

- current player in tic tac toe is now correct for drawed games
- Best move selection based on reward
- panic on unexplored root children
- Backpropagation of deterministic states

### Other

- Create release-plz.yml
- Add metainformation
- Add MIT License
- Update Readme with learnings about perfect intuition
- [**breaking**] Bias::bias now returns Self::Evaluation
- Perfect bias for connect four
- [**breaking**] Almost all tree methods generic over Evaluation
- [**breaking**] Introduce init_eval_from_game_state
- [**breaking**] Node generic over evaluation
- [**breaking**] Move update to Evaluation trait
- update readme
- Update no longer takes a previous_child_count argument
- introduce delta.previous_count
- [**breaking**] Introduce CountOrDecidedDelta
- updated_evaluation -> update
- Move updated_evaluation to CountOrDecided
- Pass only siblings to updated_evaluation
- updated_evaluation is now an associated method
- [**breaking**] Pass previous evaluation explicitly into updated_evaluation
- Pass child evaluations as iterator into update_evaluation
- Update comments on update_evaluation
- typo
- [**breaking**] Add selection_weight to evaluation
- make cmp_for a member of Evaluation
- [**breaking**] Introduce trait Evaluation
- Improve some doc comments
- Move random_play to bias module
- [**breaking**] rename simulation to random_play
- for solving connect four
- add assertion for play against perfect solver
- Beat connect four with better bias
- Play connect four against "better" expertise
- Remove superfluous parameter from backpropagation
- Avoid heap allocation of candidate buffer
- Tweak solve tic tac toe test
- Add test for connect four position 424424455557722225141717
- print move statistics then playing against perfect solver
- update dependencies
- Play connect four against yourself
- Solving tic-tac-toe
- [**breaking**] Replace u8 with Player
- [**breaking**] Introduce Player into Evaluation
- Solve win in 5 moves
- [**breaking**] Allow updated_evaluation to use children
- move propagate outcome to tree module
- Better panic message
- [**breaking**] Introduce estimated outcome
- add tic-tac-toe tests
- backpropagation is private
- Reduce allocations for building tree
- introduce Tree
- [**breaking**] rename score to reward
- Make count absolute
- Introduce bias to Search trait
- Introduce Search::NodeState
- [**breaking**] Tree now depends on game instead on Move directly
- formatting
- Do not play against perfect solver if executing all tests
- use Upper confidence bound
- Add readme
- Play against perfect solver
- Relative Score
- Terminal position
- Explore naive monte carlo for connect four
