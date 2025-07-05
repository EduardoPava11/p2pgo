// SPDX-License-Identifier: MIT OR Apache-2.0

//! Blob storage for game state and moves

use crate::{BlobHash, GameId};
use anyhow::Result;
use blake3;
use p2pgo_core::{GameEvent, GameState, Move};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Storage for game-related blobs
pub struct BlobStore {
    /// In-memory storage for blobs
    blobs: HashMap<BlobHash, Vec<u8>>,
}

impl Default for BlobStore {
    fn default() -> Self {
        Self::new()
    }
}

impl BlobStore {
    /// Create a new blob store
    pub fn new() -> Self {
        let _span = tracing::info_span!("network.blob_store", "BlobStore::new").entered();

        Self {
            blobs: HashMap::new(),
        }
    }

    /// Store a game state blob
    pub async fn store_game_state(&self, _state: &GameState) -> Result<BlobHash> {
        let _span =
            tracing::info_span!("network.blob_store", "BlobStore::store_game_state").entered();

        // TODO: Serialize and store the game state

        // Mock blob hash for now
        Ok(BlobHash::new([0u8; 32]))
    }

    /// Retrieve a game state blob
    pub async fn get_game_state(&self, _hash: &BlobHash) -> Result<GameState> {
        let _span =
            tracing::info_span!("network.blob_store", "BlobStore::get_game_state").entered();

        // TODO: Retrieve and deserialize the game state

        // Just return an empty game state for now
        Ok(GameState::new(19))
    }

    /// Store a game event blob
    pub async fn store_event(&self, _event: &GameEvent) -> Result<BlobHash> {
        let _span = tracing::info_span!("network.blob_store", "BlobStore::store_event").entered();

        // TODO: Serialize and store the event

        // Mock blob hash for now
        Ok(BlobHash::new([0u8; 32]))
    }

    /// Retrieve a game event blob
    pub async fn get_event(&self, _hash: &BlobHash) -> Result<GameEvent> {
        let _span = tracing::info_span!("network.blob_store", "BlobStore::get_event").entered();

        // TODO: Retrieve and deserialize the event

        // Just return a dummy event for now
        Ok(GameEvent::ChatMessage {
            from: p2pgo_core::Color::Black,
            message: "Dummy message".to_string(),
        })
    }
}

/// A blob containing a move and the resulting game state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveBlob {
    /// The game ID this move belongs to
    pub game_id: GameId,
    /// The actual move
    pub mv: Move,
    /// The hash of the previous move blob (None for the first move)
    pub prev_hash: Option<[u8; 32]>,
    /// The game state after this move
    pub state: GameState,
    /// The sequence number of this move (0 for the first move)
    pub sequence: u32,
}

impl MoveBlob {
    /// Create a new move blob
    pub fn new(
        game_id: GameId,
        mv: Move,
        prev_hash: Option<[u8; 32]>,
        state: GameState,
        sequence: u32,
    ) -> Self {
        Self {
            game_id,
            mv,
            prev_hash,
            state,
            sequence,
        }
    }

    /// Calculate the hash of this blob
    pub fn hash(&self) -> [u8; 32] {
        // Serialize to CBOR
        let cbor_bytes =
            serde_cbor::to_vec(self).expect("MoveBlob serialization should never fail");

        // Hash with BLAKE3
        *blake3::hash(&cbor_bytes).as_bytes()
    }

    /// Validates that this blob forms a valid continuation of the given previous state
    pub fn validate_continuation(
        &self,
        prev_state: Option<&GameState>,
        prev_blob_hash: Option<[u8; 32]>,
    ) -> Result<()> {
        // If this is a first move (no prev_hash), we just need a starting state
        if self.prev_hash.is_none() {
            if prev_state.is_none() {
                anyhow::bail!("Missing initial state");
            }
        } else {
            // For subsequent moves, we need both state and hash
            if prev_state.is_none() || prev_blob_hash.is_none() {
                anyhow::bail!("Missing previous state or hash");
            }

            // Validate prev hash matches
            if self.prev_hash.unwrap() != prev_blob_hash.unwrap() {
                anyhow::bail!("Previous hash mismatch");
            }
        }

        // Validate that the move can be applied to the previous state
        if cfg!(debug_assertions) {
            if let Some(prev) = prev_state {
                let mut test_state = prev.clone();
                test_state
                    .apply_move(self.mv.clone())
                    .map_err(|e| anyhow::anyhow!("Invalid move: {}", e))?;

                // Compare states by serializing to JSON since GameState doesn't implement PartialEq
                let test_state_json = serde_json::to_string(&test_state)
                    .map_err(|e| anyhow::anyhow!("State serialization failed: {}", e))?;
                let blob_state_json = serde_json::to_string(&self.state)
                    .map_err(|e| anyhow::anyhow!("State serialization failed: {}", e))?;

                if test_state_json != blob_state_json {
                    anyhow::bail!("Move application resulted in different state");
                }
            }
        }

        Ok(())
    }

    /// Verifies the integrity of this blob's contents
    pub fn verify(&self) -> Result<()> {
        // Basic sanity checks
        if self.sequence == 0 && self.prev_hash.is_some() {
            anyhow::bail!("First move cannot have previous hash");
        }
        if self.sequence > 0 && self.prev_hash.is_none() {
            anyhow::bail!("Non-first move must have previous hash");
        }

        // Game state must be valid (has no obvious errors)
        if self.state.board_size < 1 || self.state.board_size > 25 {
            anyhow::bail!("Invalid board size");
        }

        Ok(())
    }
}

