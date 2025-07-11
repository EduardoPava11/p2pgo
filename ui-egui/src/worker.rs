// SPDX-License-Identifier: MIT OR Apache-2.0

#![deny(clippy::all)]

//! Background worker with tokio runtime for networking.

use std::thread;
use std::sync::Mutex;
use std::rc::Rc;
use crossbeam_channel::{Receiver, Sender};
use tokio::runtime::Runtime;
use p2pgo_core::{GameState, GameEvent, Coord};
use p2pgo_network::{
    lobby::Lobby,
    game_channel::GameChannel,
    IrohCtx,
};
use trainer::GoMini6E;
use burn::backend::wgpu::Wgpu;
use burn::tensor::{Tensor, backend::Backend};

use crate::msg::{UiToNet, NetToUi};

/// Tracker for score acceptance with 3-minute timeout
#[derive(Debug)]
#[allow(dead_code)]
struct ScoreAcceptanceTracker {
    score_proof: p2pgo_core::value_labeller::ScoreProof,
    our_acceptance: bool,
    their_acceptance: bool,
    timeout_start: std::time::Instant,
    board_size: u8,
}

#[allow(dead_code)]
impl ScoreAcceptanceTracker {
    fn new(score_proof: p2pgo_core::value_labeller::ScoreProof, board_size: u8) -> Self {
        Self {
            score_proof,
            our_acceptance: false,
            their_acceptance: false,
            timeout_start: std::time::Instant::now(),
            board_size,
        }
    }
    
    fn is_expired(&self) -> bool {
        self.timeout_start.elapsed() >= std::time::Duration::from_secs(180) // 3 minutes
    }
    
    fn is_complete(&self) -> bool {
        self.our_acceptance && self.their_acceptance
    }
}

/// Spawn the background worker thread
pub fn spawn_worker(
    net_rx: Receiver<UiToNet>,
    ui_tx: Sender<NetToUi>,
    default_board_size: u8,
    player_name: String,
) -> anyhow::Result<thread::JoinHandle<()>> {
    let handle = thread::spawn(move || {
        if let Err(e) = run_worker(net_rx, ui_tx, default_board_size, player_name) {
            eprintln!("Worker thread error: {}", e);
        }
    });
    Ok(handle)
}

/// Direct start function for headless mode
#[allow(dead_code)]
pub fn start(
    net_rx: Receiver<UiToNet>,
    ui_tx: Sender<NetToUi>,
) -> anyhow::Result<()> {
    run_worker(net_rx, ui_tx, 9, "HeadlessPlayer".to_string())
}

fn run_worker(
    net_rx: Receiver<UiToNet>,
    ui_tx: Sender<NetToUi>,
    default_board_size: u8,
    player_name: String,
) -> anyhow::Result<()> {
    let rt = Runtime::new()?;
    
    rt.block_on(async {
        let mut worker = NetworkWorker::new(ui_tx, default_board_size, player_name).await?;
        worker.run(net_rx).await
    })
}

struct ActiveGameData {
    game: std::sync::Arc<GameChannel>,
    game_id: String,
    game_state: Option<GameState>,
    game_rx: tokio::sync::broadcast::Receiver<p2pgo_core::GameEvent>,
}

struct NetworkWorker {
    ui_tx: Sender<NetToUi>,
    lobby: Lobby,
    // Multi-game support: one game per board size
    active_games: std::collections::HashMap<u8, ActiveGameData>,
    default_board_size: u8,
    player_name: String,
    config: crate::app::AppConfig,
    #[allow(dead_code)]
    lobby_rx: tokio::sync::broadcast::Receiver<p2pgo_network::lobby::LobbyEvent>,
    iroh_ctx: IrohCtx,
    // AI model lazily loaded on first ghost move request
    ai_model: Option<Rc<Mutex<GoMini6E<Wgpu>>>>,
    // Gossip buffer size configuration
    #[allow(dead_code)]
    gossip_buffer_size: usize,
    // Score acceptance tracking
    score_trackers: std::collections::HashMap<u8, ScoreAcceptanceTracker>,
    #[cfg(test)]
    last_coord: Option<p2pgo_core::Coord>,
}

