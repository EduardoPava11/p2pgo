use p2pgo_core::value_labeller::ScoringMethod;
use p2pgo_core::{scoring, Color, Coord, GameState};
use std::collections::HashSet;

fn create_test_board(size: u8, stones: &[(u8, u8, Option<Color>)]) -> GameState {
    let mut game_state = GameState::new(size);

    for (x, y, color) in stones {
        let index = *y as usize * size as usize + *x as usize;
        game_state.board[index] = *color;
    }

    game_state
}

#[test]
fn test_empty_board_territory_scoring() {
    let game_state = GameState::new(9);
    let dead_stones = HashSet::new();

    let score =
        scoring::calculate_final_score(&game_state, 6.5, ScoringMethod::Territory, &dead_stones);

    assert_eq!(score.territory_black, 0);
    assert_eq!(score.territory_white, 0);
    assert_eq!(score.captures_black, 0);
    assert_eq!(score.captures_white, 0);
    assert_eq!(score.final_score, -7); // -6.5 rounded to -7
}

#[test]
fn test_simple_territory() {
    // Create a small board with black surrounding some territory
    // B B B . .
    // B . B . .
    // B B B . .
    // . . . . .
    // . . . . .
    let stones = vec![
        (0, 0, Some(Color::Black)),
        (1, 0, Some(Color::Black)),
        (2, 0, Some(Color::Black)),
        (0, 1, Some(Color::Black)),
        (2, 1, Some(Color::Black)),
        (0, 2, Some(Color::Black)),
        (1, 2, Some(Color::Black)),
        (2, 2, Some(Color::Black)),
    ];

    let game_state = create_test_board(5, &stones);
    let dead_stones = HashSet::new();

    let score =
        scoring::calculate_final_score(&game_state, 6.5, ScoringMethod::Territory, &dead_stones);

    assert_eq!(score.territory_black, 1); // (1,1) is surrounded by Black
    assert_eq!(score.territory_white, 0);
    assert_eq!(score.final_score, -6); // 1 - 0 - 6.5 = -5.5, rounded to -6
}

#[test]
fn test_area_scoring() {
    // Same board as test_simple_territory
    let stones = vec![
        (0, 0, Some(Color::Black)),
        (1, 0, Some(Color::Black)),
        (2, 0, Some(Color::Black)),
        (0, 1, Some(Color::Black)),
        (2, 1, Some(Color::Black)),
        (0, 2, Some(Color::Black)),
        (1, 2, Some(Color::Black)),
        (2, 2, Some(Color::Black)),
    ];

    let game_state = create_test_board(5, &stones);
    let dead_stones = HashSet::new();

    let score = scoring::calculate_final_score(&game_state, 6.5, ScoringMethod::Area, &dead_stones);

    // In area scoring, black gets 8 stones + 1 territory point
    assert_eq!(score.territory_black, 1);
    assert_eq!(score.final_score, 3); // (8+1) - 0 - 6.5 = 2.5, rounded to 3
}

#[test]
fn test_dead_stones() {
    // Board with some dead stones
    // B B B . .
    // B W B . .
    // B B B . .
    // . . . . .
    // . . . . .
    let stones = vec![
        (0, 0, Some(Color::Black)),
        (1, 0, Some(Color::Black)),
        (2, 0, Some(Color::Black)),
        (0, 1, Some(Color::Black)),
        (1, 1, Some(Color::White)),
        (2, 1, Some(Color::Black)),
        (0, 2, Some(Color::Black)),
        (1, 2, Some(Color::Black)),
        (2, 2, Some(Color::Black)),
    ];

    let game_state = create_test_board(5, &stones);

    // Mark the white stone as dead
    let mut dead_stones = HashSet::new();
    dead_stones.insert(Coord::new(1, 1));

    let score =
        scoring::calculate_final_score(&game_state, 6.5, ScoringMethod::Territory, &dead_stones);

    assert_eq!(score.territory_black, 1); // The point where the dead white stone was is now territory
    assert_eq!(score.final_score, -6); // 1 - 0 - 6.5 = -5.5, rounded to -6
}

#[test]
fn test_resignation_scoring() {
    let game_state = GameState::new(9);
    let dead_stones = HashSet::new();

    let score = scoring::calculate_final_score(
        &game_state,
        6.5,
        ScoringMethod::Resignation(Color::Black),
        &dead_stones,
    );

    assert_eq!(score.final_score, 100); // Black won by resignation
    assert_eq!(score.method, ScoringMethod::Resignation(Color::Black));

    let score_white = scoring::calculate_final_score(
        &game_state,
        6.5,
        ScoringMethod::Resignation(Color::White),
        &dead_stones,
    );

    assert_eq!(score_white.final_score, -100); // White won by resignation
}

#[test]
fn test_complex_territory() {
    // A more complex board with both black and white territory
    // B B B W W
    // B . B W .
    // B B B W W
    // . W W . .
    // . W . . .
    let stones = vec![
        (0, 0, Some(Color::Black)),
        (1, 0, Some(Color::Black)),
        (2, 0, Some(Color::Black)),
        (3, 0, Some(Color::White)),
        (4, 0, Some(Color::White)),
        (0, 1, Some(Color::Black)),
        (2, 1, Some(Color::Black)),
        (3, 1, Some(Color::White)),
        (0, 2, Some(Color::Black)),
        (1, 2, Some(Color::Black)),
        (2, 2, Some(Color::Black)),
        (3, 2, Some(Color::White)),
        (4, 2, Some(Color::White)),
        (1, 3, Some(Color::White)),
        (2, 3, Some(Color::White)),
        (1, 4, Some(Color::White)),
    ];

    let game_state = create_test_board(5, &stones);
    let dead_stones = HashSet::new();

    let score =
        scoring::calculate_final_score(&game_state, 6.5, ScoringMethod::Territory, &dead_stones);

    assert_eq!(score.territory_black, 1); // (1,1) is black territory
    assert_eq!(score.territory_white, 1); // (4,1) is white territory
    assert_eq!(score.final_score, -7); // 1 - 1 - 6.5 = -6.5, rounded to -7
}
