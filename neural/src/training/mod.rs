//! Training modules for neural networks

pub mod sgf_to_cbor;

pub use sgf_to_cbor::{SgfToCborConverter, batch_process_directory};