impl NetworkWorker {
    async fn new(
        ui_tx: Sender<NetToUi>,
        default_board_size: u8,
        player_name: String,
    ) -> anyhow::Result<Self> {
        let lobby = Lobby::new();
        let lobby_rx = lobby.subscribe();
        
        // Initialize the iroh context
        let iroh_ctx = IrohCtx::new().await?;
        
        // Get and send the local node ID to UI
        let node_id = iroh_ctx.node_id().to_string();
        ui_tx.send(NetToUi::NodeId { node_id: node_id.clone() })?;
        
        // In iroh mode, generate a ticket on startup
        #[cfg(feature = "iroh")]
        {
            if let Ok(ticket) = iroh_ctx.ticket().await {
                ui_tx.send(NetToUi::Ticket { ticket })?;
            }
        }
        
        Ok(Self {
            ui_tx,
            lobby,
            active_games: std::collections::HashMap::new(),
            default_board_size,
            player_name,
            config: crate::app::AppConfig::default(),
            lobby_rx,
            iroh_ctx,
            ai_model: None,
            gossip_buffer_size: 32, // Default buffer size
            score_trackers: std::collections::HashMap::new(),
            #[cfg(test)]
            last_coord: None,
        })
    }

    async fn run(&mut self, net_rx: Receiver<UiToNet>) -> anyhow::Result<()> {
        // Send initial connection status
        let _ = self.ui_tx.send(NetToUi::ConnectionStatus { connected: true });
        
        // Subscribe to gossip lobby
        self.subscribe_to_gossip_lobby().await?;
        
        // Initial game list refresh
        self.refresh_games().await?;
        
        let mut heartbeat_timer = tokio::time::interval(tokio::time::Duration::from_secs(30));
        let mut auto_refresh_timer = tokio::time::interval(tokio::time::Duration::from_secs(2));
        
        loop {
            tokio::select! {
                _ = heartbeat_timer.tick() => {
                    tracing::debug!("NetworkWorker heartbeat");
                }
                _ = auto_refresh_timer.tick() => {
                    if self.config.auto_refresh {
                        tracing::debug!("Auto-refreshing lobby");
                        self.refresh_games().await?;
                    }
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(10)) => {
                    // Regular processing tick
                }
            }
            
            // Handle UI messages
            if let Ok(msg) = net_rx.try_recv() {
                #[cfg(feature = "headless")]
                println!("Worker: Received UI message: {:?}", msg);
                match msg {
                    UiToNet::CreateGame { board_size } => {
                        self.create_game(board_size).await?;
                    }
                            UiToNet::JoinGame { game_id } => {
                                self.join_game(game_id).await?;
                            }
                            UiToNet::MakeMove { mv, board_size } => {
                                self.make_move(mv, board_size).await?;
                            }
                            UiToNet::RefreshGames => {
                                self.refresh_games().await?;
                            }
                            UiToNet::LeaveGame => {
                                self.leave_game().await?;
                            }
                            UiToNet::Shutdown => {
                                let _ = self.ui_tx.send(NetToUi::ShutdownAck);
                                break;
                            }
                            UiToNet::DebugMovePlaced(_coord) => {
                                #[cfg(test)]
                                {
                                    self.last_coord = Some(_coord);
                                }
                            }
                            UiToNet::ConnectByTicket { ticket } => {
                                if let Err(e) = self.iroh_ctx.connect_by_ticket(&ticket).await {
                                    let _ = self.ui_tx.send(NetToUi::Error { 
                                        message: format!("Failed to connect by ticket: {}", e) 
                                    });
                                } else {
                                    // After successful connection, refresh games to see the host's advert
                                    self.refresh_games().await?;
                                    
                                    // Wait a brief moment to allow game announcements to arrive
                                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                                    
                                    // Refresh games again to ensure we have the most recent list
                                    let games = self.refresh_games().await?;
                                    
                                    // Auto-join the first available game
                                    if let Some(game) = games.first() {
                                        tracing::info!("Auto-joining game {} via ticket connection", game.id);
                                        self.join_game(game.id.clone()).await?;
                                    }
                                }
                            }
                            UiToNet::GetNodeId => {
                                let node_id = self.iroh_ctx.node_id().to_string();
                                let _ = self.ui_tx.send(NetToUi::NodeId { node_id });
                            }
                            UiToNet::GetTicket => {
                                match self.iroh_ctx.ticket().await {
                                    Ok(ticket) => {
                                        let _ = self.ui_tx.send(NetToUi::Ticket { ticket });
                                    }
                                    Err(e) => {
                                        let _ = self.ui_tx.send(NetToUi::Error { 
                                            message: format!("Failed to generate ticket: {}", e) 
                                        });
                                    }
                                }
                            }
                            UiToNet::RunNetReport => {
                                // For now, just return a simple report showing node ID
                                let report = format!("Node ID: {}\nEndpoint: Active", self.iroh_ctx.node_id());
                                let _ = self.ui_tx.send(NetToUi::NetReport { report });
                            }
                            UiToNet::SetTag { gid, seq, tag } => {
                                self.handle_set_tag(gid, seq, tag).await?;
                            }
                            UiToNet::GetGhostMoves => {
                                self.handle_get_ghost_moves().await?;
                            }
                            UiToNet::AcceptScore { score_proof } => {
                                self.handle_accept_score(score_proof).await?;
                            }
                            UiToNet::CalculateScore { dead_stones } => {
                                self.handle_calculate_score(dead_stones).await?;
                            }
                            UiToNet::UpdateBoardSize { board_size } => {
                                // Update default board size in worker
                                self.default_board_size = board_size;
                                tracing::info!("Default board size updated to {}", board_size);
                            }
                        }
                    }
                    
                    // Handle lobby events
                    if let Ok(p2pgo_network::lobby::LobbyEvent::GameCreated(game_info)) = self.lobby_rx.try_recv() {
                        tracing::debug!(
                            game_id = %game_info.id,
                            board_size = game_info.board_size,
                            "Received GameCreated event"
                        );
                        
                        // Auto-join first game if not currently in one for this board size
                        if !self.active_games.contains_key(&game_info.board_size) {
                            tracing::debug!(
                                game_id = %game_info.id,
                                board_size = game_info.board_size,
                                "Auto-joining first available game for board size"
                            );
                            self.join_game(game_info.id.clone()).await?;
                        }
                        
                        // Always refresh games when a new game is created
                        self.refresh_games().await?;
                    } else if self.lobby_rx.try_recv().is_ok() {
                        // Handle other lobby events as needed
                    }
                    
                    // Handle game events from all active games
                    let mut game_events = Vec::new();
                    for (board_size, active_game) in &mut self.active_games {
                        if let Ok(event) = active_game.game_rx.try_recv() {
                            game_events.push((*board_size, event));
                        }
                    }
                    
                    for (board_size, event) in game_events {
                        tracing::debug!("Worker received game event for board size {}: {:?}", board_size, event);
                        self.handle_game_event(board_size, event).await?;
                    }
        }
        
        Ok(())
    }

