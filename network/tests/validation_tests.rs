// SPDX-License-Identifier: MIT OR Apache-2.0

use p2pgo_core::{Coord, GameState, Move};
use p2pgo_network::blob_store::{MoveBlob, MoveChain};

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper function to create an empty blob with given sequence
    fn mk_blob(id: &str, seq: u32) -> (GameState, MoveBlob) {
        let gs = GameState::new(9);
        let mv = Move::Pass;

        // For the blob state, apply the move first
        let mut blob_state = gs.clone();
        blob_state.apply_move(mv.clone()).unwrap();

        let blob = MoveBlob::new(id.to_string(), mv, None, blob_state, seq);

        (gs, blob)
    }

    /// Helper function to create a connected blob
    fn mk_connected_blob(
        id: &str,
        prev_blob: &MoveBlob,
        prev_hash: [u8; 32],
        seq: u32,
    ) -> MoveBlob {
        let mv = Move::Place(Coord::new(4, 4));
        let mut gs = prev_blob.state.clone();
        gs.apply_move(mv.clone()).unwrap();
        MoveBlob::new(id.to_string(), mv, Some(prev_hash), gs, seq)
    }

    #[test]
    fn blob_verifies_sequence_hash_pairing() {
        // First blob has sequence 0 and no prev_hash
        let (gs, blob0) = mk_blob("test-game", 0);
        assert!(blob0.verify().is_ok());

        // Second blob has sequence 1 and prev_hash
        let blob1 = mk_connected_blob("test-game", &blob0, blob0.hash(), 1);
        assert!(blob1.verify().is_ok());

        // Invalid: sequence 0 with prev_hash
        let invalid = MoveBlob::new(
            "test-game".to_string(),
            Move::Pass,
            Some([0u8; 32]),
            gs.clone(),
            0,
        );
        assert!(invalid.verify().is_err());
    }

    #[test]
    fn blob_validates_moves() {
        let game_id = "test-moves".to_string();
        let gs0 = GameState::new(9);

        // Valid first move at 4-4
        let mv = Move::Place(Coord::new(4, 4));
        let mut gs1 = gs0.clone();
        gs1.apply_move(mv.clone()).unwrap();

        let b0 = MoveBlob::new(game_id.clone(), mv, None, gs1.clone(), 0);
        assert!(b0.validate_continuation(Some(&gs0), None).is_ok());

        // Invalid: trying to play at occupied point
        let mv = Move::Place(Coord::new(4, 4));
        let b1 = MoveBlob::new(game_id, mv, Some(b0.hash()), gs1.clone(), 1);
        assert!(b1
            .validate_continuation(Some(&gs1), Some(b0.hash()))
            .is_err());
    }

    #[test]
    fn chain_enforces_consistency() {
        let mut chain = MoveChain::new("test-chain".to_string());
        let (_gs0, b0) = mk_blob("test-chain", 0);

        // First blob is fine
        match chain.add_blob(b0.clone()) {
            Ok(_) => (),
            Err(e) => panic!("Expected first blob to be accepted, got error: {}", e),
        }

        // Invalid: wrong game ID
        let other_blob = mk_blob("other-game", 1).1;
        assert!(
            chain.add_blob(other_blob).is_err(),
            "Expected error for wrong game ID"
        );

        // Invalid: disconnected sequence
        let b2 = mk_connected_blob("test-chain", &b0, b0.hash(), 2); // skips 1
        assert!(
            chain.add_blob(b2).is_err(),
            "Expected error for skipped sequence"
        );

        // Valid: correct sequence and hash linkage
        let b1 = mk_connected_blob("test-chain", &b0, b0.hash(), 1);
        match chain.add_blob(b1.clone()) {
            Ok(_) => (),
            Err(e) => panic!("Expected second blob to be accepted, got error: {}", e),
        }

        // Check final chain state
        assert!(chain.verify().is_ok(), "Expected chain validation to pass");
        assert_eq!(chain.current_sequence, 1);
    }
}
