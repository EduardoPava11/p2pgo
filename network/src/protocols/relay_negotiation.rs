//! Relay negotiation protocol for Circuit Relay V2
//!
//! Implements a reciprocal relay agreement protocol where peers
//! negotiate relay services based on mutual benefit rather than
//! centralized coordination.

use anyhow::{anyhow, Result};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Protocol for negotiating relay agreements
pub const RELAY_NEGOTIATION_PROTOCOL: &str = "/p2pgo/relay-negotiation/1.0.0";

/// Relay negotiation protocol handler
pub struct RelayNegotiationProtocol {
    /// Our relay offerings
    our_relay_offer: RelayOffer,

    /// Active relay agreements
    agreements: HashMap<PeerId, RelayAgreement>,

    /// Relay usage tracking
    usage_tracker: RelayUsageTracker,

    /// Reciprocity calculator
    reciprocity: ReciprocityCalculator,
}

/// Relay service offer from a peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayOffer {
    /// Peer offering relay service (as string for serialization)
    pub peer_id: String,

    /// Relay capacity
    pub capacity: RelayCapacity,

    /// Terms of service
    pub terms: RelayTerms,

    /// Validity period
    pub valid_until: u64, // Unix timestamp
}

/// Relay capacity specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayCapacity {
    /// Maximum concurrent connections
    pub max_connections: u32,

    /// Maximum bandwidth in MB/s
    pub max_bandwidth_mbps: f64,

    /// Maximum data per month in GB
    pub monthly_quota_gb: Option<f64>,

    /// Geographic location (optional)
    pub location: Option<GeographicLocation>,
}

/// Geographic location for latency optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeographicLocation {
    /// Country code
    pub country: String,

    /// Region/state
    pub region: Option<String>,

    /// Approximate latitude
    pub lat: f64,

    /// Approximate longitude
    pub lng: f64,
}

/// Terms for relay service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayTerms {
    /// Reciprocity requirement
    pub reciprocity: ReciprocityRequirement,

    /// Priority levels
    pub priority_tiers: Vec<PriorityTier>,

    /// Accepted protocols
    pub supported_protocols: Vec<String>,
}

/// Reciprocity requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReciprocityRequirement {
    /// No reciprocity required (altruistic)
    None,

    /// Balanced usage (1:1 ratio)
    Balanced { tolerance: f64 },

    /// Minimum relay provision
    MinimumProvision { hours_per_month: f64 },

    /// Reputation-based
    ReputationBased { min_reputation: f64 },
}

/// Priority tiers for relay access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityTier {
    /// Tier name
    pub name: String,

    /// Requirements to qualify
    pub requirements: TierRequirements,

    /// Benefits provided
    pub benefits: TierBenefits,
}

/// Requirements for a priority tier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TierRequirements {
    /// Must provide relay service
    RelayProvider { min_uptime_percent: f64 },

    /// Must maintain reciprocity ratio
    ReciprocityRatio { min_ratio: f64 },

    /// Must have reputation score
    Reputation { min_score: f64 },

    /// Combination of requirements
    All(Vec<TierRequirements>),
}

/// Benefits of a priority tier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierBenefits {
    /// Priority level (higher = better)
    pub priority: u8,

    /// Bandwidth allocation percentage
    pub bandwidth_allocation: f64,

    /// Connection limit multiplier
    pub connection_multiplier: f64,
}

/// Active relay agreement between peers
#[derive(Debug, Clone)]
pub struct RelayAgreement {
    /// The peer we have an agreement with
    pub peer_id: PeerId,

    /// Their relay offer
    pub their_offer: RelayOffer,

    /// Our relay offer to them
    pub our_offer: RelayOffer,

    /// Agreement start time
    pub started_at: Instant,

    /// Current status
    pub status: AgreementStatus,

    /// Performance metrics
    pub metrics: RelayMetrics,
}

/// Status of a relay agreement
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgreementStatus {
    /// Agreement is active
    Active,

    /// Agreement is suspended (e.g., for non-reciprocity)
    Suspended { reason: String },

    /// Agreement has expired
    Expired,

    /// Agreement was terminated
    Terminated { reason: String },
}

/// Relay performance metrics
#[derive(Debug, Clone, Default)]
pub struct RelayMetrics {
    /// Data relayed for this peer (bytes)
    pub data_relayed_for_them: u64,

    /// Data they relayed for us (bytes)
    pub data_relayed_by_them: u64,

    /// Successful relay connections
    pub successful_relays: u32,

    /// Failed relay attempts
    pub failed_relays: u32,

    /// Average latency through their relay
    pub avg_latency_ms: Option<f32>,

