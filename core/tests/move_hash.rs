// SPDX-License-Identifier: MIT OR Apache-2.0

//! Tests for MoveRecord broadcast hash functionality
#![deny(clippy::all)]

use p2pgo_core::{Move, Coord, Color, cbor::MoveRecord};
use blake3;

#[test]
fn test_move_record_broadcast_hash() {
    // Create a move record without a broadcast hash
    let mut record = MoveRecord::new(
        Move::Place(Coord::new(3, 3)),
        None,
        1625097600, // Example timestamp
        None, // prev_hash
    );
    
    // Serialize to CBOR
    let bytes = serde_cbor::to_vec(&record).unwrap();
    
    // Compute Blake3 hash
    let hash = blake3::hash(&bytes);
    
    // Set the broadcast hash
    record.broadcast_hash = Some(*hash.as_bytes());
    
    // Verify the hash is set correctly
    assert!(record.broadcast_hash.is_some());
    assert_eq!(record.broadcast_hash.unwrap(), *hash.as_bytes());
    
    // Changing the move should invalidate the hash
    let new_bytes = serde_cbor::to_vec(&record).unwrap();
    let new_hash = blake3::hash(&new_bytes);
    
    // The hash should be different now because the record includes the broadcast_hash
    assert_ne!(*hash.as_bytes(), *new_hash.as_bytes());
}

#[test]
fn test_move_record_chain_integrity() {
    // Create first move record
    let mut record1 = MoveRecord::new(
        Move::Place(Coord::new(3, 3)),
        None,
        1625097600,
        None, // prev_hash
    );
    
    // Serialize and compute hash
    let bytes1 = serde_cbor::to_vec(&record1).unwrap();
    let hash1 = blake3::hash(&bytes1);
    record1.broadcast_hash = Some(*hash1.as_bytes());
    
    // Create second move that references the first
    let mut record2 = MoveRecord::new(
        Move::Place(Coord::new(4, 4)),
        None,
        1625097610,
        Some(*hash1.as_bytes()), // Reference to previous move
    );
    
    // Serialize and compute hash
    let bytes2 = serde_cbor::to_vec(&record2).unwrap();
    let hash2 = blake3::hash(&bytes2);
    record2.broadcast_hash = Some(*hash2.as_bytes());
    
    // Verify chain integrity
    assert_eq!(record2.prev_hash.unwrap(), *hash1.as_bytes());
}
