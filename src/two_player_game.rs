use crate::{Count, Evaluation, Player};

/// Two Player games are games there two players alternate taking turns, until the game ends in
/// either victory for one player (and defeat for the other) or a draw.
pub trait TwoPlayerGame: Clone {
    /// A possible action to take in the game.
    type Move: Copy + Eq;

    /// Current state of the game yields terminal state or valid moves.
    ///
    /// # Parameters
    ///
    /// * `moves_buf`: In order to avoid repeated allocations to store legal moves, callers supply
    ///   a buffer to hold the to the method. The buffer will always contain a complete and
    ///   exclusive list of all valid moves after the call. This implies it being empty in case of
    ///   a terminal game state.
    fn state<'a>(&self, moves_buf: &'a mut Vec<Self::Move>) -> GameState<'a, Self::Move>;

    /// Change the board by playing a move. Precondition: The move must be valid.
    fn play(&mut self, mv: &Self::Move);

    /// The player whose turn it currently is. `0` For player one who starts the game `1` for player
    /// two who makes the second move. If the board is in a terminal position, it should return the
    /// player those turn it would be, i.e. the player which did not play the last move. Currently
    /// the trait requires turns to be alternating.
    fn current_player(&self) -> Player;
}

/// Possible states defining a game
#[derive(Debug, PartialEq, Eq)]
pub enum GameState<'a, Move> {
    /// Complete list of all legal moves for the current player
    Moves(&'a [Move]),
    WinPlayerOne,
    WinPlayerTwo,
    Draw,
}

impl<M> GameState<'_, M> {
    pub fn moves(&self) -> &[M] {
        match self {
            GameState::Moves(moves) => moves,
            _ => &[],
        }
    }

    pub (crate) fn map_to_evaluation(&self) -> Evaluation {
        match self {
            GameState::Moves(_) => Evaluation::Undecided(Count::default()),
            GameState::Draw => Evaluation::Draw,
            GameState::WinPlayerOne => Evaluation::Win(Player::One),
            GameState::WinPlayerTwo => Evaluation::Win(Player::Two),
        }
    }
}