//! Capability Domain Entity
//!
//! Core business definition for capability system, supporting multi-level classification, compatible with MCP (Model Context Protocol)
//! Parameters use JSON Schema format, directly compatible with MCP Capability Discovery protocol

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Capability Entity - Single source of truth
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub id: String,
    pub name: String,
    pub description: String,
    pub capability_path: CapabilityPath,
    /// Parameter JSON Schema definition (MCP compatible format)
    pub input_schema: Value,
    pub output_schema: Value,
    pub protocol: String, // MCP protocol type: "http", "stdio", "websocket", "sse", etc.
    pub endpoint: Option<String>, // Optional endpoint URL
}

impl Capability {
    /// Create new capability
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        capability_path: CapabilityPath,
        input_schema: Value,
        output_schema: Value,
        protocol: impl Into<String>,
        endpoint: Option<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            capability_path,
            input_schema,
            output_schema,
            protocol: protocol.into(),
            endpoint,
        }
    }

    /// Get required parameter field list
    pub fn required_inputs(&self) -> Vec<String> {
        self.input_schema
            .get("required")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 获取输入参数属性定义
    pub fn input_properties(&self) -> Option<&Value> {
        self.input_schema.get("properties")
    }

    /// 获取输出参数属性定义
    pub fn output_properties(&self) -> Option<&Value> {
        self.output_schema.get("properties")
    }
}

/// 功能路径 - 支持多级如 ["file", "read"]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CapabilityPath(Vec<String>);

impl CapabilityPath {
    /// 创建新的功能路径
    pub fn new(path: Vec<String>) -> Self {
        Self(path)
    }

    /// 从字符串解析，如 "file/read" -> ["file", "read"]
    pub fn from_str(path: &str) -> Self {
        let parts: Vec<String> = path
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        Self(parts)
    }

    /// 转换为字符串，如 ["file", "read"] -> "file/read"
    pub fn to_path_string(&self) -> String {
        self.0.join("/")
    }

    /// 获取路径片段
    pub fn segments(&self) -> &[String] {
        &self.0
    }

    /// 获取父路径
    pub fn parent(&self) -> Option<Self> {
        if self.0.len() <= 1 {
            None
        } else {
            Some(Self(self.0[..self.0.len() - 1].to_vec()))
        }
    }

    /// 获取路径深度
    pub fn depth(&self) -> usize {
        self.0.len()
    }

    /// 检查是否为另一路径的子路径
    pub fn is_child_of(&self, other: &CapabilityPath) -> bool {
        if self.0.len() <= other.0.len() {
            return false;
        }
        self.0.starts_with(&other.0)
    }

    /// 检查是否包含某路径（相同或是其父路径）
    pub fn contains(&self, other: &CapabilityPath) -> bool {
        if self.0.len() > other.0.len() {
            return false;
        }
        other.0.starts_with(&self.0)
    }

    /// 获取最后一级分类名
    pub fn name(&self) -> Option<&str> {
        self.0.last().map(|s| s.as_str())
    }
}


/// JSON Schema 输入参数构建器
///
/// 用于方便地构建 MCP 兼容的 JSON Schema 参数定义
pub struct InputSchema;

impl InputSchema {
    /// 创建 object 类型的参数根
    ///
    /// # Example
    /// ```
    /// let params = InputSchema::object()
    ///     .property("name", InputSchema::string().description("用户名"))
    ///     .property("age", InputSchema::integer().description("年龄").optional())
    ///     .build();
    /// ```
    pub fn object() -> ObjectSchemaBuilder {
        ObjectSchemaBuilder::new()
    }

    /// 创建 string 类型
    pub fn string() -> TypeBuilder {
        TypeBuilder::new("string")
    }

    /// 创建 integer 类型
    pub fn integer() -> TypeBuilder {
        TypeBuilder::new("integer")
    }

    /// 创建 number 类型
    pub fn number() -> TypeBuilder {
        TypeBuilder::new("number")
    }

    /// 创建 boolean 类型
    pub fn boolean() -> TypeBuilder {
        TypeBuilder::new("boolean")
    }

    /// 创建 array 类型
    pub fn array(item_schema: Value) -> TypeBuilder {
        let mut builder = TypeBuilder::new("array");
        builder.schema["items"] = item_schema;
        builder
    }

    /// 创建 string 数组类型（常用）
    pub fn string_array() -> TypeBuilder {
        Self::array(serde_json::json!({"type": "string"}))
    }

    /// 创建 enum 类型
    pub fn enum_values(values: Vec<&str>) -> TypeBuilder {
        let mut builder = TypeBuilder::new("string");
        builder.schema["enum"] = serde_json::json!(values);
        builder
    }
}

/// JSON Schema 输出参数构建器
pub struct OutputSchema;

impl OutputSchema {
    /// 创建输出 schema
    pub fn object() -> ObjectSchemaBuilder {
        ObjectSchemaBuilder::new()
    }

    /// 创建 string 类型
    pub fn string() -> TypeBuilder {
        TypeBuilder::new("string")
    }

    /// 创建 integer 类型
    pub fn integer() -> TypeBuilder {
        TypeBuilder::new("integer")
    }

    /// 创建 number 类型
    pub fn number() -> TypeBuilder {
        TypeBuilder::new("number")
    }

