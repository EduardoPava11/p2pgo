//! Example of converting SGF files to CBOR format for neural network training

use p2pgo_neural::training::SgfToCborConverter;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    // Example 1: Convert a single SGF file
    let converter = SgfToCborConverter::new(9); // 9x9 board

    let sgf_path = Path::new("game.sgf");
    let output_path = Path::new("game.cbor");

    if sgf_path.exists() {
        converter.convert_file(sgf_path, output_path)?;
        println!(
            "Converted {} to {}",
            sgf_path.display(),
            output_path.display()
        );
    }

    // Example 2: Convert multiple SGF files to a batch
    let sgf_files = vec![
        Path::new("game1.sgf"),
        Path::new("game2.sgf"),
        Path::new("game3.sgf"),
    ];

    let batch_output = Path::new("batch.cbor");

    let existing_files: Vec<&Path> = sgf_files
        .iter()
        .filter(|p| p.exists())
        .map(|p| p.as_ref())
        .collect();

    if !existing_files.is_empty() {
        converter.convert_batch(&existing_files, batch_output)?;
        println!("Created batch with {} games", existing_files.len());
    }

    // Example 3: Custom converter with filtering
    let filtered_converter = SgfToCborConverter::new(9)
        .with_opening(false) // Skip opening positions
        .with_min_move(10); // Start from move 10

    let filtered_output = Path::new("filtered.cbor");

    if sgf_path.exists() {
        filtered_converter.convert_file(sgf_path, filtered_output)?;
        println!("Created filtered training data");
    }

    Ok(())
}

#[tokio::main]
async fn batch_convert_example() -> anyhow::Result<()> {
    use p2pgo_neural::training::batch_process_directory;

    // Process an entire directory of SGF files
    let input_dir = Path::new("sgf_games/");
    let output_dir = Path::new("cbor_batches/");
    let batch_size = 100; // 100 games per batch

    if input_dir.exists() {
        batch_process_directory(input_dir, output_dir, batch_size).await?;
        println!("Batch processing complete");
    }

    Ok(())
}