/// A chain of move blobs, representing a game history
#[derive(Clone)]
pub struct MoveChain {
    /// The game ID this chain belongs to
    pub game_id: GameId,
    /// All blobs in this chain, indexed by their hash
    blobs: HashMap<[u8; 32], MoveBlob>,
    /// The hash of the current tip of the chain
    current_hash: Option<[u8; 32]>,
    /// The current sequence number
    pub current_sequence: u32,
}

impl MoveChain {
    /// Create a new empty move chain
    pub fn new(game_id: GameId) -> Self {
        Self {
            game_id,
            blobs: HashMap::new(),
            current_hash: None,
            current_sequence: 0,
        }
    }

    /// Add a move blob to the chain
    pub fn add_blob(&mut self, blob: MoveBlob) -> Result<()> {
        let hash = blob.hash();
        tracing::debug!(
            game_id = %blob.game_id,
            sequence = blob.sequence,
            hash = %hex::encode(&hash[..8]), // First 8 bytes for brevity
            move_type = ?blob.mv,
            "Adding blob to chain"
        );

        // Ensure blob is for this game
        if blob.game_id != self.game_id {
            anyhow::bail!(
                "Blob is for a different game: expected {}, got {}",
                self.game_id,
                blob.game_id
            );
        }

        // Check sequence numbers
        match self.current_hash {
            None => {
                if blob.sequence != 0 {
                    anyhow::bail!("First blob must have sequence 0, got {}", blob.sequence);
                }
            }
            Some(_) => {
                if blob.sequence != self.current_sequence + 1 {
                    anyhow::bail!(
                        "Expected blob with sequence {}, got {}",
                        self.current_sequence + 1,
                        blob.sequence
                    );
                }
            }
        }

        // Verify the blob's internal consistency
        if let Err(e) = blob.verify() {
            anyhow::bail!("Blob verification failed: {}", e);
        }

        // Get the current state for validation
        let current_blob = self.current_hash.and_then(|h| self.blobs.get(&h));
        let prev_state = current_blob.map(|b| &b.state);

        // For the first blob, use an empty state
        let empty_state = if prev_state.is_none() {
            Some(GameState::new(blob.state.board_size))
        } else {
            None
        };

        // Validate this blob as a continuation
        if let Err(e) =
            blob.validate_continuation(prev_state.or(empty_state.as_ref()), self.current_hash)
        {
            anyhow::bail!("Continuation validation failed: {}", e);
        }

        // Store the blob
        self.blobs.insert(hash, blob.clone());

        // Update current hash and sequence
        self.current_hash = Some(hash);
        self.current_sequence = blob.sequence;

        Ok(())
    }

    /// Get a blob by its hash
    pub fn get_blob(&self, hash: &[u8; 32]) -> Option<&MoveBlob> {
        self.blobs.get(hash)
    }

    /// Get the current blob
    pub fn current_blob(&self) -> Option<&MoveBlob> {
        self.current_hash
            .as_ref()
            .and_then(|hash| self.blobs.get(hash))
    }

    /// Get all blobs in sequence order (from first to last)
    pub fn get_all_blobs(&self) -> Vec<&MoveBlob> {
        let mut result = Vec::with_capacity(self.blobs.len());
        let mut current_hash = self.current_hash;

        while let Some(hash) = current_hash {
            if let Some(blob) = self.blobs.get(&hash) {
                result.push(blob);
                current_hash = blob.prev_hash;
            } else {
                // This shouldn't happen if the chain is consistent
                break;
            }
        }

        // Reverse to get chronological order (first move first)
        result.reverse();
        result
    }

    /// Verify the entire chain is consistent
    pub fn verify(&self) -> Result<()> {
        // Get all blobs in order
        let blobs = self.get_all_blobs();

        // Check sequence is continuous from 0
        for (i, blob) in blobs.iter().enumerate() {
            if blob.sequence != i as u32 {
                anyhow::bail!("Invalid sequence number at position {}", i);
            }

            // Verify each blob
            blob.verify()?;

            // For non-first blobs, validate continuation
            if i > 0 {
                let prev = blobs[i - 1];
                blob.validate_continuation(Some(&prev.state), Some(prev.hash()))?;
            }
        }

        Ok(())
    }

    /// Get all moves as MoveRecord objects (for testing)
    #[cfg(test)]
    pub fn get_all_move_records(&self) -> Vec<p2pgo_core::MoveRecord> {
        let blobs = self.get_all_blobs();
        let mut records = Vec::with_capacity(blobs.len());

        for blob in blobs {
            // Create a MoveRecord for each blob with proper hash chain
            let mut record = p2pgo_core::MoveRecord {
                mv: blob.mv.clone(),
                tag: None,
                ts: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                broadcast_hash: Some(blob.hash()),
                prev_hash: blob.prev_hash,
                signature: None,
                signer: None,
                sequence: blob.sequence,
            };

            // Calculate the broadcast hash to ensure consistency
            record.calculate_broadcast_hash();

            records.push(record);
        }

        records
    }
}
