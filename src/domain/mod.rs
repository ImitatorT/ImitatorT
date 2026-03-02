//! Domain Layer
//!
//! Core business entity definitions

pub mod agent;
pub mod capability;
pub mod invitation_code;
pub mod message;
pub mod org;
pub mod skill;
pub mod tool;
pub mod user;

pub use agent::*;
pub use message::*;
pub use org::*;
pub use skill::*;

// Export TriggerCondition from agent module since it's used in AgentMode
pub use agent::TriggerCondition;

// Selective exports to avoid conflicts
pub use capability::{
    BindingType, Capability, CapabilityAccessType, CapabilityCallContext, CapabilityPath,
    CapabilityProvider, SkillCapabilityBinding,
};
pub use tool::{
    CategoryNodeInfo, CategoryPath, JsonSchema, MatchType, ObjectSchemaBuilder, ReturnType, Tool,
    ToolCallContext, ToolProvider, TypeBuilder,
};
