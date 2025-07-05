//! Message security layer for P2P communications
//! Ensures all messages are properly signed and validated

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use blake3::Hasher;
use libp2p::PeerId;
use anyhow::{Result, anyhow};

/// Maximum age of a message before it's considered stale
const MAX_MESSAGE_AGE: Duration = Duration::from_secs(300); // 5 minutes

/// Signed message envelope for P2P communications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedMessage<T> {
    /// The actual message payload
    pub payload: T,
    /// Sender's peer ID
    pub sender: String,
    /// Message timestamp (Unix timestamp)
    pub timestamp: u64,
    /// Message nonce for replay protection
    pub nonce: [u8; 16],
    /// Ed25519 signature
    pub signature: Vec<u8>,
}

/// Message security manager
pub struct MessageSecurity {
    /// Our signing key
    signing_key: SigningKey,
    /// Our peer ID
    peer_id: PeerId,
    /// Recently seen message hashes for replay protection
    recent_messages: std::collections::HashSet<[u8; 32]>,
    /// Maximum size of recent messages cache
    max_cache_size: usize,
}

impl MessageSecurity {
    /// Create new message security manager
    pub fn new(signing_key: SigningKey, peer_id: PeerId) -> Self {
        Self {
            signing_key,
            peer_id,
            recent_messages: std::collections::HashSet::new(),
            max_cache_size: 10000,
        }
    }
    
    /// Sign a message
    pub fn sign_message<T: Serialize + Clone>(&mut self, payload: &T) -> Result<SignedMessage<T>> {
        // Generate timestamp
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();
        
        // Generate random nonce
        let mut nonce = [0u8; 16];
        use rand::Rng;
        rand::thread_rng().fill(&mut nonce);
        
        // Create message without signature
        let mut message = SignedMessage {
            payload: payload.clone(),
            sender: self.peer_id.to_string(),
            timestamp,
            nonce,
            signature: vec![],
        };
        
        // Calculate signature
        let sig_data = self.get_signing_data(&message)?;
        let signature = self.signing_key.sign(&sig_data);
        message.signature = signature.to_bytes().to_vec();
        
        // Add to recent messages
        let msg_hash = self.hash_message(&message)?;
        self.add_to_cache(msg_hash);
        
        Ok(message)
    }
    
    /// Verify a received message
    pub fn verify_message<T: Serialize + Clone + for<'de> Deserialize<'de>>(
        &mut self,
        message: &SignedMessage<T>,
        sender_key: &VerifyingKey,
    ) -> Result<()> {
        // Check timestamp
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();
        
        let message_age = current_time.saturating_sub(message.timestamp);
        if message_age > MAX_MESSAGE_AGE.as_secs() {
            return Err(anyhow!("Message too old: {} seconds", message_age));
        }
        
        // Check for replay
        let msg_hash = self.hash_message(message)?;
        if self.recent_messages.contains(&msg_hash) {
            return Err(anyhow!("Duplicate message detected"));
        }
        
        // Verify signature
        let sig_data = self.get_signing_data(message)?;
        let signature = Signature::from_slice(&message.signature)
            .map_err(|e| anyhow!("Invalid signature format: {}", e))?;
        
        sender_key.verify(&sig_data, &signature)
            .map_err(|e| anyhow!("Signature verification failed: {}", e))?;
        
        // Add to recent messages
        self.add_to_cache(msg_hash);
        
        Ok(())
    }
    
    /// Get data to be signed for a message
    fn get_signing_data<T: Serialize + Clone>(&self, message: &SignedMessage<T>) -> Result<Vec<u8>> {
        // Create a copy without signature for signing
        let signing_msg = SignedMessage {
            payload: message.payload.clone(),
            sender: message.sender.clone(),
            timestamp: message.timestamp,
            nonce: message.nonce,
            signature: vec![], // Empty signature for signing
        };
        
        // Serialize to CBOR for consistent encoding
        serde_cbor::to_vec(&signing_msg)
            .map_err(|e| anyhow!("Failed to serialize for signing: {}", e))
    }
    
    /// Hash a message for deduplication
    fn hash_message<T: Serialize>(&self, message: &SignedMessage<T>) -> Result<[u8; 32]> {
        let data = serde_cbor::to_vec(message)?;
        let mut hasher = Hasher::new();
        hasher.update(&data);
        Ok(hasher.finalize().into())
    }
    
