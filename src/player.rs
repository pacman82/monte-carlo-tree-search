/// A player of a two-player game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Player {
    One,
    Two,
}

impl Player {
    /// Turn player [`Player::One`] into [`Player::Two`] and vice versa.
    pub fn flip(&mut self) {
        *self = self.opponent();
    }

    /// Yield the other player.
    pub fn opponent(&self) -> Player {
        match self {
            Player::One => Player::Two,
            Player::Two => Player::One,
        }
    }
}
