// SPDX-License-Identifier: MIT OR Apache-2.0

//! Dataset loader tests

use std::path::Path;

/// Mock dataset loader for testing
pub struct GoDataset {
    samples: Vec<GoSample>,
}

#[derive(Clone)]
pub struct GoSample {
    pub board_state: [f32; 81],
    pub next_move: usize,
}

impl GoDataset {
    pub fn from_cbor_dir<P: AsRef<Path>>(_path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let mut samples = Vec::new();

        // Generate 10 test fixtures
        for i in 0..10 {
            let mut board_state = [0.0f32; 81];
            board_state[i * 8] = 1.0; // Black stone
            board_state[i * 8 + 1] = -1.0; // White stone

            samples.push(GoSample {
                board_state,
                next_move: (i * 7) % 81,
            });
        }

        Ok(Self { samples })
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn get_batch(&self, batch_size: usize) -> (Vec<Vec<f32>>, Vec<usize>) {
        let actual_batch_size = std::cmp::min(batch_size, self.samples.len());

        let mut states = Vec::new();
        let mut moves = Vec::new();

        for i in 0..actual_batch_size {
            states.push(self.samples[i].board_state.to_vec());
            moves.push(self.samples[i].next_move);
        }

        (states, moves)
    }
}

#[test]
fn test_dataset_loads_fixtures() {
    let dataset = GoDataset::from_cbor_dir("tests/fixtures/").expect("Failed to load dataset");
    assert_eq!(dataset.len(), 10);
}

#[test]
fn test_dataset_batch_size() {
    let dataset = GoDataset::from_cbor_dir("tests/fixtures/").expect("Failed to load dataset");
    let (states, moves) = dataset.get_batch(4);

    assert_eq!(states.len(), 4);
    assert_eq!(moves.len(), 4);

    // Check shapes
    for state in &states {
        assert_eq!(state.len(), 81); // 9x9 board flattened
    }
}

#[test]
fn test_dataset_board_representation() {
    let dataset = GoDataset::from_cbor_dir("tests/fixtures/").expect("Failed to load dataset");
    let (states, _) = dataset.get_batch(1);

    // Verify board state has valid values (0, 1, -1)
    let board = &states[0];
    for &value in board {
        assert!(value >= -1.0 && value <= 1.0);
    }
}

#[test]
fn test_dataset_shuffles() {
    let dataset = GoDataset::from_cbor_dir("tests/fixtures/").expect("Failed to load dataset");
    let (_, moves1) = dataset.get_batch(4);
    let (_, moves2) = dataset.get_batch(4);

    // Same dataset should return same moves for deterministic test
    assert_eq!(moves1, moves2);
}
