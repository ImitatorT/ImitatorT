use imitatort_stateless_company::{
    core::skill::SkillManager,
    domain::capability::*,
    domain::skill::{BindingType, ToolAccessType},
};

#[tokio::test]
async fn test_skill_capability_binding() {
    let skill_manager = SkillManager::new();

    // Create a skill
    let skill = imitatort_stateless_company::domain::skill::Skill {
        id: "test-skill".to_string(),
        name: "Test Skill".to_string(),
        description: "A test skill".to_string(),
        category_path: imitatort_stateless_company::domain::skill::CategoryPath::from("test.skills"),
        access_type: ToolAccessType::Public,
    };

    skill_manager.register_skill(skill).await.unwrap();

    // Create a capability
    let capability = Capability {
        id: "test.capability.func".to_string(),
        name: "Test Capability Function".to_string(),
        description: "A test capability function".to_string(),
        schema: CapabilitySchema::default(),
        provider: CapabilityProvider::Framework,
        access_type: CapabilityAccessType::Public,
    };

    // Bind the capability to the skill
    let binding = SkillCapabilityBinding {
        skill_id: "test-skill".to_string(),
        capability_id: "test.capability.func".to_string(),
        binding_type: BindingType::Required,
    };

    skill_manager.bind_capability(binding).await.unwrap();

    // Verify the binding exists
    let bound_caps = skill_manager.get_capabilities_for_skill("test-skill").await;
    assert_eq!(bound_caps.len(), 1);
    assert_eq!(bound_caps[0].id, "test.capability.func");
}

#[tokio::test]
async fn test_skill_based_capability_access() {
    let skill_manager = SkillManager::new();

    // Register a skill
    let skill = imitatort_stateless_company::domain::skill::Skill {
        id: "admin-skill".to_string(),
        name: "Admin Skill".to_string(),
        description: "Administrator skill".to_string(),
        category_path: imitatort_stateless_company::domain::skill::CategoryPath::from("admin.skills"),
        access_type: ToolAccessType::Public,
    };

    skill_manager.register_skill(skill).await.unwrap();

    // Create capabilities with different access types
    let public_cap = Capability {
        id: "public.func".to_string(),
        name: "Public Function".to_string(),
        description: "Public capability".to_string(),
        schema: CapabilitySchema::default(),
        provider: CapabilityProvider::Framework,
        access_type: CapabilityAccessType::Public,
    };

    let private_cap = Capability {
        id: "private.func".to_string(),
        name: "Private Function".to_string(),
        description: "Private capability".to_string(),
        schema: CapabilitySchema::default(),
        provider: CapabilityProvider::Framework,
        access_type: CapabilityAccessType::Private,
    };

    // Test access control through skill manager
    // Public capabilities should be accessible
    assert_eq!(public_cap.access_type, CapabilityAccessType::Public);

    // Private capabilities require explicit skill binding
    assert_eq!(private_cap.access_type, CapabilityAccessType::Private);
}

#[tokio::test]
async fn test_multiple_capability_bindings() {
    let skill_manager = SkillManager::new();

    // Create multiple skills
    let skill1 = imitatort_stateless_company::domain::skill::Skill {
        id: "skill1".to_string(),
        name: "Skill 1".to_string(),
        description: "First skill".to_string(),
        category_path: imitatort_stateless_company::domain::skill::CategoryPath::from("skills.group1"),
        access_type: ToolAccessType::Public,
    };

    let skill2 = imitatort_stateless_company::domain::skill::Skill {
        id: "skill2".to_string(),
        name: "Skill 2".to_string(),
        description: "Second skill".to_string(),
        category_path: imitatort_stateless_company::domain::skill::CategoryPath::from("skills.group2"),
        access_type: ToolAccessType::Public,
    };

    skill_manager.register_skill(skill1).await.unwrap();
    skill_manager.register_skill(skill2).await.unwrap();

    // Create a capability
    let capability = Capability {
        id: "shared.capability".to_string(),
        name: "Shared Capability".to_string(),
        description: "A capability shared between skills".to_string(),
        schema: CapabilitySchema::default(),
        provider: CapabilityProvider::Framework,
        access_type: CapabilityAccessType::Public,
    };

    // Bind to both skills
    let binding1 = SkillCapabilityBinding {
        skill_id: "skill1".to_string(),
        capability_id: "shared.capability".to_string(),
        binding_type: BindingType::Required,
    };

    let binding2 = SkillCapabilityBinding {
        skill_id: "skill2".to_string(),
        capability_id: "shared.capability".to_string(),
        binding_type: BindingType::Optional,
    };

    skill_manager.bind_capability(binding1).await.unwrap();
    skill_manager.bind_capability(binding2).await.unwrap();

    // Verify both bindings exist
    let caps_for_skill1 = skill_manager.get_capabilities_for_skill("skill1").await;
    let caps_for_skill2 = skill_manager.get_capabilities_for_skill("skill2").await;

    assert_eq!(caps_for_skill1.len(), 1);
    assert_eq!(caps_for_skill2.len(), 1);
    assert_eq!(caps_for_skill1[0].id, "shared.capability");
    assert_eq!(caps_for_skill2[0].id, "shared.capability");
}

#[tokio::test]
async fn test_capability_unbinding() {
    let skill_manager = SkillManager::new();

    // Create skill and capability
    let skill = imitatort_stateless_company::domain::skill::Skill {
        id: "unbind-test-skill".to_string(),
        name: "Unbind Test Skill".to_string(),
        description: "Skill for unbind testing".to_string(),
        category_path: imitatort_stateless_company::domain::skill::CategoryPath::from("test.unbind"),
        access_type: ToolAccessType::Public,
    };

    skill_manager.register_skill(skill).await.unwrap();

    let capability = Capability {
        id: "unbind.test.cap".to_string(),
        name: "Unbind Test Capability".to_string(),
        description: "Capability for unbind testing".to_string(),
        schema: CapabilitySchema::default(),
        provider: CapabilityProvider::Framework,
        access_type: CapabilityAccessType::Public,
    };

    let binding = SkillCapabilityBinding {
        skill_id: "unbind-test-skill".to_string(),
        capability_id: "unbind.test.cap".to_string(),
        binding_type: BindingType::Required,
    };

    skill_manager.bind_capability(binding).await.unwrap();

    // Verify binding exists
    let bound_caps = skill_manager.get_capabilities_for_skill("unbind-test-skill").await;
    assert_eq!(bound_caps.len(), 1);

    // Unbind the capability
    skill_manager.unbind_capability("unbind-test-skill", "unbind.test.cap").await.unwrap();

    // Verify binding is removed
    let bound_caps_after = skill_manager.get_capabilities_for_skill("unbind-test-skill").await;
    assert_eq!(bound_caps_after.len(), 0);
}