    /// Uptime percentage
    pub uptime_percent: f32,
}

/// Tracks relay usage across all peers
pub struct RelayUsageTracker {
    /// Usage records per peer
    usage_records: HashMap<PeerId, UsageRecord>,

    /// Global usage stats
    global_stats: GlobalRelayStats,
}

/// Usage record for a specific peer
#[derive(Debug, Clone, Default)]
pub struct UsageRecord {
    /// Total data relayed (bytes)
    pub total_data_relayed: u64,

    /// Total connections relayed
    pub total_connections: u32,

    /// Usage by time period
    pub hourly_usage: Vec<(Instant, u64)>,

    /// Reciprocity balance
    pub reciprocity_balance: i64, // Positive = we owe them, Negative = they owe us
}

/// Global relay statistics
#[derive(Debug, Clone, Default)]
pub struct GlobalRelayStats {
    /// Total data relayed for others
    pub total_data_provided: u64,

    /// Total data relayed by others for us
    pub total_data_consumed: u64,

    /// Number of active relay agreements
    pub active_agreements: u32,

    /// Overall reciprocity ratio
    pub reciprocity_ratio: f64,
}

/// Calculates reciprocity between peers
pub struct ReciprocityCalculator {
    /// Weight factors for different metrics
    weights: ReciprocityWeights,
}

/// Weights for reciprocity calculation
#[derive(Debug, Clone)]
pub struct ReciprocityWeights {
    /// Weight for data volume
    pub data_weight: f64,

    /// Weight for connection count
    pub connection_weight: f64,

    /// Weight for uptime
    pub uptime_weight: f64,

    /// Weight for latency
    pub latency_weight: f64,
}

impl Default for ReciprocityWeights {
    fn default() -> Self {
        Self {
            data_weight: 0.4,
            connection_weight: 0.2,
            uptime_weight: 0.3,
            latency_weight: 0.1,
        }
    }
}

/// Negotiation messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NegotiationMessage {
    /// Propose a relay agreement
    ProposeAgreement { offer: RelayOffer },

    /// Accept an agreement
    AcceptAgreement {
        offer_hash: [u8; 32],
        counter_offer: RelayOffer,
    },

    /// Reject an agreement
    RejectAgreement {
        offer_hash: [u8; 32],
        reason: String,
    },

    /// Update existing agreement
    UpdateAgreement { updated_offer: RelayOffer },

    /// Request current metrics
    RequestMetrics,

    /// Share metrics (metrics not serializable, so just send summary)
    ShareMetrics {
        data_relayed_for_them: u64,
        data_relayed_by_them: u64,
        uptime_percent: f32,
    },

    /// Suspend agreement
    SuspendAgreement { reason: String },

    /// Resume agreement
    ResumeAgreement,

    /// Terminate agreement
    TerminateAgreement { reason: String },
}

impl RelayNegotiationProtocol {
    /// Create a new relay negotiation protocol
    pub fn new(our_capacity: RelayCapacity) -> Self {
        let our_relay_offer = RelayOffer {
            peer_id: PeerId::random().to_string(), // Would be our actual peer ID
            capacity: our_capacity,
            terms: RelayTerms {
                reciprocity: ReciprocityRequirement::Balanced { tolerance: 0.2 },
                priority_tiers: Self::default_priority_tiers(),
                supported_protocols: vec![
                    "/p2pgo/game/1.0.0".to_string(),
                    "/p2pgo/sync/1.0.0".to_string(),
                ],
            },
            valid_until: (Instant::now() + Duration::from_secs(86400))
                .elapsed()
                .as_secs(),
        };

        Self {
            our_relay_offer,
            agreements: HashMap::new(),
            usage_tracker: RelayUsageTracker {
                usage_records: HashMap::new(),
                global_stats: GlobalRelayStats::default(),
            },
            reciprocity: ReciprocityCalculator {
                weights: ReciprocityWeights::default(),
            },
        }
    }

    /// Default priority tiers
    fn default_priority_tiers() -> Vec<PriorityTier> {
        vec![
            PriorityTier {
                name: "Contributor".to_string(),
                requirements: TierRequirements::RelayProvider {
                    min_uptime_percent: 80.0,
                },
                benefits: TierBenefits {
                    priority: 3,
                    bandwidth_allocation: 0.4,
                    connection_multiplier: 2.0,
                },
            },
            PriorityTier {
                name: "Balanced".to_string(),
                requirements: TierRequirements::ReciprocityRatio { min_ratio: 0.8 },
                benefits: TierBenefits {
                    priority: 2,
                    bandwidth_allocation: 0.3,
                    connection_multiplier: 1.5,
                },
            },
            PriorityTier {
                name: "Basic".to_string(),
                requirements: TierRequirements::Reputation { min_score: 0.0 },
                benefits: TierBenefits {
                    priority: 1,
                    bandwidth_allocation: 0.2,
                    connection_multiplier: 1.0,
                },
            },
        ]
    }

