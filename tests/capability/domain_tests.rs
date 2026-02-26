use imitatort_stateless_company::domain::capability::*;

#[tokio::test]
async fn test_capability_creation() {
    let schema = CapabilitySchema::builder()
        .input_param("param1", "string", "A test parameter")
        .output_type("string", "Test output")
        .build();

    let capability = Capability {
        id: "test-capability".to_string(),
        name: "Test Capability".to_string(),
        description: "A test capability".to_string(),
        schema,
        provider: CapabilityProvider::Framework,
        access_type: CapabilityAccessType::Public,
    };

    assert_eq!(capability.id, "test-capability");
    assert_eq!(capability.name, "Test Capability");
    assert_eq!(capability.description, "A test capability");
}

#[tokio::test]
async fn test_capability_path_operations() {
    let path1 = CapabilityPath::from("test.category.function");
    let path2 = CapabilityPath::from("test.category.function");
    let path3 = CapabilityPath::from("other.category.function");

    assert_eq!(path1, path2);
    assert_ne!(path1, path3);

    let parts: Vec<&str> = path1.as_str().split('.').collect();
    assert_eq!(parts, vec!["test", "category", "function"]);
}

#[tokio::test]
async fn test_capability_schema_builder() {
    let schema = CapabilitySchema::builder()
        .input_param("input1", "string", "First input")
        .input_param("input2", "integer", "Second input")
        .output_type("object", "Result object")
        .build();

    assert!(schema.input.contains_key("input1"));
    assert!(schema.input.contains_key("input2"));
    assert_eq!(schema.output.r#type, "object");
    assert_eq!(schema.output.description, "Result object");
}

#[tokio::test]
async fn test_capability_access_types() {
    let public_cap = Capability {
        id: "public".to_string(),
        name: "Public Capability".to_string(),
        description: "Public capability".to_string(),
        schema: CapabilitySchema::default(),
        provider: CapabilityProvider::Framework,
        access_type: CapabilityAccessType::Public,
    };

    let private_cap = Capability {
        id: "private".to_string(),
        name: "Private Capability".to_string(),
        description: "Private capability".to_string(),
        schema: CapabilitySchema::default(),
        provider: CapabilityProvider::Framework,
        access_type: CapabilityAccessType::Private,
    };

    assert_eq!(public_cap.access_type, CapabilityAccessType::Public);
    assert_eq!(private_cap.access_type, CapabilityAccessType::Private);
}

#[tokio::test]
async fn test_skill_capability_binding() {
    let binding = SkillCapabilityBinding {
        skill_id: "test-skill".to_string(),
        capability_id: "test-capability".to_string(),
        binding_type: BindingType::Required,
    };

    assert_eq!(binding.skill_id, "test-skill");
    assert_eq!(binding.capability_id, "test-capability");
    assert_eq!(binding.binding_type, BindingType::Required);
}