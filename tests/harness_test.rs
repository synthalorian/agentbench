use agentbench::harness::generic::GenericOpenAIHarness;
use agentbench::harness::{HarnessAdapter, HarnessAdapterConfig, HarnessRegistry};

#[tokio::test]
async fn test_generic_harness_init() {
    let mut harness = GenericOpenAIHarness::new();
    let config = HarnessAdapterConfig {
        name: "test-generic".to_string(),
        endpoint: Some("http://localhost:8080/v1".to_string()),
        api_key: None,
        model: Some("test-model".to_string()),
        extra: Default::default(),
    };

    let result = harness.init(config).await;
    assert!(result.is_ok());
    assert_eq!(harness.name(), "generic-openai");
}

#[test]
fn test_harness_registry() {
    let mut registry = HarnessRegistry::new();
    assert!(registry.list().is_empty());

    let harness = GenericOpenAIHarness::new();
    registry.register("generic".to_string(), Box::new(harness));

    let list = registry.list();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0], "generic");

    let retrieved = registry.get("generic");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name(), "generic-openai");

    assert!(registry.get("nonexistent").is_none());
}

#[tokio::test]
async fn test_harness_health_check_without_init() {
    let harness = GenericOpenAIHarness::new();
    let result = harness.health_check().await;
    // Returns Ok(false) when not initialized, not an error
    assert!(result.is_ok());
    assert!(!result.unwrap());
}