    /// Propose a relay agreement to a peer
    pub async fn propose_agreement(&mut self, peer_id: PeerId) -> Result<()> {
        // Check if we already have an agreement
        if self.agreements.contains_key(&peer_id) {
            return Err(anyhow!("Agreement already exists with peer"));
        }

        // Send proposal
        let _message = NegotiationMessage::ProposeAgreement {
            offer: self.our_relay_offer.clone(),
        };

        // In practice, this would send via libp2p
        info!("Proposing relay agreement to {:?}", peer_id);

        Ok(())
    }

    /// Handle incoming negotiation message
    pub async fn handle_message(
        &mut self,
        from: PeerId,
        message: NegotiationMessage,
    ) -> Result<()> {
        match message {
            NegotiationMessage::ProposeAgreement { offer } => {
                self.handle_proposal(from, offer).await?;
            }
            NegotiationMessage::AcceptAgreement {
                offer_hash,
                counter_offer,
            } => {
                self.handle_acceptance(from, offer_hash, counter_offer)
                    .await?;
            }
            NegotiationMessage::RequestMetrics => {
                self.handle_metrics_request(from).await?;
            }
            NegotiationMessage::ShareMetrics {
                data_relayed_for_them,
                data_relayed_by_them,
                uptime_percent,
            } => {
                // Convert back to metrics struct
                let metrics = RelayMetrics {
                    data_relayed_for_them,
                    data_relayed_by_them,
                    successful_relays: 0,
                    failed_relays: 0,
                    avg_latency_ms: None,
                    uptime_percent,
                };
                self.handle_metrics_update(from, metrics).await?;
            }
            _ => {
                // Handle other message types
            }
        }

        Ok(())
    }

    /// Handle relay agreement proposal
    async fn handle_proposal(&mut self, from: PeerId, offer: RelayOffer) -> Result<()> {
        // Evaluate the offer
        let acceptable = self.evaluate_offer(&offer)?;

        if acceptable {
            // Accept and create agreement
            let agreement = RelayAgreement {
                peer_id: from,
                their_offer: offer.clone(),
                our_offer: self.our_relay_offer.clone(),
                started_at: Instant::now(),
                status: AgreementStatus::Active,
                metrics: RelayMetrics::default(),
            };

            self.agreements.insert(from, agreement);

            // Send acceptance
            let offer_hash = Self::hash_offer(&offer);
            let _message = NegotiationMessage::AcceptAgreement {
                offer_hash,
                counter_offer: self.our_relay_offer.clone(),
            };

            info!("Accepted relay agreement from {:?}", from);
        } else {
            // Reject the offer
            let offer_hash = Self::hash_offer(&offer);
            let _message = NegotiationMessage::RejectAgreement {
                offer_hash,
                reason: "Terms not acceptable".to_string(),
            };

            info!("Rejected relay agreement from {:?}", from);
        }

        Ok(())
    }

    /// Evaluate if an offer is acceptable
    fn evaluate_offer(&self, offer: &RelayOffer) -> Result<bool> {
        // Check if offer is still valid
        let now = Instant::now().elapsed().as_secs();
        if offer.valid_until < now {
            return Ok(false);
        }

        // Check capacity
        if offer.capacity.max_connections < 10 {
            return Ok(false);
        }

        // Check reciprocity requirements
        match &offer.terms.reciprocity {
            ReciprocityRequirement::None => Ok(true),
            ReciprocityRequirement::Balanced { tolerance } => {
                // We can handle balanced reciprocity
                Ok(*tolerance >= 0.1)
            }
            _ => Ok(true), // Accept other types for now
        }
    }

    /// Handle agreement acceptance
    async fn handle_acceptance(
        &mut self,
        from: PeerId,
        _offer_hash: [u8; 32],
        counter_offer: RelayOffer,
    ) -> Result<()> {
        // Verify the offer hash matches what we sent
        // Create agreement
        let agreement = RelayAgreement {
            peer_id: from,
            their_offer: counter_offer,
            our_offer: self.our_relay_offer.clone(),
            started_at: Instant::now(),
            status: AgreementStatus::Active,
            metrics: RelayMetrics::default(),
        };

        self.agreements.insert(from, agreement);
        info!("Relay agreement established with {:?}", from);

        Ok(())
    }

