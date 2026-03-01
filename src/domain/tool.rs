//! Tool Domain Entity
//!
//! Core business definition for tool system, supporting multi-level classification
//! Parameters use JSON Schema format, directly compatible with OpenAI Tool Calling

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Tool Entity - Single source of truth
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: CategoryPath,
    /// Parameter JSON Schema definition
    pub parameters: Value,
    pub returns: ReturnType,
}

impl Tool {
    /// Create new tool
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        category: CategoryPath,
        parameters: Value,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            category,
            parameters,
            returns: ReturnType::default(),
        }
    }

    /// Set return type
    pub fn with_returns(mut self, returns: ReturnType) -> Self {
        self.returns = returns;
        self
    }

    /// Get required parameter field list
    pub fn required_params(&self) -> Vec<String> {
        self.parameters
            .get("required")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 获取参数属性定义
    pub fn param_properties(&self) -> Option<&Value> {
        self.parameters.get("properties")
    }
}

/// 分类路径 - 支持多级如 ["file", "read"]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CategoryPath(Vec<String>);

impl CategoryPath {
    /// 创建新的分类路径
    pub fn new(path: Vec<String>) -> Self {
        Self(path)
    }

    /// 从字符串解析，如 "file/read" -> ["file", "read"]
    pub fn from_string(path: &str) -> Self {
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
    pub fn is_child_of(&self, other: &CategoryPath) -> bool {
        if self.0.len() <= other.0.len() {
            return false;
        }
        self.0.starts_with(&other.0)
    }

    /// 检查是否包含某路径（相同或是其父路径）
    pub fn contains(&self, other: &CategoryPath) -> bool {
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

/// JSON Schema 参数构建器
///
/// 用于方便地构建 OpenAI 兼容的 JSON Schema 参数定义
pub struct JsonSchema;

impl JsonSchema {
    /// 创建 object 类型的参数根
    ///
    /// # Example
    /// ```ignore
    /// use imitatort::domain::tool::JsonSchema;
    ///
    /// let params = JsonSchema::object()
    ///     .property("name", JsonSchema::string().description("用户名"))
    ///     .property("age", JsonSchema::integer().description("年龄").optional())
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
        Self::array(json!({"type": "string"}))
    }

    /// 创建 enum 类型
    pub fn enum_values(values: Vec<&str>) -> TypeBuilder {
        let mut builder = TypeBuilder::new("string");
        builder.schema["enum"] = json!(values);
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

        let properties = self.schema["properties"]
            .as_object_mut()
            .expect("Expected 'properties' to be an object in schema");
        properties.insert(name.to_string(), builder.build());

        // 如果属性是必填的，添加到 required 数组
        if is_required {
            let required = self.schema["required"]
                .as_array_mut()
                .expect("Expected 'required' to be an array in schema");
            required.push(json!(name));
        }

        self
    }

    /// 直接添加原始 JSON Schema 属性
    pub fn raw_property(mut self, name: &str, schema: Value, is_required: bool) -> Self {
        let properties = self.schema["properties"]
            .as_object_mut()
            .expect("Expected 'properties' to be an object in schema");
        properties.insert(name.to_string(), schema);

        if is_required {
            let required = self.schema["required"]
                .as_array_mut()
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

/// 返回值类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnType {
    pub description: String,
    /// JSON Schema 格式返回值定义
    pub return_schema: Value,
}

impl ReturnType {
    /// 创建新返回值类型
    pub fn new(description: impl Into<String>, return_schema: Value) -> Self {
        Self {
            description: description.into(),
            return_schema,
        }
    }
}

impl Default for ReturnType {
    fn default() -> Self {
        Self {
            description: "无返回值".to_string(),
            return_schema: json!({"type": "null"}),
        }
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

/// Tool 提供者接口
///
/// 框架工具和应用工具都实现此接口，通过 Composite 模式统一查询
pub trait ToolProvider: Send + Sync {
    /// 获取所有工具定义
    fn list_tools(&self) -> Vec<Tool>;

    /// 搜索工具（支持名称、描述、分类）
    fn search_tools(&self, query: &str, match_type: MatchType) -> Vec<Tool>;

    /// 按分类获取工具
    fn list_tools_by_category(&self, category: &str) -> Vec<Tool>;

    /// 获取分类树（JSON 序列化友好）
    fn get_category_tree(&self) -> CategoryNodeInfo;
}

/// 分类节点信息（用于展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryNodeInfo {
    pub name: String,
    pub path: String,
    pub tool_count: usize,
    pub children: Vec<CategoryNodeInfo>,
}

impl CategoryNodeInfo {
    pub fn new(name: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            tool_count: 0,
            children: Vec::new(),
        }
    }
}

/// Tool 调用上下文
///
/// 包含单次工具调用时的上下文信息
#[derive(Debug, Clone)]
pub struct ToolCallContext {
    /// 调用者 Agent ID
    pub caller_id: String,
    /// 调用时间戳
    pub timestamp: i64,
    /// 会话ID（如果有）
    pub session_id: Option<String>,
}

impl ToolCallContext {
    /// 创建新的调用上下文
    pub fn new(caller_id: impl Into<String>) -> Self {
        Self {
            caller_id: caller_id.into(),
            timestamp: chrono::Utc::now().timestamp(),
            session_id: None,
        }
    }

    /// 设置会话ID
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }
}
