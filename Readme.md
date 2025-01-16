# Monte Carlo Tree Search

A Rust crate to perfom monte carlo tree searches.

## Motivation

This repository and code has been created to explore two topics. First topic is the interface design space around monte carlo tree searches. The algorithm is inherently very domain independent. Can we find nice interfaces to reuse it often. The second idea is around counted scores. Could we work with explicit probabilities and make the hidden assumptions about distributions of the game state visible? Could we balance exploration vs exploitation better if we try to minimize the variance of the root node?