    /// 创建 boolean 类型
    pub fn boolean() -> TypeBuilder {
        TypeBuilder::new("boolean")
    }

    /// 创建 array 类型
    pub fn array(item_schema: Value) -> TypeBuilder {
        let mut builder = TypeBuilder::new("array");
        builder.schema["items"] = item_schema;
        builder
    }
}

use serde_json::json;

/// Object 类型构建器
pub struct ObjectSchemaBuilder {
    schema: Value,
}

impl ObjectSchemaBuilder {
    fn new() -> Self {
        Self {
            schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }

    /// 添加属性
    pub fn property(mut self, name: &str, builder: TypeBuilder) -> Self {
        // 先检查是否必填，再移动 builder
        let is_required = builder.required;

        let properties = self.schema["properties"].as_object_mut()
            .expect("Expected 'properties' to be an object in schema");
        properties.insert(name.to_string(), builder.build());

        // 如果属性是必填的，添加到 required 数组
        if is_required {
            let required = self.schema["required"].as_array_mut()
                .expect("Expected 'required' to be an array in schema");
            required.push(json!(name));
        }

        self
    }

    /// 直接添加原始 JSON Schema 属性
    pub fn raw_property(mut self, name: &str, schema: Value, is_required: bool) -> Self {
        let properties = self.schema["properties"].as_object_mut()
            .expect("Expected 'properties' to be an object in schema");
        properties.insert(name.to_string(), schema);

        if is_required {
            let required = self.schema["required"].as_array_mut()
                .expect("Expected 'required' to be an array in schema");
            required.push(json!(name));
        }

        self
    }

    /// 构建最终的 JSON Schema
    pub fn build(self) -> Value {
        self.schema
    }
}

/// 类型构建器
pub struct TypeBuilder {
    schema: Value,
    required: bool,
}

impl TypeBuilder {
    fn new(type_name: &str) -> Self {
        Self {
            schema: json!({"type": type_name}),
            required: true,
        }
    }

    /// 设置描述
    pub fn description(mut self, desc: &str) -> Self {
        self.schema["description"] = json!(desc);
        self
    }

    /// 设置为可选（用于构建时判断）
    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    /// 添加 enum 约束
    pub fn enum_values(mut self, values: Vec<&str>) -> Self {
        self.schema["enum"] = json!(values);
        self
    }

    /// 构建最终的 JSON Schema
    pub fn build(self) -> Value {
        self.schema
    }
}

/// 匹配类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MatchType {
    /// 精确匹配
    Exact,
    /// 模糊匹配
    #[default]
    Fuzzy,
}

impl Default for MatchType {
    fn default() -> Self {
        MatchType::Fuzzy
    }
}

/// Capability 提供者接口
///
/// 框架功能和应用功能都实现此接口，通过 Composite 模式统一查询
pub trait CapabilityProvider: Send + Sync {
    /// 获取所有功能定义
    fn list_capabilities(&self) -> Vec<Capability>;

    /// 搜索功能（支持名称、描述、分类）
    fn search_capabilities(&self, query: &str, match_type: MatchType) -> Vec<Capability>;

    /// 按分类获取功能
    fn list_capabilities_by_path(&self, path: &str) -> Vec<Capability>;

    /// 获取分类树（JSON 序列化友好）
    fn get_capability_tree(&self) -> CapabilityNodeInfo;
}

/// 分类节点信息（用于展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityNodeInfo {
    pub name: String,
    pub path: String,
    pub capability_count: usize,
    pub children: Vec<CapabilityNodeInfo>,
}

impl CapabilityNodeInfo {
    pub fn new(name: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            capability_count: 0,
            children: Vec::new(),
        }
    }
}

/// 功能访问类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CapabilityAccessType {
    /// 公共功能：任何人都可以调用
    Public,
    /// 私有功能：需要特定技能才能调用
    Private,
}

/// Capability 调用上下文
///
/// 包含单次功能调用时的上下文信息
#[derive(Debug, Clone)]
pub struct CapabilityCallContext {
    /// 调用者 Agent ID
    pub caller_id: String,
    /// 调用时间戳
    pub timestamp: i64,
    /// 会话ID（如果有）
    pub session_id: Option<String>,
    /// 调用参数
    pub parameters: Value,
}

impl CapabilityCallContext {
    /// 创建新的调用上下文
    pub fn new(caller_id: impl Into<String>, parameters: Value) -> Self {
        Self {
            caller_id: caller_id.into(),
            timestamp: chrono::Utc::now().timestamp(),
            session_id: None,
            parameters,
        }
    }

    /// 设置会话ID
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }
}

/// 技能与功能的关系
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCapabilityBinding {
    pub skill_id: String,
    pub capability_id: String,
    pub binding_type: BindingType,
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl SkillCapabilityBinding {
    /// 创建新的绑定
    pub fn new(
        skill_id: impl Into<String>,
        capability_id: impl Into<String>,
        binding_type: BindingType,
    ) -> Self {
        Self {
            skill_id: skill_id.into(),
            capability_id: capability_id.into(),
            binding_type,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// 设置绑定元数据
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// 绑定类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BindingType {
    /// 强绑定：技能必须有此功能才能正常工作
    Required,
    /// 可选绑定：技能可以利用此功能增强功能
    Optional,
}