    /// Handle metrics request
    async fn handle_metrics_request(&self, from: PeerId) -> Result<()> {
        if let Some(agreement) = self.agreements.get(&from) {
            let _message = NegotiationMessage::ShareMetrics {
                data_relayed_for_them: agreement.metrics.data_relayed_for_them,
                data_relayed_by_them: agreement.metrics.data_relayed_by_them,
                uptime_percent: agreement.metrics.uptime_percent,
            };
            // Send metrics
        }
        Ok(())
    }

    /// Handle metrics update
    async fn handle_metrics_update(&mut self, from: PeerId, metrics: RelayMetrics) -> Result<()> {
        if let Some(agreement) = self.agreements.get_mut(&from) {
            // Update our view of their performance
            agreement.metrics = metrics;

            // Check reciprocity
            self.check_reciprocity(from)?;
        }
        Ok(())
    }

    /// Check and enforce reciprocity
    fn check_reciprocity(&mut self, peer_id: PeerId) -> Result<()> {
        if let Some(agreement) = self.agreements.get_mut(&peer_id) {
            let ratio = self.reciprocity.calculate_ratio(&agreement.metrics);

            match &agreement.their_offer.terms.reciprocity {
                ReciprocityRequirement::Balanced { tolerance } => {
                    if ratio < (1.0 - tolerance) || ratio > (1.0 + tolerance) {
                        // Suspend agreement for non-reciprocity
                        agreement.status = AgreementStatus::Suspended {
                            reason: format!("Reciprocity ratio {} outside tolerance", ratio),
                        };
                        warn!(
                            "Suspended relay agreement with {:?} due to reciprocity",
                            peer_id
                        );
                    }
                }
                _ => {} // Other reciprocity types
            }
        }

        Ok(())
    }

    /// Calculate hash of an offer for verification
    fn hash_offer(offer: &RelayOffer) -> [u8; 32] {
        use blake3::Hasher;
        let mut hasher = Hasher::new();

        // Hash the serialized offer
        if let Ok(bytes) = serde_json::to_vec(offer) {
            hasher.update(&bytes);
        }

        let hash = hasher.finalize();
        let mut result = [0u8; 32];
        result.copy_from_slice(&hash.as_bytes()[..32]);
        result
    }

    /// Get current relay agreements
    pub fn get_active_agreements(&self) -> Vec<&RelayAgreement> {
        self.agreements
            .values()
            .filter(|a| a.status == AgreementStatus::Active)
            .collect()
    }

    /// Update relay usage statistics
    pub fn record_relay_usage(
        &mut self,
        peer_id: PeerId,
        bytes_relayed: u64,
        direction: RelayDirection,
    ) {
        if let Some(record) = self.usage_tracker.usage_records.get_mut(&peer_id) {
            record.total_data_relayed += bytes_relayed;

            match direction {
                RelayDirection::ForThem => {
                    record.reciprocity_balance += bytes_relayed as i64;
                    self.usage_tracker.global_stats.total_data_provided += bytes_relayed;
                }
                RelayDirection::ForUs => {
                    record.reciprocity_balance -= bytes_relayed as i64;
                    self.usage_tracker.global_stats.total_data_consumed += bytes_relayed;
                }
            }

            // Update hourly usage
            record.hourly_usage.push((Instant::now(), bytes_relayed));

            // Keep only last 24 hours
            let cutoff = Instant::now() - Duration::from_secs(86400);
            record.hourly_usage.retain(|(time, _)| *time > cutoff);
        }

        // Update global reciprocity ratio
        if self.usage_tracker.global_stats.total_data_consumed > 0 {
            self.usage_tracker.global_stats.reciprocity_ratio =
                self.usage_tracker.global_stats.total_data_provided as f64
                    / self.usage_tracker.global_stats.total_data_consumed as f64;
        }
    }
}

/// Direction of relay usage
pub enum RelayDirection {
    /// We relayed data for them
    ForThem,
    /// They relayed data for us
    ForUs,
}

impl ReciprocityCalculator {
    /// Calculate reciprocity ratio from metrics
    pub fn calculate_ratio(&self, metrics: &RelayMetrics) -> f64 {
        let data_ratio = if metrics.data_relayed_by_them > 0 {
            metrics.data_relayed_for_them as f64 / metrics.data_relayed_by_them as f64
        } else {
            f64::INFINITY
        };

        let connection_ratio = if metrics.successful_relays > 0 {
            1.0 // Simplified for now
        } else {
            0.0
        };

        // Weighted average
        data_ratio * self.weights.data_weight
            + connection_ratio * self.weights.connection_weight
            + (metrics.uptime_percent as f64 / 100.0) * self.weights.uptime_weight
    }
}

use tracing::info;
use tracing::warn;
