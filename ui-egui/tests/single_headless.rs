// SPDX-License-Identifier: MIT OR Apache-2.0

#[tokio::test]
async fn single_headless_test() -> anyhow::Result<()> {
    #[cfg(feature = "headless")]
    {
        use tokio::task;
        use tokio::time::{timeout, Duration};

        println!("Starting single headless test");

        let result = timeout(
            Duration::from_secs(5),
            task::spawn_blocking(|| {
                println!("Running headless function");
                p2pgo_ui_egui::headless()
            }),
        )
        .await;

        match result {
            Ok(Ok(Ok(()))) => {
                println!("Single headless instance completed successfully");
                Ok(())
            }
            Err(_) => {
                anyhow::bail!("Test timed out after 5 seconds")
            }
            Ok(Err(e)) => {
                anyhow::bail!("Join error: {:?}", e)
            }
            Ok(Ok(Err(e))) => {
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
