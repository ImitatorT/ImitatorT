//! Domain Layer
//!
//! Core business entity definitions

pub mod agent;
pub mod message;
pub mod org;
pub mod skill;
pub mod tool;
pub mod capability;
pub mod user;
pub mod invitation_code;

pub use agent::*;
pub use message::*;
pub use org::*;
pub use skill::*;

// Export TriggerCondition from agent module since it's used in AgentMode
pub use agent::TriggerCondition;

// Selective exports to avoid conflicts
pub use tool::{Tool, CategoryPath, ReturnType, ToolProvider, MatchType, CategoryNodeInfo, ToolCallContext, JsonSchema, ObjectSchemaBuilder, TypeBuilder};
pub use capability::{Capability, CapabilityPath, CapabilityCallContext, CapabilityProvider, CapabilityAccessType, SkillCapabilityBinding, BindingType};
