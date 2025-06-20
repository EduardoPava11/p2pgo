# Implementation Guide for Iroh v0.35 Integration

## Key Issues to Fix

1. API Changes in iroh v0.35:
   - The `iroh::net` module structure has changed
   - `iroh_docs::DocId` structure has changed
   - `iroh_gossip::TopicId` is now `iroh_gossip::proto::TopicId`
   - `Router::new()` API has changed
   - `Endpoint::bind_port()` method no longer exists
   - Base64 encoding/decoding requires `base64::Engine` trait in scope

2. Move::None no longer exists:
   - Replace with more appropriate values like `Move::Pass`

3. Field access:
   - Field `docs` of `IrohCtx` is private, need to use accessor methods
   - Entry access patterns for document entries have changed in iroh-docs v0.35

4. Type mismatches:
   - `prev_hash` is `Option<String>` in MoveRecord but `Option<[u8; 32]>` elsewhere

## Implementation Steps

1. Update imports in iroh_endpoint.rs:
```rust
use iroh::{Endpoint, protocol::Router};
use iroh_blobs::Blobs;
use iroh_docs::Docs;
use iroh_gossip::{Gossip, proto::TopicId};
```

2. Update GameChannel.rs:
   - Replace Move::None with Move::Pass
   - Fix type mismatches between `Option<String>` and `Option<[u8; 32]>`

3. Add base64 Engine trait for encoding/decoding:
```rust
use base64::Engine;
```

4. Add missing feature flags in Cargo.toml:
```toml
[features]
iroh = ["dep:iroh", "dep:iroh-docs", "dep:iroh-gossip", "dep:iroh-blobs", "tokio/full", "blake3"]
```

5. Fix API usage patterns:
   - Update document access methods
   - Update gossip subscription methods
   - Ensure proper endpoint creation and connection

The networking functionality is close to working, but requires these API updates to operate correctly with iroh v0.35.