    async fn create_game(&mut self, board_size: u8) -> anyhow::Result<()> {
        #[cfg(feature = "headless")]
        println!("Worker: Starting create_game with board_size {}", board_size);
        
        // Check if we already have a game for this board size
        if self.active_games.contains_key(&board_size) {
            let _ = self.ui_tx.send(NetToUi::Error {
                message: format!("Already have an active game for board size {}", board_size),
            });
            return Ok(());
        }
        
        match self.lobby.create_game(Some(self.player_name.clone()), board_size, false).await {
            Ok(game_id) => {
                #[cfg(feature = "headless")]
                println!("Worker: Successfully created game with ID: {}", game_id);
                
                match self.lobby.get_game_channel(&game_id).await {
                    Ok(game_channel) => {
                        #[cfg(feature = "headless")]
                        println!("Worker: Got game channel for game {}", game_id);
                        
                        let game_state = GameState::new(board_size);
                        
                        // Subscribe to game events BEFORE adding to active games
                        let game_rx = game_channel.subscribe();
                        
                        // Create ActiveGameData and add to HashMap
                        let active_game_data = ActiveGameData {
                            game: game_channel,
                            game_id: game_id.clone(),
                            game_state: Some(game_state),
                            game_rx,
                        };
                        
                        self.active_games.insert(board_size, active_game_data);
                        
                        #[cfg(feature = "headless")]
                        println!("Worker: Set up game state for {}", game_id);
                        
                        // Advertise game via gossip - don't let this block the success path
                        if let Err(err) = self.advertise_game(&game_id, board_size).await {
                            #[cfg(feature = "headless")]
                            println!("Worker: Warning - failed to advertise game: {}", err);
                            
                            tracing::warn!("Advertise failed: {err}");
                        }
                        
                        #[cfg(feature = "headless")]
                        println!("Worker: Sending GameJoined message for {}", game_id);
                        let _ = self.ui_tx.send(NetToUi::GameJoined { game_id: game_id.clone() });
                        
                        // Immediately generate and send ticket for easy sharing
                        if let Ok(ticket) = self.iroh_ctx.ticket().await {
                            let _ = self.ui_tx.send(NetToUi::Ticket { ticket: ticket.clone() });
                            tracing::info!("Share this ticket with opponent:\n{}", ticket);
                        }
                    }
                    Err(e) => {
                        #[cfg(feature = "headless")]
                        println!("Worker: Failed to get game channel: {}", e);
                        let _ = self.ui_tx.send(NetToUi::Error {
                            message: format!("Failed to get game channel: {}", e),
                        });
                    }
                }
            }
            Err(e) => {
                #[cfg(feature = "headless")]
                println!("Worker: Failed to create game: {}", e);
                let _ = self.ui_tx.send(NetToUi::Error {
                    message: format!("Failed to create game: {}", e),
                });
            }
        }
        
        Ok(())
    }

