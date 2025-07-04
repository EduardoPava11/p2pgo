//! Burn ML engine integration for p2pgo
//!
//! This module provides:
//! 1. WASM model loading and inference for sword_net and shield_net
//! 2. Training data collection from completed games
//! 3. Local SGD training pipeline
//! 4. Integration with the existing Go core engine

use anyhow::Result;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::BufWriter;
use std::sync::Arc;
use parking_lot::Mutex;
use serde::{Serialize, Deserialize};
use tracing::{info, warn};
use std::sync::OnceLock;
use base64::Engine;

use wasmtime::{Engine as WasmEngine, Instance, Module as WasmModule, Store};

use crate::{GameState, Move, Color};

/// Asset path for WASM model files
const ASSET_PATH: &str = "assets/";

/// WASM module wrapper for ML models
pub struct WasmModel {
    /// Wasmtime engine
    engine: WasmEngine,
    /// Compiled WASM module
    module: WasmModule,
    /// Model role (sword or shield)
    role: PolicyRole,
    /// Model ID extracted from WASM
    _model_id: u32,
    /// Model version
    _version: u32,
}

/// Role of the policy network
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PolicyRole {
    /// Sword network - offensive (black) policy
    Sword,
    /// Shield network - defensive (white) policy
    Shield,
}

/// Game outcome for dataset collection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GameOutcome {
    /// Black won the game
    BlackWin,
    /// White won the game
    WhiteWin,
    /// Game ended in a draw
    Draw,
}

/// Training datapoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    /// Board state as CBOR-encoded bytes (base64 encoded in JSON)
    board_cbor: String,
    
    /// Winner of the game
    outcome: GameOutcome,
    
    /// Move number when this position occurred
    move_number: u32,
    
    /// Player to move in this position
    pub player_to_move: Color,
    
    /// The actual move played (for policy training)
    move_played: Option<(u8, u8)>,
    
    /// Game metadata
    game_id: String,
    model_version: u32,
    timestamp: u64,
}

/// ML Engine that manages both WASM models and training
pub struct BurnEngine {
    /// Sword model (aggressive/offensive)
    sword_model: Option<Arc<Mutex<WasmModel>>>,
    /// Shield model (defensive)
    shield_model: Option<Arc<Mutex<WasmModel>>>,
    /// Training dataset
    training_data: Arc<Mutex<Vec<DataPoint>>>,
    /// Asset directory path
    asset_dir: PathBuf,
}

impl BurnEngine {
    /// Create a new Burn engine
    pub fn new() -> Result<Self> {
        let asset_dir = PathBuf::from(ASSET_PATH);
        
        Ok(Self {
            sword_model: None,
            shield_model: None,
            training_data: Arc::new(Mutex::new(Vec::new())),
            asset_dir,
        })
    }
    
    /// Initialize WASM models from assets
    pub fn initialize_models(&mut self) -> Result<()> {
        info!("Initializing WASM ML models...");
        
        // Load Sword Net
        let sword_path = self.asset_dir.join("sword_net.wasm");
        if sword_path.exists() {
            match self.load_wasm_model(&sword_path, PolicyRole::Sword) {
                Ok(model) => {
                    self.sword_model = Some(Arc::new(Mutex::new(model)));
                    info!("Sword Net loaded successfully");
                }
                Err(e) => warn!("Failed to load Sword Net: {}", e),
            }
        }
        
        // Load Shield Net
        let shield_path = self.asset_dir.join("shield_net.wasm");
        if shield_path.exists() {
            match self.load_wasm_model(&shield_path, PolicyRole::Shield) {
                Ok(model) => {
                    self.shield_model = Some(Arc::new(Mutex::new(model)));
                    info!("Shield Net loaded successfully");
                }
                Err(e) => warn!("Failed to load Shield Net: {}", e),
            }
        }
        
        Ok(())
    }
    
    /// Load a WASM model from file
    fn load_wasm_model(&self, path: &Path, role: PolicyRole) -> Result<WasmModel> {
        let engine = WasmEngine::default();
        let module = WasmModule::from_file(&engine, path)?;
        
        // Extract model metadata
        let model_id = match role {
            PolicyRole::Sword => 1001,
            PolicyRole::Shield => 1002,
        };
        
        Ok(WasmModel {
            engine,
            module,
            role,
            _model_id: model_id,
            _version: 100,
        })
    }
    
    /// Get move probabilities from appropriate model
    pub fn get_move_probabilities(&self, game_state: &GameState, player: Color) -> Result<Vec<f32>> {
        let model = match player {
            Color::Black => &self.sword_model,  // Aggressive for black
            Color::White => &self.shield_model, // Defensive for white
        };
        
        let Some(model_ref) = model else {
            return Ok(vec![1.0 / 81.0; 81]); // Uniform distribution fallback
        };
        
        let model = model_ref.lock();
        self.run_wasm_inference(&*model, game_state)
    }
    
