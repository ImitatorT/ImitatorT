//! Tool 执行器接口
//!
//! 框架只定义执行器能力接口，具体实现由应用层提供

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

use crate::core::skill::SkillManager;
use crate::core::tool::ToolRegistry;
use crate::domain::tool::ToolCallContext;

pub mod framework_tools;

pub use framework_tools::{FrameworkToolExecutor, ToolEnvironment};


/// 工具执行器接口 - 由具体实现者提供
#[async_trait]
pub trait ToolExecutor: Send + Sync {
    /// 执行工具调用
    ///
    /// # Arguments
    /// * `tool_id` - 工具ID
    /// * `params` - 工具参数（JSON对象）
    /// * `context` - 工具调用上下文
    ///
    /// # Returns
    /// 工具执行结果（JSON值）
    async fn execute(&self, tool_id: &str, params: Value, context: &ToolCallContext) -> Result<Value>;

    /// 检查是否支持某工具
    fn can_execute(&self, tool_id: &str) -> bool;

    /// 检查是否可以在特定技能上下文中执行工具
    fn can_execute_with_skills(&self, _tool_id: &str, _skills: &[String]) -> bool {
        true // 默认实现允许所有技能
    }

    /// 获取执行器支持的所有工具ID
    fn supported_tools(&self) -> Vec<String> {
        Vec::new()
    }
}

/// 工具调用结果
#[derive(Debug, Clone)]
pub struct ToolResult {
    /// 是否成功
    pub success: bool,
    /// 结果数据
    pub data: Value,
    /// 错误信息（如果失败）
    pub error: Option<String>,
}

impl ToolResult {
    /// 创建成功结果
    pub fn success(data: Value) -> Self {
        Self {
            success: true,
            data,
            error: None,
        }
    }

    /// 创建失败结果
    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            data: Value::Null,
            error: Some(msg.into()),
        }
    }
}

/// 工具执行器注册表
///
/// 管理多个执行器，根据工具ID路由到对应的执行器
pub struct ToolExecutorRegistry {
    executors: Vec<Box<dyn ToolExecutor>>,
    skill_manager: Arc<SkillManager>,
}

impl ToolExecutorRegistry {
    /// 创建注册表
    pub fn new(skill_manager: Arc<SkillManager>) -> Self {
        Self {
            executors: Vec::new(),
            skill_manager,
        }
    }

    /// 创建注册表（使用默认技能管理器）
    pub fn with_default_skill_manager(tool_registry: Arc<ToolRegistry>) -> Self {
        let skill_manager = Arc::new(SkillManager::new_with_tool_registry(tool_registry));
        Self::new(skill_manager)
    }

    /// 注册执行器
    pub fn register(&mut self, executor: Box<dyn ToolExecutor>) {
        self.executors.push(executor);
    }

    /// 查找可以执行指定工具的执行器
    pub fn find_executor(&self, tool_id: &str) -> Option<&dyn ToolExecutor> {
        self.executors
            .iter()
            .find(|e| e.can_execute(tool_id))
            .map(|e| e.as_ref())
    }

    /// 查找可以执行指定工具且具备相应技能的执行器
    fn find_executor_with_skills(&self, tool_id: &str, skills: &[String]) -> Option<&dyn ToolExecutor> {
        self.executors
            .iter()
            .find(|e| e.can_execute(tool_id) && e.can_execute_with_skills(tool_id, skills))
            .map(|e| e.as_ref())
    }

    /// 执行工具调用（自动路由到合适的执行器）
    pub async fn execute(&self, tool_id: &str, params: Value, context: &ToolCallContext) -> Result<ToolResult> {
        match self.find_executor(tool_id) {
            Some(executor) => {
                let result = executor.execute(tool_id, params, context).await?;
                Ok(ToolResult::success(result))
            }
            None => Ok(ToolResult::error(format!(
                "No executor found for tool: {}",
                tool_id
            ))),
        }
    }

    /// 执行工具调用（带技能验证）
    pub async fn execute_with_skills(
        &self,
        tool_id: &str,
        params: Value,
        context: &ToolCallContext,
        caller_skills: &[String],
    ) -> Result<ToolResult> {
        // 首先检查技能权限
        if !self.skill_manager.can_call_tool(tool_id, caller_skills) {
            return Ok(ToolResult::error(format!(
                "Insufficient skills to execute tool: {}",
                tool_id
            )));
        }

        // 查找可以执行的执行器
        match self.find_executor_with_skills(tool_id, caller_skills) {
            Some(executor) => {
                let result = executor.execute(tool_id, params, context).await?;
                Ok(ToolResult::success(result))
            }
            None => Ok(ToolResult::error(format!(
                "No executor found for tool: {}",
                tool_id
            ))),
        }
    }

    /// 检查是否有执行器支持该工具
    pub fn can_execute(&self, tool_id: &str) -> bool {
        self.find_executor(tool_id).is_some()
    }

    /// 检查是否有执行器支持该工具（带技能验证）
    pub fn can_execute_with_skills(&self, tool_id: &str, skills: &[String]) -> bool {
        self.find_executor_with_skills(tool_id, skills).is_some() &&
        self.skill_manager.can_call_tool(tool_id, skills)
    }

    /// 获取所有支持的工具ID
    pub fn list_supported_tools(&self) -> Vec<String> {
        self.executors
            .iter()
            .flat_map(|e| e.supported_tools())
            .collect()
    }
}

// 注意：由于ToolExecutorRegistry现在需要SkillManager参数，不能提供默认实现
// 用户需要显式创建实例

/// 函数式工具执行器包装
///
/// 允许将普通异步函数包装为ToolExecutor
pub struct FnToolExecutor {
    tool_id: String,
    handler: Box<dyn Fn(Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value>> + Send>> + Send + Sync>,
}

impl FnToolExecutor {
    /// 创建函数式执行器
    pub fn new<F, Fut>(tool_id: impl Into<String>, handler: F) -> Self
    where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Value>> + Send + 'static,
    {
        Self {
            tool_id: tool_id.into(),
            handler: Box::new(move |v| Box::pin(handler(v))),
        }
    }
}

#[async_trait]
impl ToolExecutor for FnToolExecutor {
    async fn execute(&self, tool_id: &str, params: Value, _context: &ToolCallContext) -> Result<Value> {
        if tool_id != self.tool_id {
            return Err(anyhow::anyhow!("Tool ID mismatch: {} != {}", tool_id, self.tool_id));
        }
        (self.handler)(params).await
    }

    fn can_execute(&self, tool_id: &str) -> bool {
        tool_id == self.tool_id
    }

    fn supported_tools(&self) -> Vec<String> {
        vec![self.tool_id.clone()]
    }
}

/// 工具调用上下文
///
/// 包含工具调用时的上下文信息
#[derive(Debug, Clone)]
pub struct ToolContext {
    /// 调用者Agent ID
    pub caller_id: String,
    /// 调用时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// 额外的上下文数据
    pub metadata: std::collections::HashMap<String, String>,
}

impl ToolContext {
    /// 创建新上下文
    pub fn new(caller_id: impl Into<String>) -> Self {
        Self {
            caller_id: caller_id.into(),
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// 添加元数据
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}