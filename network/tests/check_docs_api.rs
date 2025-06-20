//! Test to check iroh-docs API

#[cfg(feature = "iroh")]
#[tokio::test] 
async fn check_docs_api() {
    use iroh_docs::{self, store::Store};
    
    // Print some API info to help with debugging
    println!("iroh-docs version available types:");
    println!("Store: {}", std::any::type_name::<Store>());
    
    // Try to import some common types to see what's available
    let store_types = vec![
        "Store", "MemStore", "BlobStore"
    ];
    
    println!("Available store types: {:?}", store_types);
    
    // This test doesn't need to do anything - it's just for debugging API
    assert!(true);
}