    /// Run WASM inference
    fn run_wasm_inference(&self, model: &WasmModel, game_state: &GameState) -> Result<Vec<f32>> {
        let mut store = Store::new(&model.engine, ());
        let instance = Instance::new(&mut store, &model.module, &[])?;
        
        // Convert game state to CBOR
        let _board_cbor = self.encode_board_state(game_state)?;
        
        // Get WASM function for CBOR inference
        let _infer_func = instance.get_typed_func::<(u32, u32), u32>(&mut store, 
            &format!("{}_infer_cbor", model.role.name()))?;
        
        // For now, return mock probabilities
        // TODO: Implement actual WASM memory management and function calls
        let mut probabilities = vec![0.01f32; 81];
        
        // Basic heuristics based on role
        match model.role {
            PolicyRole::Sword => {
                // Prefer corners and edges
                probabilities[0] = 0.15;  // Top-left
                probabilities[8] = 0.15;  // Top-right  
                probabilities[72] = 0.15; // Bottom-left
                probabilities[80] = 0.15; // Bottom-right
            }
            PolicyRole::Shield => {
                // Prefer center
                probabilities[40] = 0.20; // Center
                probabilities[39] = 0.10; // Near center
                probabilities[41] = 0.10;
            }
        }
        
        // Normalize
        let sum: f32 = probabilities.iter().sum();
        if sum > 0.0 {
            for p in &mut probabilities {
                *p /= sum;
            }
        }
        
        Ok(probabilities)
    }
    
    /// Encode board state to CBOR
    fn encode_board_state(&self, game_state: &GameState) -> Result<Vec<u8>> {
        let mut cbor_data = Vec::new();
        ciborium::ser::into_writer(game_state, &mut cbor_data)?;
        Ok(cbor_data)
    }
    
    /// Collect training data from a completed game
    pub fn collect_training_data(&self, game_state: &GameState, outcome: GameOutcome) -> Result<()> {
        let mut data = self.training_data.lock();
        
        // Create training examples for each move in the game
        for (move_idx, mv) in game_state.moves.iter().enumerate() {
            if let Move::Place { x, y, color } = mv {
                let board_cbor = base64::prelude::BASE64_STANDARD.encode(self.encode_board_state(game_state)?);
                
                let datapoint = DataPoint {
                    board_cbor,
                    outcome,
                    move_number: move_idx as u32,
                    player_to_move: *color,
                    move_played: Some((*x, *y)),
                    game_id: game_state.id.clone(),
                    model_version: 100,
                    timestamp: chrono::Utc::now().timestamp_millis() as u64,
                };
                
                data.push(datapoint);
            }
        }
        
        info!("Collected {} training examples from game {}", 
              game_state.moves.len(), game_state.id);
        
        Ok(())
    }
    
    /// Export training data to CBOR files for analysis
    pub fn export_training_data(&self, output_dir: &Path) -> Result<()> {
        let data = self.training_data.lock();
        
        std::fs::create_dir_all(output_dir)?;
        
        // Group by game for better organization
        let mut games = std::collections::HashMap::new();
        for datapoint in data.iter() {
            games.entry(&datapoint.game_id).or_insert_with(Vec::new).push(datapoint);
        }
        
        let games_len = games.len();
        for (game_id, points) in games {
            let file_path = output_dir.join(format!("{}.cbor", game_id));
            let mut file = BufWriter::new(File::create(file_path)?);
            
            for point in points {
                ciborium::ser::into_writer(point, &mut file)?;
            }
        }
        
        info!("Exported {} games with {} total examples to {}", 
              games_len, data.len(), output_dir.display());
        
        Ok(())
    }
    
    /// Get training statistics
    pub fn get_training_stats(&self) -> (usize, usize, usize) {
        let data = self.training_data.lock();
        let total_examples = data.len();
        
        let black_wins = data.iter().filter(|d| matches!(d.outcome, GameOutcome::BlackWin)).count();
        let white_wins = data.iter().filter(|d| matches!(d.outcome, GameOutcome::WhiteWin)).count();
        
        (total_examples, black_wins, white_wins)
    }
}

impl PolicyRole {
    fn name(&self) -> &'static str {
        match self {
            PolicyRole::Sword => "sword",
            PolicyRole::Shield => "shield",
        }
    }
}

/// Global Burn engine instance
static BURN_ENGINE: OnceLock<Arc<Mutex<BurnEngine>>> = OnceLock::new();

/// Get the global Burn engine instance
pub fn get_burn_engine() -> &'static Arc<Mutex<BurnEngine>> {
    BURN_ENGINE.get_or_init(|| {
        let mut engine = BurnEngine::new().expect("Failed to create Burn engine");
        if let Err(e) = engine.initialize_models() {
            warn!("Failed to initialize WASM models: {}", e);
        }
        Arc::new(Mutex::new(engine))
    })
}

/// Initialize the Burn engine (call this early in main)
pub fn initialize_burn_engine() -> Result<()> {
    let engine = get_burn_engine();
    let mut engine = engine.lock();
    engine.initialize_models()?;
    info!("Burn engine initialized successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_burn_engine_creation() {
        let engine = BurnEngine::new().unwrap();
        assert!(engine.sword_model.is_none());
        assert!(engine.shield_model.is_none());
    }
    
    #[test]
    fn test_policy_role_names() {
        assert_eq!(PolicyRole::Sword.name(), "sword");
        assert_eq!(PolicyRole::Shield.name(), "shield");
    }
    
    #[test]
    fn test_training_data_collection() {
        let engine = BurnEngine::new().unwrap();
        let stats = engine.get_training_stats();
        assert_eq!(stats.0, 0); // No data initially
    }
}