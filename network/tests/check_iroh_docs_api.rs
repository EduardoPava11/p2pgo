// Test to verify how to use the iroh-docs API in v0.35

#[cfg(feature = "iroh")]
#[tokio::test]
async fn check_iroh_docs_api() {
    // Try to access iroh-docs API components to determine proper imports
    use iroh_docs::{self, store::fs::Store};

    use iroh::{Endpoint, NodeAddr};

    // Try to create a minimal docs client to see the API
    let store = Store::memory();
    let author = iroh_docs::store::Author::new();

    // Print out component info
    println!("-- iroh-docs v0.35 API info --");
    println!("Store type: {}", std::any::type_name::<Store>());
    println!(
        "Author type: {}",
        std::any::type_name::<iroh_docs::store::Author>()
    );

    // Try to create an endpoint
    let endpoint = Endpoint::builder()
        .bind()
        .await
        .expect("Failed to create endpoint");

    // Try to create a docs instance - iroh docs API looks like:
    // iroh_docs::DocDriver

    // This is expected to fail but should show the correct API path in the error
    println!("Testing docs instance creation...");
    let _docs = iroh_docs::DocDriver::builder()
        .author(author.clone())
        .network(endpoint)
        .store(store);
    println!("Docs API accessed successfully");

    assert!(true, "If this test compiles, we know the API structure");
}