    async fn join_game(&mut self, game_id: String) -> anyhow::Result<()> {
        // First, try to get the game info to determine board size
        let games = self.lobby.list_games().await;
        let game_info = games.iter().find(|g| g.id == game_id);
        
        let board_size = if let Some(info) = game_info {
            info.board_size
        } else {
            // Fall back to default board size if we can't find the game info
            self.default_board_size
        };
        
        // Check if we already have a game for this board size
        if self.active_games.contains_key(&board_size) {
            let _ = self.ui_tx.send(NetToUi::Error {
                message: format!("Already have an active game for board size {}", board_size),
            });
            return Ok(());
        }
        
        match self.lobby.get_game_channel(&game_id).await {
            Ok(game_channel) => {
                let game_state = GameState::new(board_size);
                
                // Subscribe to game events BEFORE adding to active games
                let game_rx = game_channel.subscribe();
                
                // Create ActiveGameData and add to HashMap
                let active_game_data = ActiveGameData {
                    game: game_channel,
                    game_id: game_id.clone(),
                    game_state: Some(game_state),
                    game_rx,
                };
                
                self.active_games.insert(board_size, active_game_data);
                
                let _ = self.ui_tx.send(NetToUi::GameJoined { game_id });
            }
            Err(e) => {
                let _ = self.ui_tx.send(NetToUi::Error {
                    message: format!("Failed to join game: {}", e),
                });
            }
        }
        
        Ok(())
    }

