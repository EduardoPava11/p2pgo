// SPDX-License-Identifier: MIT OR Apache-2.0

//! CBOR serialization helpers for game state
//! 
//! This module provides functions for serializing and deserializing
//! game state and events using the Concise Binary Object Representation (CBOR).

use crate::{GameState, GameEvent, Move};
use serde::{Serialize, Deserialize};
use serde_repr::{Serialize_repr, Deserialize_repr};

/// Move annotation tags for training
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum Tag {
    Activity = 0,   // proactive, attacking
    Avoidance = 1,  // defensive / territory
    Reactivity = 2, // answer to last move
}

/// A move record with optional annotation tag and timestamp
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MoveRecord {
    pub mv: Move,
    pub tag: Option<Tag>,
    pub ts: u64, // unix seconds
    /// Blake3 hash of the CBOR-serialized bytes, set after successful broadcast
    pub broadcast_hash: Option<[u8; 32]>,
    /// Blake3 hash of the previous move record for chain integrity
    pub prev_hash: Option<[u8; 32]>,
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


