// SPDX-License-Identifier: MIT OR Apache-2.0

//! CBOR serialization helpers for game state
//! 
//! This module provides functions for serializing and deserializing
//! game state and events using the Concise Binary Object Representation (CBOR).

use crate::{GameState, GameEvent, Move};
use serde::{Serialize, Deserialize};
use serde_repr::{Serialize_repr, Deserialize_repr};
use blake3;
use ed25519_dalek::{Signer, Verifier};

// Helper module for serializing fixed-size byte arrays
mod serde_arrays_64 {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use serde::de::Error;
    use std::convert::TryInto;

    pub fn serialize<S>(bytes: &Option<[u8; 64]>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match bytes {
            Some(array) => array.to_vec().serialize(serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<[u8; 64]>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<Vec<u8>>::deserialize(deserializer)?;
        match opt {
            Some(vec) => {
                let arr: [u8; 64] = vec.try_into()
                    .map_err(|_| D::Error::custom("Expected 64 bytes for signature"))?;
                Ok(Some(arr))
            }
            None => Ok(None),
        }
    }
}

/// Move annotation tags for training
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum Tag {
    Activity = 0,   // proactive, attacking
    Avoidance = 1,  // defensive / territory
    Reactivity = 2, // answer to last move
}

/// A move record with optional annotation tag and timestamp
///
/// # Signatures
/// 
/// MoveRecords can be signed using ed25519 signatures for authenticating the
/// sender of a move. The signature is generated using the sender's ed25519 
/// keypair and covers all fields of the MoveRecord except the signature and
/// signer fields themselves.
///
/// When signing:
/// 1. The `signature` and `signer` fields are set to None
/// 2. The record is serialized to bytes
/// 3. The bytes are signed with the sender's keypair
/// 4. The signature and public key (as hex) are stored in the MoveRecord
///
/// When verifying:
/// 1. A copy of the record is made with `signature` and `signer` set to None
/// 2. The copy is serialized to bytes
/// 3. The signature is verified against these bytes using the signer's public key
///
/// For backward compatibility, unsigned moves are still accepted but logged as warnings.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MoveRecord {
    pub mv: Move,
    pub tag: Option<Tag>,
    pub ts: u64, // unix seconds
    /// Blake3 hash of the CBOR-serialized bytes, set after successful broadcast
    pub broadcast_hash: Option<[u8; 32]>,
    /// Blake3 hash of the previous move record for chain integrity
    pub prev_hash: Option<[u8; 32]>,
    /// Ed25519 signature of the serialized move data
    #[serde(with = "serde_arrays_64")]
    pub signature: Option<[u8; 64]>,
    /// Node ID (public key) of the signer
    pub signer: Option<String>,
    /// Sequence number for this move in the chain
    pub sequence: u32,
}

impl MoveRecord {
    /// Create a new MoveRecord with proper hash chain
    pub fn new(mv: Move, tag: Option<Tag>, ts: u64, prev_hash: Option<[u8; 32]>) -> Self {
            
        let mut record = Self {
            mv,
            tag,
            ts,
            broadcast_hash: None,
            prev_hash,
            signature: None,
            signer: None,
            sequence: 0, // Default to 0, should be updated when adding to chain
        };
        
        // Calculate and set the broadcast hash
        record.calculate_broadcast_hash();
        record
    }
    
    /// Create a new MoveRecord with auto-generated timestamp
    pub fn new_with_timestamp(mv: Move, tag: Option<Tag>, prev_hash: Option<[u8; 32]>) -> Self {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self::new(mv, tag, ts, prev_hash)
    }
    
    /// Calculate and set the broadcast hash for this move record
    pub fn calculate_broadcast_hash(&mut self) {
        // Temporarily clear broadcast_hash to avoid including it in the hash calculation
        let old_hash = self.broadcast_hash.take();
        
        // Serialize without the broadcast_hash
        if let Ok(bytes) = serde_cbor::to_vec(self) {
            let hash = blake3::hash(&bytes);
            self.broadcast_hash = Some(*hash.as_bytes());
        } else {
            // Restore the old hash if serialization failed
            self.broadcast_hash = old_hash;
            tracing::error!("Failed to serialize MoveRecord for hash calculation");
        }
    }
    
    /// Create a pass move record
    pub fn pass(prev_hash: Option<[u8; 32]>) -> Self {
        Self::new_with_timestamp(Move::Pass, None, prev_hash)
    }
    
    /// Create a resign move record  
    pub fn resign(prev_hash: Option<[u8; 32]>) -> Self {
        Self::new_with_timestamp(Move::Resign, None, prev_hash)
    }
    
    /// Create a stone placement move record
    pub fn place(coord: crate::Coord, color: crate::Color, tag: Option<Tag>, prev_hash: Option<[u8; 32]>) -> Self {
        Self::new_with_timestamp(Move::Place { x: coord.x, y: coord.y, color }, tag, prev_hash)
    }
    
