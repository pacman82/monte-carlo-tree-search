# Monte Carlo Tree Search

A Rust crate to perfom monte carlo tree searches on two player board games.

## Motivation

This repository and code has been created to explore two topics. First topic is the interface design space around monte carlo tree searches. The algorithm is inherently very domain independent. Can we find nice interfaces to reuse it often. The second idea is around counted scores. Could we work with explicit probabilities and make the hidden assumptions about distributions of the game state visible? Could we balance exploration vs exploitation better if we try to minimize the variance of the root node?

## Learnings so far

### Design

In the beginning I fell into the trap of temporal decomposition, structuring the code around the steps of a monte carlo tree search:

* Selection
* Expansion
* Simulation
* Backpropagation

Yet in order to isolate the decisions I want to play around with (using baysean probabilities vs. UCB, utilizing the tree search for different domains, checking if tree search is viable for proofs if bias are good enough, etc. ) it turns out each of these steps is affected by any decision. So far I think the following domains should be separated.

* The actual data structure of the tree. Is it box allocated nodes with references to each other, or rather contigious arrays of nodes and links (I choose the latter design). This decision should not matter for other parts of the code.
* The action space and assumptions about the difference agents. Here we are currently more specialized than might be necessary. This is currently handled by the `TwoPlayerGame` trait, and as the name might indicated is to be intended to be implemented for two player board games. Currenly assuming alternating order between players, perfect information, no random elements. Right now there are implementations for TicTacToe and ConnectFour in the tests. Works pretty well.
* For `Evaluation` currently we are only using an Upper confidence bound based strategy, which is used during backpropagation and selection.
* `Bias` is already exchangable. There is a generic implementation `RandomPlayoutBias` which works for any game. A slightly more sophisticated heuristic is necessary to make the moves viable for connect-four.
