use std::collections::HashMap;
use std::sync::Arc;

use imitatort_stateless_company::core::capability_provider::{CapabilityProvider, CompositeCapabilityProvider};
use imitatort_stateless_company::domain::capability::*;

// Mock capability provider for testing
struct MockCapabilityProvider {
    capabilities: Vec<Capability>,
}

impl MockCapabilityProvider {
    fn new(capabilities: Vec<Capability>) -> Self {
        Self { capabilities }
    }
}

#[async_trait::async_trait]
impl CapabilityProvider for MockCapabilityProvider {
    async fn list_capabilities(&self) -> Result<Vec<Capability>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.capabilities.clone())
    }

    async fn search_capabilities(&self, query: &str) -> Result<Vec<Capability>, Box<dyn std::error::Error + Send + Sync>> {
        let filtered: Vec<Capability> = self.capabilities
            .iter()
            .filter(|cap| cap.name.contains(query) || cap.description.contains(query))
            .cloned()
            .collect();
        Ok(filtered)
    }
}

#[tokio::test]
async fn test_mock_capability_provider() {
    let cap1 = Capability {
        id: "cap1".to_string(),
        name: "Test Capability 1".to_string(),
        description: "A test capability".to_string(),
        schema: CapabilitySchema::default(),
        provider: CapabilityProvider::Framework,
        access_type: CapabilityAccessType::Public,
    };

    let cap2 = Capability {
        id: "cap2".to_string(),
        name: "Another Capability".to_string(),
        description: "Another test capability".to_string(),
        schema: CapabilitySchema::default(),
        provider: CapabilityProvider::Framework,
        access_type: CapabilityAccessType::Public,
    };

    let provider = MockCapabilityProvider::new(vec![cap1.clone(), cap2.clone()]);

    let caps = provider.list_capabilities().await.unwrap();
    assert_eq!(caps.len(), 2);

    let search_results = provider.search_capabilities("Test").await.unwrap();
    assert_eq!(search_results.len(), 1);
    assert_eq!(search_results[0].id, "cap1");
}

#[tokio::test]
async fn test_composite_capability_provider() {
    let cap1 = Capability {
        id: "provider1.cap1".to_string(),
        name: "Provider 1 Capability".to_string(),
        description: "From provider 1".to_string(),
        schema: CapabilitySchema::default(),
        provider: CapabilityProvider::Framework,
        access_type: CapabilityAccessType::Public,
    };

    let cap2 = Capability {
        id: "provider2.cap1".to_string(),
        name: "Provider 2 Capability".to_string(),
        description: "From provider 2".to_string(),
        schema: CapabilitySchema::default(),
        provider: CapabilityProvider::Framework,
        access_type: CapabilityAccessType::Public,
    };

    let provider1 = MockCapabilityProvider::new(vec![cap1]);
    let provider2 = MockCapabilityProvider::new(vec![cap2]);

    let composite = CompositeCapabilityProvider::new(vec![
        Box::new(provider1),
        Box::new(provider2),
    ]);

    let all_caps = composite.list_capabilities().await.unwrap();
    assert_eq!(all_caps.len(), 2);

    let search_results = composite.search_capabilities("Provider").await.unwrap();
    assert_eq!(search_results.len(), 2);
}

#[tokio::test]
async fn test_composite_capability_provider_empty() {
    let composite = CompositeCapabilityProvider::new(vec![]);

    let caps = composite.list_capabilities().await.unwrap();
    assert_eq!(caps.len(), 0);

    let search_results = composite.search_capabilities("anything").await.unwrap();
    assert_eq!(search_results.len(), 0);
}

#[tokio::test]
async fn test_framework_capability_provider() {
    use imitatort_stateless_company::core::capability_provider::FrameworkCapabilityProvider;

    let provider = FrameworkCapabilityProvider::new();
    let caps = provider.list_capabilities().await.unwrap();

    // Framework provider should have at least one capability
    assert!(!caps.is_empty());

    // Search should work
    let search_results = provider.search_capabilities("").await.unwrap();
    assert_eq!(caps.len(), search_results.len());
}