    /// Serialize this move record to CBOR bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        match serde_cbor::to_vec(self) {
            Ok(bytes) => bytes,
            Err(err) => {
                tracing::error!("Failed to serialize move record: {}", err);
                Vec::new() // Return empty vector on error
            }
        }
    }
    
    /// Sign the move record using an ed25519 keypair
    pub fn sign(&mut self, keypair: &ed25519_dalek::SigningKey) {
        // Clear any existing signature to not include it in the signing data
        self.signature = None;
        self.signer = None;
        
        // Serialize the move record to bytes without the signature
        let bytes = self.to_bytes();
        
        // Sign the bytes
        let signature = keypair.sign(&bytes);
        
        // Set the signature and signer (derived from verifying key)
        self.signature = Some(signature.to_bytes());
        self.signer = Some(hex::encode(keypair.verifying_key().as_bytes()));
    }
    
    /// Check if this move record is signed
    pub fn is_signed(&self) -> bool {
        self.signature.is_some() && self.signer.is_some()
    }
    
    /// Get the signer's node ID as a hex string, if available
    pub fn get_signer(&self) -> Option<&str> {
        self.signer.as_deref()
    }
    
    /// Verify the signature on this move record
    pub fn verify_signature(&self) -> bool {
        match (&self.signature, &self.signer) {
            (Some(sig), Some(signer)) => {
                // Create a temporary copy of the record without the signature for verification
                let mut temp_record = self.clone();
                temp_record.signature = None;
                temp_record.signer = None;
                
                // Serialize to bytes
                let bytes = temp_record.to_bytes();
                
                // Parse hex signer to get verifying key
                match hex::decode(signer) {
                    Ok(signer_bytes) => {
                        if signer_bytes.len() != 32 {
                            tracing::warn!("Invalid signer key length");
                            return false;
                        }
                        
                        // Convert to [u8; 32] for ed25519_dalek
                        let mut key_bytes = [0u8; 32];
                        key_bytes.copy_from_slice(&signer_bytes);
                        
                        // Create verifying key
                        match ed25519_dalek::VerifyingKey::from_bytes(&key_bytes) {
                            Ok(verifying_key) => {                        // Convert signature bytes to ed25519_dalek::Signature
                        let signature = match ed25519_dalek::Signature::try_from(*sig) {
                            Ok(sig) => sig,
                            Err(e) => {
                                tracing::warn!("Failed to parse signature: {}", e);
                                return false;
                            }
                        };
                        
                        // Verify signature
                        match verifying_key.verify(&bytes, &signature) {
                            Ok(_) => true,
                            Err(e) => {
                                tracing::warn!("Signature verification failed: {}", e);
                                false
                            }
                        }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to create verifying key: {}", e);
                                false
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to decode signer hex: {}", e);
                        false
                    }
                }
            }
            (None, _) => {
                tracing::debug!("No signature present, skipping verification");
                false
            }
            (_, None) => {
                tracing::debug!("No signer present, skipping verification");
                false
            }
        }
    }
    
    /// Verify signature if present, return true if valid or if no signature exists
    pub fn verify_signature_lenient(&self) -> bool {
        if self.is_signed() {
            self.verify_signature()
        } else {
            // No signature to verify, consider it "valid" for backward compatibility
            true
        }
    }
}

/// Serialize game state to CBOR
pub fn serialize_game_state(state: &GameState) -> Vec<u8> {
    match serde_cbor::to_vec(state) {
        Ok(bytes) => bytes,
        Err(err) => {
            tracing::error!("Failed to serialize game state: {}", err);
            Vec::new() // Return empty vector on error
        }
    }
}

/// Deserialize game state from CBOR
pub fn deserialize_game_state(data: &[u8]) -> Option<GameState> {
    if data.is_empty() {
        return None;
    }

    match serde_cbor::from_slice(data) {
        Ok(state) => Some(state),
        Err(err) => {
            tracing::error!("Failed to deserialize game state: {}", err);
            None
        }
    }
}

/// Serialize game event to CBOR
pub fn serialize_game_event(event: &GameEvent) -> Vec<u8> {
    match serde_cbor::to_vec(event) {
        Ok(bytes) => bytes,
        Err(err) => {
            tracing::error!("Failed to serialize game event: {}", err);
            Vec::new() // Return empty vector on error
        }
    }
}

/// Deserialize game event from CBOR
pub fn deserialize_game_event(data: &[u8]) -> Option<GameEvent> {
    if data.is_empty() {
        return None;
    }
    
    match serde_cbor::from_slice(data) {
        Ok(event) => Some(event),
        Err(err) => {
            tracing::error!("Failed to deserialize game event: {}", err);
            None
        }
    }
}


