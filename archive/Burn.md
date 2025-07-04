Below is the Copilot task file you asked for.
Paste the block into .github/TASK_burn_integration.md and let Copilot run it.
It wires the Burn deep-learning crate into your project so every player can turn the blobs they generate while playing Go into a local LLM (or policy-network) that they fully own—and lets them tweak board-size–specific models and hyper-parameters on their machine.

⸻


🛠 COPILOT PROMPT — “Burn-powered self-training bot pipeline”

Workspace : p2pgo   •   Toolchain : stable •  No unsafe

════════════════════════════════════════════════════════════════════
OVERVIEW  (why & what)
════════════════════════════════════════════════════════════════════
Burn 🦀 (https://github.com/tracel-ai/burn) is a Rust DL framework that
offers CPU + GPU back-ends via `wgpu` [oai_citation:0‡burn.dev](https://burn.dev/blog/cross-platform-gpu-backend/?utm_source=chatgpt.com) and already ships
examples for transformer text generation, WGAN, etc. [oai_citation:1‡burn.dev](https://burn.dev/burn-book/examples.html?utm_source=chatgpt.com)  
We’ll add a **burn-training** crate that:

1.  Ingests the CBOR “move-per-blob” files you already write (one doc per
    game, stored via `iroh-docs`).
2.  Yields a **Dataset** implementing `burn_dataset::Dataset` so users
    can filter by board size or self-label (rank, blitz, etc.) [oai_citation:2‡burn.dev](https://burn.dev/blog/building-blocks-dataset/?utm_source=chatgpt.com).
3.  Defines a **small transformer policy head** (`burn_transformers`
    fork – see early WIP repo [oai_citation:3‡github.com](https://github.com/bkonkle/burn-transformers?utm_source=chatgpt.com)) and a value head for win
    rate prediction (AlphaGo style).
4.  Exposes a **CLI command**  
    `p2pgo-cli train --size 9 --epochs 3 --layers 6 --lr 3e-4 --gpu`  
    that saves a `.burn` checkpoint per user.

Later the GUI will load that checkpoint to power a “Play vs Bot” mode.

════════════════════════════════════════════════════════════════════
STEP 0 · New crates & deps  (≤ 25 LOC in Cargo.toml)
--------------------------------------------------------------------
* Add **workspace member** `burn-training/`.
* Top-level `[workspace.dependencies]`
  ```toml
  burn          = { version = "0.11", features = ["train"] }  # latest
  burn-dataset  = "0.11"     # data utilities [oai_citation:4‡crates.io](https://crates.io/crates/burn-dataset?utm_source=chatgpt.com)
  burn-tch      = { version = "0.11", optional = true }       # GPU via libtorch
  burn-wgpu     = { version = "0.11", optional = true }       # GPU via wgpu
  serde_cbor    = "0.12"
  hex           = "0.4"
  clap          = { version = "4", features = ["derive"] }

	•	Feature flags propagate: gpu-wgpu, gpu-tch, cpu.

════════════════════════════════════════════════════════════════════
STEP 1 · Dataset loader  (≤ 120 LOC)

burn-training/src/dataset.rs

// SPDX-License-Identifier: MIT OR Apache-2.0
use burn_dataset::{Dataset, Sample};
use p2pgo_core::{GameState, Move, Color};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct GoSample {
    pub board: Vec<Vec<i8>>,    // -1 empty, 0 black, 1 white
    pub next_move: (u8, u8),    // for policy training
    pub winner:  Option<Color>, // for value head
}

impl Sample for GoSample {
    type Data = burn::tensor::Data<i8>;
    fn data(&self) -> Self::Data {
        burn::tensor::Data::from(self.board.clone())
    }
}

pub struct GoDataset { samples: Vec<GoSample> }
impl Dataset<GoSample> for GoDataset {
    fn get(&self, idx: usize) -> Option<GoSample> { self.samples.get(idx).cloned() }
    fn len(&self) -> usize { self.samples.len() }
}

impl GoDataset {
    /// Load from an iroh-docs blob directory exported to disk
    pub fn from_cbor_dir(dir: &std::path::Path, board: u8) -> anyhow::Result<Self> {
        let mut samples = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let bytes = std::fs::read(entry?.path())?;
            let gs: GameState = serde_cbor::from_slice(&bytes)?;
            if gs.board_size != board { continue; }
            // generate (state, next_move) pairs
            for w in gs.moves.windows(2) {
                if let [Move::Place(c0), Move::Place(c1)] = w {
                    let mut mat = vec![vec![-1i8; board as usize]; board as usize];
                    for (i, col) in gs.board.iter().enumerate() {
                        mat[i / board as usize][i % board as usize] =
                            match col { Some(Color::Black) => 0, Some(Color::White) => 1, _ => -1 };
                    }
                    samples.push(GoSample {
                        board: mat,
                        next_move: (c1.x, c1.y),
                        winner: if gs.is_game_over(){ gs.moves.last().and_then(|m| match m {
                            Move::Resign => Some(gs.current_player.opposite()),
                            _ => None }) } else { None }
                    });
                }
            }
        }
        Ok(Self { samples })
    }
}

(Uses Burn’s Dataset API so identical to the MNIST example ￼.)

════════════════════════════════════════════════════════════════════
STEP 2 · Tiny transformer policy network  (≤ 150 LOC)

burn-training/src/model.rs

use burn::{module::Module, nn, tensor, config::Config};

#[derive(Config)]
pub struct GoTransformerConfig {
    pub boardsize: usize,
    #[config(default = 6)] pub n_layers:  usize,
    #[config(default = 4)] pub n_heads:   usize,
    #[config(default = 128)] pub d_model: usize,
}

#[derive(Module)]
pub struct GoTransformer {
    transformer: nn::transformer::TransformerEncoder,
    head: nn::Linear,
}

impl GoTransformer {
    pub fn new(cfg: &GoTransformerConfig) -> Self {
        let tcfg = nn::transformer::TransformerEncoderConfig::new(
            cfg.d_model, cfg.n_heads, cfg.n_layers);
        Self {
            transformer: nn::transformer::TransformerEncoder::new(&tcfg),
            head: nn::Linear::new(cfg.d_model, (cfg.boardsize * cfg.boardsize) as _),
        }
    }
    pub fn forward(&self, x: tensor::Tensor<i8>) -> tensor::Tensor<f32> {
        let x = x.to_dtype::<f32>().reshape(&[-1, 1, (self.board_side* self.board_side) as _]);
        let h = self.transformer.forward(x);
        self.head.forward(h.flatten(1, -1))
    }
}

(Enough for a policy head; add a value head later.)

════════════════════════════════════════════════════════════════════
STEP 3 · Training CLI  (≤ 120 LOC)
	•	Extend cli/src/main.rs:

#[derive(Subcommand)]
enum Cmd { Train(TrainArgs) }

#[derive(Args)]
struct TrainArgs {
    #[arg(long, default_value="9")] board: u8,
    #[arg(long, default_value="3")] epochs: usize,
    #[arg(long, default_value="6")] layers: usize,
    #[arg(long, default_value="3e-4")] lr: f32,
    #[arg(long)] gpu: bool,
    #[arg(long)] data_dir: String,
}


	•	Inside match Cmd::Train:

let ds = GoDataset::from_cbor_dir(Path::new(&args.data_dir), args.board)?;
let cfg = GoTransformerConfig {
    boardsize: args.board as _, n_layers: args.layers, ..Default::default()
};
let device = if args.gpu { burn_wgpu::WgpuDevice::default() } else { burn::tensor::cpu::Device::default() };
let mut model = GoTransformer::new(&cfg).to_device(&device);
let optim = burn::optim::adam::AdamConfig::new().init(&model);
for epoch in 0..args.epochs {
    for batch in ds.iter().batch(128) { /* forward, loss, backward */ }
    println!("epoch {epoch} done");
}
model.save(format!("models/go_{}x{}_e{}.burn", args.board,args.board,args.epochs))?;



════════════════════════════════════════════════════════════════════
STEP 4 · GUI “Train Bot” button  (≤ 40 LOC)
	•	In Main Menu add [ Train Local Bot ].
	•	Opens file-picker to choose a data folder (exported blobs) and spawns
p2pgo-cli train … as a background process; show logs in an egui modal.

════════════════════════════════════════════════════════════════════
STEP 5 · Export blobs to disk  (≤ 40 LOC)
	•	Provide p2pgo-cli export --gid <id> --out ./data/
Uses network::debug::fetch_doc(gid) to dump CBOR per move.

════════════════════════════════════════════════════════════════════
FINISH LINE
════════════════════════════════════════════════════════════════════
	•	cargo check --workspace --features gpu-wgpu
	•	p2pgo-cli export --gid abc … then p2pgo-cli train --board 9 …
	•	Model file appears under models/.
	•	GUI “Play vs Bot” loads model (later).

Commit message suggestion:

feat(burn): self-training pipeline → export blobs → Burn dataset → transformer policy model → training CLI

Stop when code compiles; training loop can be a stub with TODO loss.

---

### Why Burn?

* Extreme performance: GPU via wgpu or cudnn/libtorch [oai_citation:6‡burn.dev](https://burn.dev/blog/cross-platform-gpu-backend/?utm_source=chatgpt.com).  
* Pure Rust → no Python tool-chain—fits your all-Rust stack.  
* Datasets & training loop APIs already demoed for MNIST and text-gen [oai_citation:7‡burn.dev](https://burn.dev/burn-book/examples.html?utm_source=chatgpt.com) [oai_citation:8‡burn.dev](https://burn.dev/blog/building-blocks-dataset/?utm_source=chatgpt.com).  
* Community extensions for transformers exist (burn-transformers WIP) [oai_citation:9‡github.com](https://github.com/bkonkle/burn-transformers?utm_source=chatgpt.com), so adapting to a 9×9 Go policy net is straightforward.

With this task file Copilot will wire Burn in, give every player a **local training CLI**, and set the stage for playing against personalized bots trained from their own P2P game data.