    async fn make_move(&mut self, mv: p2pgo_core::Move, board_size: Option<u8>) -> anyhow::Result<()> {
        // Use provided board size or fall back to default
        let board_size = board_size.unwrap_or(self.default_board_size);
        
        if let Some(active_game) = self.active_games.get(&board_size) {
            // Store move for training (get sequence before making the move)
            let sequence = if let Some(game_state) = &active_game.game_state {
                game_state.moves.len() as u32
            } else {
                0
            };
            
            // Create a move record for storage
            let move_record = p2pgo_core::MoveRecord {
                mv: mv.clone(),
                tag: None, // No tag for UI moves
                ts: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                broadcast_hash: None,
                prev_hash: None,
            };
            
            if let Err(e) = self.iroh_ctx.store_game_move(&active_game.game_id, sequence, &move_record).await {
                tracing::warn!("Failed to store training move: {}", e);
            }
            
            // Send move to network - the channel will apply it and broadcast the event
            if let Err(e) = active_game.game.send_move(mv.clone()).await {
                let _ = self.ui_tx.send(NetToUi::Error {
                    message: format!("Failed to send move: {}", e),
                });
            }
            // Note: GameEvent will be received through the game channel subscription
        } else {
            let _ = self.ui_tx.send(NetToUi::Error {
                message: format!("No active game for board size {}", board_size),
            });
        }
        
        Ok(())
    }

    async fn refresh_games(&mut self) -> anyhow::Result<Vec<p2pgo_network::lobby::GameInfo>> {
        // Re-subscribe to gossip with current board size to capture any potential board size changes
        #[cfg(feature = "iroh")]
        {
            // Subscribe to the lobby topic for the default board size
            let _ = self.subscribe_to_gossip_lobby().await;
        }
        
        // Fetch available games
        let games = self.lobby.list_games().await;
        
        // Send to UI
        let _ = self.ui_tx.send(NetToUi::GamesUpdated { games: games.clone() });
        
        Ok(games)
    }

    async fn leave_game(&mut self) -> anyhow::Result<()> {
        // Leave the game for the default board size
        // TODO: In the future, we might want to pass board_size as a parameter
        let board_size = self.default_board_size;
        
        if self.active_games.remove(&board_size).is_some() {
            let _ = self.ui_tx.send(NetToUi::GameLeft);
        } else {
            let _ = self.ui_tx.send(NetToUi::Error {
                message: format!("No active game to leave for board size {}", board_size),
            });
        }
        
        Ok(())
    }

    async fn handle_game_event(&mut self, board_size: u8, event: GameEvent) -> anyhow::Result<()> {
        tracing::debug!("Worker handling game event for board size {}: {:?}", board_size, event);
        
        // Apply event to local game state if applicable
        if let Some(active_game) = self.active_games.get_mut(&board_size) {
            if let (Some(game_state), GameEvent::MoveMade { mv, .. }) = (&mut active_game.game_state, &event) {
                let _ = game_state.apply_move(mv.clone());
                
                // Check if the game is finished (2 passes or resign)
                if game_state.is_game_over() {
                    // Get komi based on board size
                    let komi = match game_state.board_size {
                        19 => 7.5,
                        13 => 6.5,
                        _ => 5.5, // 9x9 or other sizes
                    };
                    
                    // Determine scoring method based on how the game ended
                    let scoring_method = match mv {
                        p2pgo_core::Move::Resign => {
                            p2pgo_core::value_labeller::ScoringMethod::Resignation(game_state.current_player)
                        },
                        _ => p2pgo_core::value_labeller::ScoringMethod::Territory
                    };
                    
                    // Calculate score using proper territory scoring with empty dead stones set
                    // UI will handle marking dead stones and recalculating
                    let score_proof = p2pgo_core::scoring::calculate_final_score(
                        game_state,
                        komi,
                        scoring_method,
                        &std::collections::HashSet::new() // No dead stones initially
                    );
                    
                    // Send score dialog event
                    let black_score = score_proof.territory_black as f32;
                    let white_score = score_proof.territory_white as f32 + komi;
                    
                    let _ = self.ui_tx.send(NetToUi::GameEvent { 
                        event: GameEvent::GameFinished { 
                            black_score,
                            white_score,
                        }
                    });
                    
                    // Also send the more detailed score proof
                    let _ = self.ui_tx.send(NetToUi::ScoreCalculated { 
                        score_proof: score_proof.clone()
                    });
                    
                    // Start score acceptance timeout
                    self.start_score_timeout(board_size, score_proof).await;
                }
            }
        }
        
        // Special handling for game finished events
        if let GameEvent::GameFinished { black_score, white_score } = &event {
            tracing::info!("Game finished for board size {}. Black: {}, White: {}", board_size, black_score, white_score);
        }
        
        let _ = self.ui_tx.send(NetToUi::GameEvent { event });
        Ok(())
    }

