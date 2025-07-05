// SPDX-License-Identifier: MIT OR Apache-2.0

//! Tests for move record hash chain integrity

#[cfg(test)]
mod tests {
    use blake3;
    use p2pgo_core::engine::{calculate_move_hash, MoveRecord};
    use proptest::{collection::vec, prelude::*};
    use quickcheck::{quickcheck, Arbitrary, Gen};
    use std::collections::HashSet;
    use std::iter;

    // Helper to create a move record with proper hash chain
    fn create_move_with_hash(
        prev_hash: Option<[u8; 32]>,
        coord: (u8, u8),
        player: u8,
    ) -> MoveRecord {
        let mut record = MoveRecord::new(coord.0, coord.1, player);

        // Set previous hash if provided
        if let Some(hash) = prev_hash {
            record.prev_hash = hash;
        }

        // Calculate and set the broadcast hash
        let move_data = record.to_bytes();
        record.broadcast_hash = calculate_move_hash(&record.prev_hash, &move_data);

        record
    }

    #[test]
    fn test_move_record_hash_chain() {
        // Create a chain of 10 moves
        let mut moves = Vec::new();
        let mut prev_hash = None;

        for i in 0..10 {
            let player = (i % 2) as u8 + 1; // Alternate between player 1 and 2
            let coord = ((i / 9) as u8, (i % 9) as u8); // Coordinates from (0,0) to (1,0)

            let record = create_move_with_hash(prev_hash, coord, player);
            prev_hash = Some(record.broadcast_hash);
            moves.push(record);
        }

        // Verify the hash chain integrity
        for i in 1..moves.len() {
            assert_eq!(
                moves[i].prev_hash,
                moves[i - 1].broadcast_hash,
                "Hash chain broken at move {}",
                i
            );
        }
    }

    #[test]
    fn test_calculate_move_hash() {
        let prev_hash = [0u8; 32];
        let move_data = vec![1, 2, 3, 4, 5];

        // Calculate hash using our function
        let hash1 = calculate_move_hash(&prev_hash, &move_data);

        // Calculate expected hash using blake3 directly
        let mut hasher = blake3::Hasher::new();
        hasher.update(&prev_hash);
        hasher.update(&move_data);
        let hash2 = hasher.finalize().into();

        assert_eq!(
            hash1, hash2,
            "Hash calculation doesn't match expected result"
        );
    }

    // Simple arbitrary implementation for MoveRecord
    #[derive(Clone, Debug)]
    struct ArbMoveRecord(MoveRecord);

    impl Arbitrary for ArbMoveRecord {
        fn arbitrary(g: &mut Gen) -> Self {
            let x = u8::arbitrary(g) % 19; // Limit to 19x19 board
            let y = u8::arbitrary(g) % 19;
            let player = (u8::arbitrary(g) % 2) + 1; // Player 1 or 2

            // Generate random previous hash
            let mut prev_hash = [0u8; 32];
            for byte in prev_hash.iter_mut() {
                *byte = u8::arbitrary(g);
            }

            let mut record = MoveRecord::new(x, y, player);
            record.prev_hash = prev_hash;

            // Calculate proper broadcast hash
            let move_data = record.to_bytes();
            record.broadcast_hash = calculate_move_hash(&record.prev_hash, &move_data);

            ArbMoveRecord(record)
        }
    }

    #[test]
    fn prop_hash_uniqueness() {
        fn test(records: Vec<ArbMoveRecord>) -> bool {
            // Extract unique moves only (ignore duplicates)
            let unique_moves: HashSet<_> =
                records.iter().map(|r| (r.0.x, r.0.y, r.0.player)).collect();

            // Skip test if we don't have enough unique moves
            if unique_moves.len() < 2 {
                return true;
            }

            // Get the broadcast hashes
            let hashes: Vec<_> = records.iter().map(|r| r.0.broadcast_hash).collect();

            // Check for hash collisions
            let unique_hashes: HashSet<_> = hashes.iter().collect();
            unique_hashes.len() == hashes.len()
        }

        quickcheck(test as fn(Vec<ArbMoveRecord>) -> bool);
    }

    proptest! {
        #[test]
        fn prop_hash_chain_valid(
            moves in vec((0..19u8, 0..19u8, 1..3u8), 1..20),
        ) {
            let mut chain = Vec::new();
            let mut prev_hash = None;

            for &(x, y, player) in &moves {
                let record = create_move_with_hash(prev_hash, (x, y), player);
                prev_hash = Some(record.broadcast_hash);
                chain.push(record);
            }

            // Verify integrity
            for i in 1..chain.len() {
                prop_assert_eq!(chain[i].prev_hash, chain[i-1].broadcast_hash);

                // Also verify hash calculation is correct
                let recalculated = calculate_move_hash(
                    &chain[i-1].broadcast_hash,
                    &chain[i].to_bytes()
                );
                prop_assert_eq!(recalculated, chain[i].broadcast_hash);
            }
        }
    }

    #[test]
    fn test_tamper_detection() {
        // Create a chain of moves
        let mut chain = Vec::new();
        let mut prev_hash = None;

        for i in 0..5 {
            let player = (i % 2) as u8 + 1;
            let record = create_move_with_hash(prev_hash, (i as u8, i as u8), player);
            prev_hash = Some(record.broadcast_hash);
            chain.push(record);
        }

        // Now tamper with move 3's coordinate
        let mut tampered_chain = chain.clone();
        tampered_chain[2].x += 1;

        // Create proper hashes for the rest of the chain
        for i in 3..tampered_chain.len() {
            let prev_hash = tampered_chain[i - 1].broadcast_hash;
            let move_data = tampered_chain[i].to_bytes();
            tampered_chain[i].prev_hash = prev_hash;
            tampered_chain[i].broadcast_hash = calculate_move_hash(&prev_hash, &move_data);
        }

        // Check that the tampered chain has different hashes than original
        assert_ne!(
            chain.last().unwrap().broadcast_hash,
            tampered_chain.last().unwrap().broadcast_hash,
            "Tampered chain should have different final hash"
        );

        // But the tampered chain should still maintain internal integrity
        for i in 1..tampered_chain.len() {
            assert_eq!(
                tampered_chain[i].prev_hash,
                tampered_chain[i - 1].broadcast_hash,
                "Tampered chain lost internal hash integrity"
            );
        }
    }
}
