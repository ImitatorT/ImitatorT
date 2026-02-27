use std::sync::Arc;
use tokio::sync::RwLock;

use imitatort::core::capability::CapabilityRegistry;
use imitatort::domain::capability::*;

#[tokio::test]
async fn test_capability_registry_basic_operations() {
    let registry = CapabilityRegistry::new();

    let capability = Capability {
        id: "test-capability".to_string(),
        name: "Test Capability".to_string(),
        description: "A test capability".to_string(),
        schema: CapabilitySchema::default(),
        provider: CapabilityProvider::Framework,
        access_type: CapabilityAccessType::Public,
    };

    // Register capability
    let result = registry.register(capability.clone()).await;
    assert!(result.is_ok());

    // Get capability by ID
    let retrieved = registry.get_by_id("test-capability").await.unwrap();
    assert_eq!(retrieved.id, "test-capability");
    assert_eq!(retrieved.name, "Test Capability");
}

#[tokio::test]
async fn test_capability_registry_list_all() {
    let registry = CapabilityRegistry::new();

    let cap1 = Capability {
        id: "cap1".to_string(),
        name: "Capability 1".to_string(),
        description: "First capability".to_string(),
        schema: CapabilitySchema::default(),
        provider: CapabilityProvider::Framework,
        access_type: CapabilityAccessType::Public,
    };

    let cap2 = Capability {
        id: "cap2".to_string(),
        name: "Capability 2".to_string(),
        description: "Second capability".to_string(),
        schema: CapabilitySchema::default(),
        provider: CapabilityProvider::Framework,
        access_type: CapabilityAccessType::Public,
    };

    registry.register(cap1).await.unwrap();
    registry.register(cap2).await.unwrap();

    let all_caps = registry.list_all().await;
    assert_eq!(all_caps.len(), 2);
    assert!(all_caps.iter().any(|c| c.id == "cap1"));
    assert!(all_caps.iter().any(|c| c.id == "cap2"));
}

#[tokio::test]
async fn test_capability_registry_with_categories() {
    let registry = CapabilityRegistry::new();

    let cap1 = Capability {
        id: "test.math.add".to_string(),
        name: "Math Add".to_string(),
        description: "Addition capability".to_string(),
        schema: CapabilitySchema::default(),
        provider: CapabilityProvider::Framework,
        access_type: CapabilityAccessType::Public,
    };

    let cap2 = Capability {
        id: "test.math.subtract".to_string(),
        name: "Math Subtract".to_string(),
        description: "Subtraction capability".to_string(),
        schema: CapabilitySchema::default(),
        provider: CapabilityProvider::Framework,
        access_type: CapabilityAccessType::Public,
    };

    registry.register(cap1).await.unwrap();
    registry.register(cap2).await.unwrap();

    // Test category-based retrieval
    let math_caps = registry.get_by_category("test.math").await;
    assert_eq!(math_caps.len(), 2);

    let all_caps = registry.list_all().await;
    assert_eq!(all_caps.len(), 2);
}

#[tokio::test]
async fn test_capability_registry_concurrent_access() {
    let registry = Arc::new(CapabilityRegistry::new());

    let mut handles = vec![];

    // Spawn multiple tasks to register capabilities concurrently
    for i in 0..10 {
        let reg_clone = registry.clone();
        let handle = tokio::spawn(async move {
            let cap = Capability {
                id: format!("cap{}", i),
                name: format!("Capability {}", i),
                description: format!("Capability {}", i),
                schema: CapabilitySchema::default(),
                provider: CapabilityProvider::Framework,
                access_type: CapabilityAccessType::Public,
            };
            reg_clone.register(cap).await.unwrap();
        });
        handles.push(handle);
    }

    // Wait for all registrations to complete
    for handle in handles {
        handle.await.unwrap();
    }

    let all_caps = registry.list_all().await;
    assert_eq!(all_caps.len(), 10);
}

#[tokio::test]
async fn test_capability_registry_not_found() {
    let registry = CapabilityRegistry::new();

    let result = registry.get_by_id("nonexistent").await;
    assert!(result.is_none());
}