// SPDX-License-Identifier: MIT OR Apache-2.0

//! CBOR roundtrip tests for MoveRecord

use p2pgo_core::{MoveRecord, Tag, Move, Coord};
use serde_cbor;

#[test]
fn test_move_record_cbor_roundtrip() {
    let original = MoveRecord {
        mv: Move::Place(Coord::new(4, 4)),
        tag: Some(Tag::Activity),
        ts: 1234567890,
        broadcast_hash: None,
        prev_hash: None,
    };
    
    // Serialize to CBOR
    let cbor_data = serde_cbor::to_vec(&original).expect("Failed to serialize to CBOR");
    
    // Deserialize from CBOR
    let deserialized: MoveRecord = serde_cbor::from_slice(&cbor_data)
        .expect("Failed to deserialize from CBOR");
    
    // Verify roundtrip
    assert_eq!(original.mv, deserialized.mv);
    assert_eq!(original.tag, deserialized.tag);
    assert_eq!(original.ts, deserialized.ts);
    assert_eq!(original.broadcast_hash, deserialized.broadcast_hash);
    assert_eq!(original.prev_hash, deserialized.prev_hash);
}

#[test]
fn test_tag_enum_values() {
    // Verify tag enum values match expected
    assert_eq!(Tag::Activity as u8, 0);
    assert_eq!(Tag::Avoidance as u8, 1);
    assert_eq!(Tag::Reactivity as u8, 2);
}

#[test]
fn test_move_record_without_tag() {
    let original = MoveRecord {
        mv: Move::Pass,
        tag: None,
        ts: 9876543210,
        broadcast_hash: None,
        prev_hash: None,
    };
    
    let cbor_data = serde_cbor::to_vec(&original).expect("Failed to serialize");
    let deserialized: MoveRecord = serde_cbor::from_slice(&cbor_data)
        .expect("Failed to deserialize");
    
    assert_eq!(original.mv, deserialized.mv);
    assert_eq!(original.tag, deserialized.tag);
    assert_eq!(original.ts, deserialized.ts);
    assert_eq!(original.broadcast_hash, deserialized.broadcast_hash);
    assert_eq!(original.prev_hash, deserialized.prev_hash);
}

#[test]
fn test_all_tag_variants_roundtrip() {
    let tags = vec![Tag::Activity, Tag::Avoidance, Tag::Reactivity];
    
    for tag in tags {
        let original = MoveRecord {
            mv: Move::Place(Coord::new(0, 0)),
            tag: Some(tag),
            ts: 1000,
            broadcast_hash: None,
            prev_hash: None,
        };
        
        let cbor_data = serde_cbor::to_vec(&original).expect("Failed to serialize");
        let deserialized: MoveRecord = serde_cbor::from_slice(&cbor_data)
            .expect("Failed to deserialize");
        
        assert_eq!(original.tag, deserialized.tag);
    }
}