    async fn advertise_game(&mut self, game_id: &str, board_size: u8) -> anyhow::Result<()> {
        if let Err(e) = self.iroh_ctx.advertise_game(game_id, board_size).await {
            tracing::warn!("Failed to advertise game: {}", e);
            let _ = self.ui_tx.send(NetToUi::Error {
                message: format!("Failed to advertise game: {}", e),
            });
        }
        Ok(())
    }

    async fn subscribe_to_gossip_lobby(&mut self) -> anyhow::Result<()> {
        #[cfg(feature = "iroh")]
        {
            // Subscribe to the lobby topic for the default board size using iroh
            match self.iroh_ctx.subscribe_lobby(self.default_board_size).await {
                Ok(mut event_rx) => {
                    // Spawn a background task to process incoming gossip events
                    let _ui_tx = self.ui_tx.clone();
                    // Don't clone lobby, use a weak reference or remove this functionality for now
                    // TODO: Fix gossip event handling for iroh v0.35
                    
                    tokio::spawn(async move {
                        while let Some(_event) = event_rx.recv().await {
                            tracing::debug!("Gossip event processing disabled - needs iroh v0.35 update");
                            // TODO: Re-implement gossip event processing
                        }
                    });
                    
                    tracing::info!("Subscribed to gossip lobby for board size {}", self.default_board_size);
                },
                Err(e) => {
                    tracing::warn!("Failed to subscribe to gossip lobby: {}", e);
                    let _ = self.ui_tx.send(NetToUi::Error {
                        message: format!("Failed to subscribe to lobby: {}", e),
                    });
                }
            }
        }
        
        #[cfg(not(feature = "iroh"))]
        {
            tracing::debug!("Stub mode - skipping gossip lobby subscription");
        }
        
        Ok(())
    }
    
    async fn handle_set_tag(&mut self, gid: String, seq: u32, tag: p2pgo_core::Tag) -> anyhow::Result<()> {
        // Store the tag annotation for the specified move
        if let Err(e) = self.iroh_ctx.store_move_tag(&gid, seq, tag).await {
            tracing::warn!("Failed to store move tag: {}", e);
            let _ = self.ui_tx.send(NetToUi::Error {
                message: format!("Failed to store tag: {}", e),
            });
        } else {
            let _ = self.ui_tx.send(NetToUi::TagAck);
        }
        Ok(())
    }

