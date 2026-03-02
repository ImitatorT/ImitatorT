//! 通用测试辅助模块

use imitatort::core::messaging::MessageBus;
use imitatort::core::store::MemoryStore;
use imitatort::core::tool::ToolRegistry;
use imitatort::domain::Organization;
use imitatort::infrastructure::tool::{FrameworkToolExecutor, ToolEnvironment, ToolExecutor};
use imitatort::core::skill::SkillManager;
use std::sync::Arc;

/// 创建用于测试的 Mock Tool Executor
pub fn create_test_tool_executor() -> Arc<dyn ToolExecutor> {
    let message_bus = Arc::new(MessageBus::new());
    let org = Arc::new(tokio::sync::RwLock::new(Organization::new("test", "test")));
    let tool_registry = Arc::new(ToolRegistry::new());
    let store = Arc::new(MemoryStore::new());
    let skill_manager = Arc::new(SkillManager::new_with_tool_registry(tool_registry.clone()));

    let env = ToolEnvironment::new(message_bus, org, tool_registry, store, skill_manager);
    Arc::new(FrameworkToolExecutor::new(env))
}
