// SPDX-License-Identifier: MIT OR Apache-2.0

//! Tests for ed25519 signatures in MoveRecord

use p2pgo_core::{MoveRecord, Move, Coord};
use ed25519_dalek::{SigningKey, VerifyingKey, Signature};

#[test]
fn test_move_record_sign_verify() {
    // Create a signing key for testing
    let mut rng = rand::thread_rng();
    let signing_key = SigningKey::generate(&mut rng);
    let verifying_key = signing_key.verifying_key();
    
    // Create a move record
    let mut record = MoveRecord::place(Coord::new(3, 4), None, None);
    
    // Sign the record
    record.sign(&signing_key);
    
    // Verify the record has a signature and signer
    assert!(record.signature.is_some());
    assert!(record.signer.is_some());
    
    // Verify the signature is valid
    assert!(record.verify_signature());
}

#[test]
fn test_invalid_signature() {
    // Create two different signing keys
    let mut rng = rand::thread_rng();
    let signing_key1 = SigningKey::generate(&mut rng);
    let signing_key2 = SigningKey::generate(&mut rng);
    
    // Create and sign a move record with the first key
    let mut record = MoveRecord::place(Coord::new(3, 4), None, None);
    record.sign(&signing_key1);
    
    // Verify the original signature is valid
    assert!(record.verify_signature());
    
    // Replace the signature with one from a different key
    let data = record.to_bytes();
    let signature2 = signing_key2.sign(&data);
    record.signature = Some(signature2.to_bytes());
    
    // The signature should now be invalid
    assert!(!record.verify_signature());
}

#[test]
fn test_signature_after_modification() {
    // Create a signing key
    let mut rng = rand::thread_rng();
    let signing_key = SigningKey::generate(&mut rng);
    
    // Create and sign a move record
    let mut record = MoveRecord::place(Coord::new(3, 4), None, None);
    record.sign(&signing_key);
    
    // Verify the original signature is valid
    assert!(record.verify_signature());
    
    // Modify the record after signing
    record.mv = Move::Place(Coord::new(5, 5));
    
    // The signature should now be invalid
    assert!(!record.verify_signature());
}

#[test]
fn test_round_trip_serialization() {
    // Create a signing key
    let mut rng = rand::thread_rng();
    let signing_key = SigningKey::generate(&mut rng);
    
    // Create and sign a move record
    let mut record = MoveRecord::place(Coord::new(3, 4), None, None);
    record.sign(&signing_key);
    
    // Serialize to CBOR
    let cbor_bytes = serde_cbor::to_vec(&record).expect("Failed to serialize to CBOR");
    
    // Deserialize from CBOR
    let deserialized: MoveRecord = serde_cbor::from_slice(&cbor_bytes).expect("Failed to deserialize from CBOR");
    
    // The signature should still be valid
    assert!(deserialized.verify_signature());
    
    // The move should be the same
    match (record.mv, deserialized.mv) {
        (Move::Place(c1), Move::Place(c2)) => {
            assert_eq!(c1.x(), c2.x());
            assert_eq!(c1.y(), c2.y());
        }
        _ => panic!("Moves don't match after serialization roundtrip"),
    }
}

#[test]
fn test_lenient_verification() {
    // Create a move record without signing it
    let record = MoveRecord::place(Coord::new(3, 4), None, None);
    
    // Normal verification should fail
    assert!(!record.verify_signature());
    
    // Lenient verification should pass
    assert!(record.verify_signature_lenient());
    
    // Confirm it's not signed
    assert!(!record.is_signed());
    assert_eq!(record.get_signer(), None);
}