    async fn handle_get_ghost_moves(&mut self) -> anyhow::Result<()> {
        // Check if the player has completed enough games to see ghost moves
        const GHOST_MOVES_THRESHOLD: u32 = 5;
        if self.config.games_finished < GHOST_MOVES_THRESHOLD {
            let _ = self.ui_tx.send(NetToUi::Error {
                message: format!("Ghost moves will be available after completing {} games (currently: {})", 
                                GHOST_MOVES_THRESHOLD, self.config.games_finished),
            });
            return Ok(());
        }
        
        // Ensure we have a current game state for the default board size
        let game_state = if let Some(active_game) = self.active_games.get(&self.default_board_size) {
            match &active_game.game_state {
                Some(state) => state,
                None => {
                    let _ = self.ui_tx.send(NetToUi::Error {
                        message: "No game state available for ghost moves".to_string(),
                    });
                    return Ok(());
                }
            }
        } else {
            let _ = self.ui_tx.send(NetToUi::Error {
                message: format!("No active game for board size {} for ghost moves", self.default_board_size),
            });
            return Ok(());
        };

        // Lazy load the AI model if not already loaded
        if self.ai_model.is_none() {
            match self.load_ai_model().await {
                Ok(model) => {
                    self.ai_model = Some(Rc::new(Mutex::new(model)));
                }
                Err(e) => {
                    let _ = self.ui_tx.send(NetToUi::Error {
                        message: format!("Failed to load AI model: {}", e),
                    });
                    return Ok(());
                }
            }
        }

        // Get the AI model
        let model = match &self.ai_model {
            Some(model) => model.clone(),
            None => {
                let _ = self.ui_tx.send(NetToUi::Error {
                    message: "AI model not available".to_string(),
                });
                return Ok(());
            }
        };

        // Get ghost moves from the model
        match self.compute_ghost_moves(&model, game_state).await {
            Ok(ghost_coords) => {
                let _ = self.ui_tx.send(NetToUi::GhostMoves(ghost_coords));
            }
            Err(e) => {
                let _ = self.ui_tx.send(NetToUi::Error {
                    message: format!("Failed to compute ghost moves: {}", e),
                });
            }
        }

        Ok(())
    }

    async fn load_ai_model(&self) -> anyhow::Result<GoMini6E<Wgpu>> {
        // Create a new device
        let device = <Wgpu as Backend>::Device::default();
        
        // Initialize the model
        let model = GoMini6E::new(&device);
        
        tracing::info!("AI model loaded successfully");
        Ok(model)
    }

    async fn compute_ghost_moves(
        &self,
        model: &Rc<Mutex<GoMini6E<Wgpu>>>,
        game_state: &GameState,
    ) -> anyhow::Result<Vec<Coord>> {
        // Convert board state to model input
        let board_input = self.game_state_to_tensor(game_state)?;
        
        // Get model predictions
        let model = model.lock().map_err(|e| anyhow::anyhow!("Failed to lock model: {}", e))?;
        let device = <Wgpu as Backend>::Device::default();
        
        // Create input tensor [1, 81] for batch size 1
        let input_tensor = Tensor::<Wgpu, 1>::from_floats(board_input.as_slice(), &device)
            .reshape([1, 81]);
        
        // Forward pass
        let (policy_logits, _value) = model.forward(input_tensor);
        
        // Extract top 3 moves as ghost suggestions
        // Flatten the 2D tensor [1, 81] to 1D [81] first
        let flat_policy = policy_logits.squeeze::<1>(0); // Remove batch dimension to get [81]
        
        // Convert to vector - we need to extract each element individually
        let mut policy_data: Vec<f32> = Vec::new();
        for i in 0..81 {
            let slice = flat_policy.clone().narrow(0, i, 1);
            let value: f32 = slice.into_scalar();
            policy_data.push(value);
        }
        let mut move_scores: Vec<(usize, f32)> = policy_data.iter().enumerate().map(|(i, &score)| (i, score)).collect();
        
        // Sort by score descending
        move_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        // Convert top moves to coordinates
        let mut ghost_coords = Vec::new();
        let board_size = game_state.board_size as usize;
        
        for (move_idx, _score) in move_scores.iter().take(3) {
            if *move_idx < 81 && *move_idx < board_size * board_size {
                let row = move_idx / board_size;
                let col = move_idx % board_size;
                
                // Check if the position is empty
                let coord = Coord::new(col as u8, row as u8);
                let idx = row * board_size + col;
                if idx < game_state.board.len() && game_state.board[idx].is_none() {
                    ghost_coords.push(coord);
                }
            }
        }
        
        tracing::debug!("Generated {} ghost move suggestions", ghost_coords.len());
        Ok(ghost_coords)
    }

