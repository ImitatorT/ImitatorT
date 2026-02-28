//! Capability 执行器接口
//!
//! 框架只定义执行器能力接口，具体实现由应用层提供

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

use crate::core::skill::SkillManager;
use crate::core::tool::ToolRegistry;
use crate::core::capability::CapabilityRegistry;
use crate::domain::capability::CapabilityCallContext;

#[async_trait]
pub trait CapabilityExecutor: Send + Sync {
    /// 执行功能调用
    ///
    /// # Arguments
    /// * `capability_id` - 功能ID
    /// * `params` - 功能参数（JSON对象）
    /// * `context` - 功能调用上下文
    ///
    /// # Returns
    /// 功能执行结果（JSON值）
    async fn execute(&self, capability_id: &str, params: Value, context: &CapabilityCallContext) -> Result<Value>;

    /// 检查是否支持某功能
    fn can_execute(&self, capability_id: &str) -> bool;

    /// 检查是否可以在特定技能上下文中执行功能
    fn can_execute_with_skills(&self, _capability_id: &str, _skills: &[String]) -> bool {
        true // 默认实现允许所有技能
    }

    /// 获取执行器支持的所有功能ID
    fn supported_capabilities(&self) -> Vec<String> {
        Vec::new()
    }
}

/// 功能调用结果
#[derive(Debug, Clone)]
pub struct CapabilityResult {
    /// 是否成功
    pub success: bool,
    /// 结果数据
    pub data: Value,
    /// 错误信息（如果失败）
    pub error: Option<String>,
}

impl CapabilityResult {
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

/// 功能执行器注册表
///
/// 管理多个执行器，根据功能ID路由到对应的执行器
pub struct CapabilityExecutorRegistry {
    executors: Vec<Box<dyn CapabilityExecutor>>,
    skill_manager: Arc<SkillManager>,
}

impl CapabilityExecutorRegistry {
    /// 创建注册表
    pub fn new(skill_manager: Arc<SkillManager>) -> Self {
        Self {
            executors: Vec::new(),
            skill_manager,
        }
    }

    /// 创建注册表（使用默认技能管理器）
    pub fn with_default_skill_manager(
        tool_registry: Arc<ToolRegistry>,
        capability_registry: Arc<CapabilityRegistry>,
    ) -> Self {
        let skill_manager = Arc::new(SkillManager::new_with_registries(tool_registry, capability_registry));
        Self::new(skill_manager)
    }

    /// 注册执行器
    pub fn register(&mut self, executor: Box<dyn CapabilityExecutor>) {
        self.executors.push(executor);
    }

    /// 查找可以执行指定功能的执行器
    pub fn find_executor(&self, capability_id: &str) -> Option<&dyn CapabilityExecutor> {
        self.executors
            .iter()
            .find(|e| e.can_execute(capability_id))
            .map(|e| e.as_ref())
    }

    /// 查找可以执行指定功能且具备相应技能的执行器
    fn find_executor_with_skills(&self, capability_id: &str, skills: &[String]) -> Option<&dyn CapabilityExecutor> {
        self.executors
            .iter()
            .find(|e| e.can_execute(capability_id) && e.can_execute_with_skills(capability_id, skills))
            .map(|e| e.as_ref())
    }

    /// 执行功能调用（自动路由到合适的执行器）
    pub async fn execute(&self, capability_id: &str, params: Value, context: &CapabilityCallContext) -> Result<CapabilityResult> {
        match self.find_executor(capability_id) {
            Some(executor) => {
                let result = executor.execute(capability_id, params, context).await?;
                Ok(CapabilityResult::success(result))
            }
            None => Ok(CapabilityResult::error(format!(
                "No executor found for capability: {}",
                capability_id
            ))),
        }
    }

    /// 执行功能调用（带技能验证）
    pub async fn execute_with_skills(
        &self,
        capability_id: &str,
        params: Value,
        context: &CapabilityCallContext,
        caller_skills: &[String],
    ) -> Result<CapabilityResult> {
        // 首先检查技能权限
        if !self.skill_manager.can_call_capability(capability_id, caller_skills) {
            return Ok(CapabilityResult::error(format!(
                "Insufficient skills to execute capability: {}",
                capability_id
            )));
        }

        // 查找可以执行的执行器
        match self.find_executor_with_skills(capability_id, caller_skills) {
            Some(executor) => {
                let result = executor.execute(capability_id, params, context).await?;
                Ok(CapabilityResult::success(result))
            }
            None => Ok(CapabilityResult::error(format!(
                "No executor found for capability: {}",
                capability_id
            ))),
        }
    }

    /// 检查是否有执行器支持该功能
    pub fn can_execute(&self, capability_id: &str) -> bool {
        self.find_executor(capability_id).is_some()
    }

    /// 检查是否有执行器支持该功能（带技能验证）
    pub fn can_execute_with_skills(&self, capability_id: &str, skills: &[String]) -> bool {
        self.find_executor_with_skills(capability_id, skills).is_some() &&
        self.skill_manager.can_call_capability(capability_id, skills)
    }

    /// 获取所有支持的功能ID
    pub fn list_supported_capabilities(&self) -> Vec<String> {
        self.executors
            .iter()
            .flat_map(|e| e.supported_capabilities())
            .collect()
    }
}

/// 函数式功能执行器包装
///
/// Capability executor function type alias
type CapabilityHandlerFn = dyn Fn(Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value>> + Send>> + Send + Sync;

/// 允许将普通异步函数包装为CapabilityExecutor
pub struct FnCapabilityExecutor {
    capability_id: String,
    handler: Box<CapabilityHandlerFn>,
}

impl FnCapabilityExecutor {
    /// 创建函数式执行器
    pub fn new<F, Fut>(capability_id: impl Into<String>, handler: F) -> Self
    where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Value>> + Send + 'static,
    {
        Self {
            capability_id: capability_id.into(),
            handler: Box::new(move |v| Box::pin(handler(v))),
        }
    }
}

#[async_trait]
impl CapabilityExecutor for FnCapabilityExecutor {
    async fn execute(&self, capability_id: &str, params: Value, _context: &CapabilityCallContext) -> Result<Value> {
        if capability_id != self.capability_id {
            return Err(anyhow::anyhow!("Capability ID mismatch: {} != {}", capability_id, self.capability_id));
        }
        (self.handler)(params).await
    }

    fn can_execute(&self, capability_id: &str) -> bool {
        capability_id == self.capability_id
    }

    fn supported_capabilities(&self) -> Vec<String> {
        vec![self.capability_id.clone()]
    }
}

/// 功能调用上下文
///
/// 包含功能调用时的上下文信息
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CapabilityContext {
    /// 调用者Agent ID
    pub caller_id: String,
    /// 调用时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// 额外的上下文数据
    pub metadata: std::collections::HashMap<String, String>,
}

impl CapabilityContext {
    /// 创建新上下文
    #[allow(dead_code)]
    pub fn new(caller_id: impl Into<String>) -> Self {
        Self {
            caller_id: caller_id.into(),
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// 添加元数据
    #[allow(dead_code)]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}