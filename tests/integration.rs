use monte_carlo_tree_search::{
    CountWdlSolved, GameState, Player, RandomPlayout, Search, TwoPlayerGame, UcbSolver,
};

#[test]
fn player_one_always_wins() {
    /// A rather silly game which is always in a terminal state with player one winning. Not much fun,
    /// but useful for testing.
    #[derive(Clone)]
    struct PlayerOneAlwaysWins;

    impl TwoPlayerGame for PlayerOneAlwaysWins {
        type Move = ();

        fn play(&mut self, _move: &()) {}

        fn state<'a>(&self, _moves_buf: &'a mut Vec<()>) -> GameState<'a, ()> {
            GameState::WinPlayerOne
        }

        fn current_player(&self) -> Player {
            Player::One
        }
    }

    let game = PlayerOneAlwaysWins;

    let tree = Search::new(game, UcbSolver::<RandomPlayout<_>>::new());

    assert_eq!(CountWdlSolved::Win(Player::One), tree.evaluation());
}