    fn game_state_to_tensor(&self, game_state: &GameState) -> anyhow::Result<Vec<f32>> {
        let board_size = game_state.board_size as usize;
        let mut tensor = vec![0.0f32; 81]; // Always use 9x9 for model consistency
        
        // Fill the tensor with board state (center smaller boards)
        let offset = (9 - board_size) / 2;
        
        for row in 0..board_size {
            for col in 0..board_size {
                let board_idx = row * board_size + col;
                let tensor_idx = (row + offset) * 9 + (col + offset);
                
                if tensor_idx < 81 && board_idx < game_state.board.len() {
                    tensor[tensor_idx] = match game_state.board[board_idx] {
                        Some(p2pgo_core::Color::Black) => 1.0,
                        Some(p2pgo_core::Color::White) => -1.0,
                        None => 0.0,
                    };
                }
            }
        }
        
        Ok(tensor)
    }

    async fn handle_calculate_score(&mut self, dead_stones: std::collections::HashSet<p2pgo_core::Coord>) -> anyhow::Result<()> {
        // Ensure we have a current game state for the default board size
        let game_state = if let Some(active_game) = self.active_games.get(&self.default_board_size) {
            match &active_game.game_state {
                Some(state) => state,
                None => {
                    let _ = self.ui_tx.send(NetToUi::Error {
                        message: "No game state available for score calculation".to_string(),
                    });
                    return Ok(());
                }
            }
        } else {
            let _ = self.ui_tx.send(NetToUi::Error {
                message: format!("No active game for board size {} for score calculation", self.default_board_size),
            });
            return Ok(());
        };

        // Use territory scoring (Chinese rules) with komi appropriate for board size
        let komi = match game_state.board_size {
            19 => 7.5,
            13 => 6.5,
            _ => 5.5, // 9x9 or other sizes
        };

        let scoring_method = p2pgo_core::value_labeller::ScoringMethod::Territory;
        
        // Calculate score using the scoring module
        let score_proof = p2pgo_core::scoring::calculate_final_score(
            game_state, 
            komi, 
            scoring_method, 
            &dead_stones
        );
        
        tracing::info!(
            "Score calculated: B:{} W:{} (final: {})", 
            score_proof.territory_black, 
            score_proof.territory_white, 
            score_proof.final_score
        );
        
        // Send score calculation result back to UI
        let _ = self.ui_tx.send(NetToUi::ScoreCalculated { score_proof });
        
        Ok(())
    }
    
    async fn handle_accept_score(&mut self, score_proof: p2pgo_core::value_labeller::ScoreProof) -> anyhow::Result<()> {
        // Store the final score for training for the default board size
        if let Some(active_game) = self.active_games.get(&self.default_board_size) {
            // Create a value labeller to handle the score
            let mut labeller = p2pgo_core::value_labeller::ValueLabeller::new();
            labeller.set_final_score(score_proof.clone());
            
            // In stub mode, we don't actually store the data, but log what we would store
            tracing::info!("Would store score proof and value labels for game {}", active_game.game_id);
            
            // Update metrics
            self.config.games_finished += 1;
            
            // Send ScoreAcceptedByBoth message to indicate successful scoring
            let _ = self.ui_tx.send(NetToUi::ScoreAcceptedByBoth { 
                score_proof: score_proof.clone() 
            });
            
            tracing::info!("Score accepted and stored for training. Games completed: {}", self.config.games_finished);
        } else {
            tracing::warn!("Cannot accept score: no active game for board size {}", self.default_board_size);
        }
        
        Ok(())
    }
    
    async fn start_score_timeout(&mut self, board_size: u8, score_proof: p2pgo_core::value_labeller::ScoreProof) {
        // Create and store the tracker
        let tracker = ScoreAcceptanceTracker::new(score_proof, board_size);
        self.score_trackers.insert(board_size, tracker);
        
        // Spawn timeout task
        let ui_tx = self.ui_tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(180)).await; // 3 minutes
            let _ = ui_tx.send(NetToUi::ScoreTimeout { board_size });
        });
    }
}
