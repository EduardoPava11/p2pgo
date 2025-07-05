// SPDX-License-Identifier: MIT OR Apache-2.0
use tokio::{
    task,
    time::{sleep, Duration},
};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn gui_two_player_round_trip() -> anyhow::Result<()> {
    #[cfg(feature = "headless")]
    {
        println!("Starting two-player GUI end-to-end test");

        // launch node A
        let a = task::spawn_blocking(|| {
            println!("Starting player A");
            p2pgo_ui_egui::headless()
        });

        // slight stagger so lobby topic exists
        sleep(Duration::from_millis(200)).await;

        // launch node B
        let b = task::spawn_blocking(|| {
            println!("Starting player B");
            p2pgo_ui_egui::headless()
        });

        // wait for both to complete with timeout
        let timeout_duration = Duration::from_secs(15);
        println!(
            "Waiting for both players to complete within {} seconds",
            timeout_duration.as_secs()
        );

        let result_a = tokio::time::timeout(timeout_duration, a).await;
        let result_b = tokio::time::timeout(timeout_duration, b).await;

        // Handle the results properly
        match (result_a, result_b) {
            (Ok(Ok(Ok(()))), Ok(Ok(Ok(())))) => {
                // Both completed successfully
                println!("Both headless instances completed successfully");

                // For now, just verify that both processes completed without panicking
                // The actual state verification can be enhanced once P2P sync is fully implemented
                Ok(())
            }
            (Err(_), _) | (_, Err(_)) => {
                anyhow::bail!("Test timed out after 15 seconds")
            }
            (Ok(Err(e)), _) | (_, Ok(Err(e))) => {
                anyhow::bail!("Join error: {:?}", e)
            }
            (Ok(Ok(Err(e))), _) | (_, Ok(Ok(Err(e)))) => {
                anyhow::bail!("Headless function error: {}", e)
            }
        }
    }

    #[cfg(not(feature = "headless"))]
    {
        println!("Skipping test - headless feature not enabled");
        Ok(())
    }
}
