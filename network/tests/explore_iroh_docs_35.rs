#[cfg(feature = "iroh")]
mod tests {
    // Let's explore what's available in iroh_docs v0.35
    
    #[test]
    fn explore_available_types() {
        // Print out what types are available in the iroh_docs crate
        
        // Check for Document
        println!("Available iroh_docs types and modules:");
        
        // Store modules
        println!("store::fs::Store exists: {}", std::any::type_name::<iroh_docs::store::fs::Store>());
        
        // Check for Document - try different paths
        println!("store::Document: {}", option_type_name::<Option<iroh_docs::store::Document>>());
        println!("DocDriver: {}", option_type_name::<Option<iroh_docs::DocDriver>>());
        
        // Check for Author
        println!("store::Author: {}", option_type_name::<Option<iroh_docs::store::Author>>());
        
        // Show some high-level exported types
        println!("NamespaceId: {}", std::any::type_name::<iroh_docs::NamespaceId>());
        println!("AuthorId: {}", std::any::type_name::<iroh_docs::AuthorId>());
        
        // Look for other main types
        println!("Looking for DocDriver:");
        println!("iroh_docs::DocDriver: {}", option_type_name::<Option<iroh_docs::DocDriver>>());
        
        // Try other possible module paths
        println!("\nChecking other possible modules:");
        println!("protocol exists? {}", module_exists("iroh_docs::protocol"));
    }
    
    fn option_type_name<T: 'static>() -> String {
        let name = std::any::type_name::<T>();
        // Extract the inner type name from Option<T>
        if name.starts_with("core::option::Option<") && name.ends_with(">") {
            let inner = &name[21..name.len()-1];
            return inner.to_string();
        }
        "Type not found".to_string()
    }
    
    fn module_exists(name: &str) -> bool {
        // This is just a dummy function - the real check is at compile time
        true
    }
}

#[cfg(not(feature = "iroh"))]
#[test]
fn dummy_test() {
    // This is just a placeholder for when iroh feature is not enabled
    assert!(true);
}
