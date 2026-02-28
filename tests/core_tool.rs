//! Tool 注册表测试

use imitatort::core::tool::ToolRegistry;
use imitatort::domain::tool::{CategoryPath, Tool, JsonSchema, ReturnType};
use serde_json::json;

fn create_test_tool(id: &str, category: &str) -> Tool {
    Tool::new(
        id,
        format!("Tool {}", id),
        "Test description",
        CategoryPath::from_string(category),
        JsonSchema::object()
            .property("arg", JsonSchema::string().description("Test argument"))
            .build(),
    ).with_returns(ReturnType::new(
        "Test return",
        json!({"type": "string"}),
    ))
}

#[tokio::test]
async fn test_register_and_get() {
    let registry = ToolRegistry::new();
    let tool = create_test_tool("test.tool", "test/category");

    registry.register(tool.clone()).await.unwrap();

    assert!(registry.contains("test.tool"));
    let retrieved = registry.get("test.tool").unwrap();
    assert_eq!(retrieved.id, "test.tool");
}

#[tokio::test]
async fn test_find_by_category() {
    let registry = ToolRegistry::new();

    // 注册不同分类的工具
    registry.register(create_test_tool("file.read", "file/read")).await.unwrap();
    registry.register(create_test_tool("file.write", "file/write")).await.unwrap();
    registry.register(create_test_tool("file.delete", "file/delete")).await.unwrap();
    registry.register(create_test_tool("chat.send", "chat/send")).await.unwrap();

    // 查询 file 分类（包含子分类）
    let file_tools = registry.find_by_category("file").await;
    assert_eq!(file_tools.len(), 3);

    // 查询 file/read 分类
    let read_tools = registry.find_by_category("file/read").await;
    assert_eq!(read_tools.len(), 1);
    assert_eq!(read_tools[0].id, "file.read");
}

#[tokio::test]
async fn test_list_subcategories() {
    let registry = ToolRegistry::new();

    registry.register(create_test_tool("a.b", "a/b")).await.unwrap();
    registry.register(create_test_tool("a.c", "a/c")).await.unwrap();
    registry.register(create_test_tool("a.d.e", "a/d/e")).await.unwrap();

    let subcats = registry.list_subcategories("a").await;
    assert_eq!(subcats.len(), 3);
    assert!(subcats.contains(&"b".to_string()));
    assert!(subcats.contains(&"c".to_string()));
    assert!(subcats.contains(&"d".to_string()));
}

#[tokio::test]
async fn test_list_all_categories() {
    let registry = ToolRegistry::new();

    registry.register(create_test_tool("a.b", "a/b")).await.unwrap();
    registry.register(create_test_tool("a.c.d", "a/c/d")).await.unwrap();

    let categories = registry.list_all_categories().await;
    assert!(categories.contains(&"a".to_string()));
    assert!(categories.contains(&"a/b".to_string()));
    assert!(categories.contains(&"a/c".to_string()));
    assert!(categories.contains(&"a/c/d".to_string()));
}

#[tokio::test]
async fn test_unregister() {
    let registry = ToolRegistry::new();
    let tool = create_test_tool("temp.tool", "temp/category");

    registry.register(tool).await.unwrap();
    assert_eq!(registry.len(), 1);

    registry.unregister("temp.tool").await.unwrap();
    assert_eq!(registry.len(), 0);
    assert!(!registry.contains("temp.tool"));
}

#[tokio::test]
async fn test_duplicate_register() {
    let registry = ToolRegistry::new();
    let tool = create_test_tool("dup.tool", "test");

    registry.register(tool.clone()).await.unwrap();
    let result = registry.register(tool).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_deep_category() {
    let registry = ToolRegistry::new();

    registry
        .register(create_test_tool("deep.tool", "a/b/c/d/e"))
        .await
        .unwrap();

    // 各级查询都应该能找到
    assert_eq!(registry.find_by_category("a").await.len(), 1);
    assert_eq!(registry.find_by_category("a/b").await.len(), 1);
    assert_eq!(registry.find_by_category("a/b/c/d/e").await.len(), 1);

    // 不存在的路径
    assert_eq!(registry.find_by_category("a/b/x").await.len(), 0);
}