    /// Add message hash to cache with size limit
    fn add_to_cache(&mut self, hash: [u8; 32]) {
        if self.recent_messages.len() >= self.max_cache_size {
            // Remove oldest entries (this is simplified, real implementation would use LRU)
            self.recent_messages.clear();
        }
        self.recent_messages.insert(hash);
    }
}

/// Security policy for different message types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityPolicy {
    /// Message must be signed and verified
    Required,
    /// Message signing is optional
    Optional,
    /// No signing required (for public broadcasts)
    None,
}

/// Get security policy for a message type
pub fn get_security_policy(message_type: &str) -> SecurityPolicy {
    match message_type {
        // Game moves must always be signed
        "game_move" | "game_state" | "game_sync" => SecurityPolicy::Required,
        // Chat and social features require signing
        "chat" | "friend_request" | "challenge" => SecurityPolicy::Required,
        // Discovery can be unsigned
        "game_list" | "peer_discovery" | "relay_info" => SecurityPolicy::Optional,
        // Public broadcasts don't need signing
        "network_stats" | "version_info" => SecurityPolicy::None,
        // Default to required for unknown types
        _ => SecurityPolicy::Required,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;
    use rand::RngCore;
    
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestMessage {
        content: String,
        value: u32,
    }
    
    #[test]
    fn test_sign_and_verify() {
        let mut csprng = OsRng;
        let mut key_bytes = [0u8; 32];
        csprng.fill_bytes(&mut key_bytes);
        let signing_key = SigningKey::from_bytes(&key_bytes);
        let verifying_key = signing_key.verifying_key();
        let peer_id = PeerId::random();
        
        let mut security = MessageSecurity::new(signing_key, peer_id);
        
        let msg = TestMessage {
            content: "Hello P2P".to_string(),
            value: 42,
        };
        
        // Sign message
        let signed = security.sign_message(&msg).unwrap();
        
        // Verify message
        let mut verifier_key_bytes = [0u8; 32];
        csprng.fill_bytes(&mut verifier_key_bytes);
        let verifier_key = SigningKey::from_bytes(&verifier_key_bytes);
        let mut verifier = MessageSecurity::new(verifier_key, PeerId::random());
        verifier.verify_message(&signed, &verifying_key).unwrap();
    }
    
    #[test]
    fn test_replay_protection() {
        let mut csprng = OsRng;
        let mut key_bytes = [0u8; 32];
        csprng.fill_bytes(&mut key_bytes);
        let signing_key = SigningKey::from_bytes(&key_bytes);
        let verifying_key = signing_key.verifying_key();
        let peer_id = PeerId::random();
        
        let mut security = MessageSecurity::new(signing_key, peer_id);
        
        let msg = TestMessage {
            content: "Test".to_string(),
            value: 1,
        };
        
        let signed = security.sign_message(&msg).unwrap();
        
        let mut verifier_key_bytes = [0u8; 32];
        csprng.fill_bytes(&mut verifier_key_bytes);
        let verifier_key = SigningKey::from_bytes(&verifier_key_bytes);
        let mut verifier = MessageSecurity::new(verifier_key, PeerId::random());
        
        // First verification should succeed
        verifier.verify_message(&signed, &verifying_key).unwrap();
        
        // Second verification should fail (replay)
        assert!(verifier.verify_message(&signed, &verifying_key).is_err());
    }
    
    #[test]
    fn test_tamper_detection() {
        let mut csprng = OsRng;
        let mut key_bytes = [0u8; 32];
        csprng.fill_bytes(&mut key_bytes);
        let signing_key = SigningKey::from_bytes(&key_bytes);
        let verifying_key = signing_key.verifying_key();
        let peer_id = PeerId::random();
        
        let mut security = MessageSecurity::new(signing_key, peer_id);
        
        let msg = TestMessage {
            content: "Original".to_string(),
            value: 100,
        };
        
        let mut signed = security.sign_message(&msg).unwrap();
        
        // Tamper with message
        signed.payload.value = 200;
        
        let mut verifier_key_bytes = [0u8; 32];
        csprng.fill_bytes(&mut verifier_key_bytes);
        let verifier_key = SigningKey::from_bytes(&verifier_key_bytes);
        let mut verifier = MessageSecurity::new(verifier_key, PeerId::random());
        
        // Verification should fail
        assert!(verifier.verify_message(&signed, &verifying_key).is_err());
    }
}