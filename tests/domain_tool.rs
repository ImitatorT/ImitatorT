//! Tool 领域实体测试

use imitatort_stateless_company::domain::tool::{CategoryPath, Tool, JsonSchema, ReturnType};
use serde_json::json;

#[test]
fn test_category_path_from_str() {
    let path = CategoryPath::from_str("file/read");
    assert_eq!(path.segments(), vec!["file", "read"]);
    assert_eq!(path.to_path_string(), "file/read");
}

#[test]
fn test_category_path_parent() {
    let path = CategoryPath::from_str("file/read/text");
    assert_eq!(path.parent(), Some(CategoryPath::from_str("file/read")));
    assert_eq!(
        path.parent().unwrap().parent(),
        Some(CategoryPath::from_str("file"))
    );
    assert_eq!(CategoryPath::from_str("file").parent(), None);
}

#[test]
fn test_category_path_is_child_of() {
    let child = CategoryPath::from_str("file/read/text");
    let parent = CategoryPath::from_str("file/read");
    let grandparent = CategoryPath::from_str("file");

    assert!(child.is_child_of(&parent));
    assert!(child.is_child_of(&grandparent));
    assert!(parent.is_child_of(&grandparent));
    assert!(!parent.is_child_of(&child));
}

#[test]
fn test_category_path_contains() {
    let parent = CategoryPath::from_str("file/read");
    let child = CategoryPath::from_str("file/read/text");

    assert!(parent.contains(&child));
    assert!(!child.contains(&parent));
    assert!(parent.contains(&parent)); // 包含自身
}

#[test]
fn test_tool_creation() {
    let tool = Tool::new(
        "file.read",
        "读取文件",
        "读取指定路径的文件内容",
        CategoryPath::from_str("file/read"),
        JsonSchema::object()
            .property("path", JsonSchema::string().description("文件路径"))
            .build(),
    ).with_returns(ReturnType::new(
        "文件内容",
        json!({"type": "string"}),
    ));

    assert_eq!(tool.id, "file.read");
    assert_eq!(tool.name, "读取文件");
    assert_eq!(tool.category.to_path_string(), "file/read");
}

#[test]
fn test_json_schema_builder() {
    let params = JsonSchema::object()
        .property("name", JsonSchema::string().description("用户名"))
        .property("age", JsonSchema::integer().description("年龄").optional())
        .build();

    assert!(params["properties"]["name"].is_object());
    assert!(params["properties"]["age"].is_object());
    assert_eq!(params["required"][0], "name"); // name 是必需的
}
