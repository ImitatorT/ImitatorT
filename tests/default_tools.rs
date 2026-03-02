//! 默认工具测试
//!
//! 测试文件操作、命令执行、网页 Fetch 工具

use imitatort::core::messaging::MessageBus;
use imitatort::core::tool::ToolRegistry;
use imitatort::core::tool_provider::FrameworkToolProvider;
use imitatort::domain::tool::ToolCallContext;
use imitatort::domain::tool::ToolProvider;
use imitatort::infrastructure::tool::{FrameworkToolExecutor, ToolEnvironment};
use imitatort::{Organization, SkillManager, SqliteStore};
use serde_json::json;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::RwLock;

/// 创建测试用的 ToolEnvironment
async fn create_test_environment() -> (ToolEnvironment, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let message_bus = Arc::new(MessageBus::new());
    let organization = Arc::new(RwLock::new(Organization::new()));
    let tool_registry = Arc::new(ToolRegistry::new());
    let message_store = Arc::new(
        SqliteStore::new_in_memory().expect("Failed to create SQLite store"),
    );
    let skill_manager = Arc::new(SkillManager::new_with_tool_registry(tool_registry.clone()));

    let env = ToolEnvironment::new(
        message_bus,
        organization,
        tool_registry,
        message_store,
        skill_manager,
    );

    (env, temp_dir)
}

// ==================== 文件操作测试 ====================

#[tokio::test]
async fn test_file_read_write() {
    let (env, temp_dir) = create_test_environment().await;
    let executor = FrameworkToolExecutor::new(env);
    let context = ToolCallContext::new("test-agent");

    let test_file = temp_dir.path().join("test.txt");
    let test_path = test_file.to_string_lossy();
    let test_content = "Hello, ImitatorT!";

    // 测试写入文件
    let write_result = executor
        .execute(
            "file.write",
            json!({
                "path": test_path,
                "content": test_content,
            }),
            &context,
        )
        .await
        .expect("Execute failed");

    assert!(write_result.success);
    assert!(write_result.error.is_none());

    // 测试读取文件
    let read_result = executor
        .execute(
            "file.read",
            json!({
                "path": test_path,
            }),
            &context,
        )
        .await
        .expect("Execute failed");

    assert!(read_result.success);
    let content = read_result.data["content"].as_str().unwrap();
    assert_eq!(content, test_content);
}

#[tokio::test]
async fn test_file_append() {
    let (env, temp_dir) = create_test_environment().await;
    let executor = FrameworkToolExecutor::new(env);
    let context = ToolCallContext::new("test-agent");

    let test_file = temp_dir.path().join("append_test.txt");
    let test_path = test_file.to_string_lossy();

    // 第一次写入
    executor
        .execute(
            "file.write",
            json!({
                "path": test_path,
                "content": "Line 1\n",
            }),
            &context,
        )
        .await
        .unwrap();

    // 追加内容
    let append_result = executor
        .execute(
            "file.write",
            json!({
                "path": test_path,
                "content": "Line 2\n",
                "append": true,
            }),
            &context,
        )
        .await
        .unwrap();

    assert!(append_result.success);

    // 读取验证
    let read_result = executor
        .execute(
            "file.read",
            json!({
                "path": test_path,
            }),
            &context,
        )
        .await
        .unwrap();

    let content = read_result.data["content"].as_str().unwrap();
    assert!(content.contains("Line 1"));
    assert!(content.contains("Line 2"));
}

#[tokio::test]
async fn test_file_delete() {
    let (env, temp_dir) = create_test_environment().await;
    let executor = FrameworkToolExecutor::new(env);
    let context = ToolCallContext::new("test-agent");

    let test_file = temp_dir.path().join("delete_test.txt");
    let test_path = test_file.to_string_lossy();

    // 先创建文件
    executor
        .execute(
            "file.write",
            json!({
                "path": test_path,
                "content": "To be deleted",
            }),
            &context,
        )
        .await
        .unwrap();

    // 测试删除
    let delete_result = executor
        .execute(
            "file.delete",
            json!({
                "path": test_path,
            }),
            &context,
        )
        .await
        .unwrap();

    assert!(delete_result.success);

    // 验证文件已删除
    assert!(!std::path::Path::new(&test_path.as_ref()).exists());
}

#[tokio::test]
async fn test_file_list() {
    let (env, temp_dir) = create_test_environment().await;
    let executor = FrameworkToolExecutor::new(env);
    let context = ToolCallContext::new("test-agent");

    let test_dir = temp_dir.path().join("test_subdir");
    tokio::fs::create_dir(&test_dir).await.unwrap();

    // 创建几个测试文件
    tokio::fs::write(test_dir.join("file1.rs"), "content1")
        .await
        .unwrap();
    tokio::fs::write(test_dir.join("file2.rs"), "content2")
        .await
        .unwrap();
    tokio::fs::write(test_dir.join("file3.txt"), "content3")
        .await
        .unwrap();

    let test_path = test_dir.to_string_lossy();

    // 测试列出目录
    let list_result = executor
        .execute(
            "file.list",
            json!({
                "path": test_path,
            }),
            &context,
        )
        .await
        .unwrap();

    assert!(list_result.success);
    let entries = list_result.data["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 3);

    // 测试通配符过滤 *.rs
    let filter_result = executor
        .execute(
            "file.list",
            json!({
                "path": test_path,
                "pattern": "*.rs",
            }),
            &context,
        )
        .await
        .unwrap();

    assert!(filter_result.success);
    let filtered_entries = filter_result.data["entries"].as_array().unwrap();
    assert_eq!(filtered_entries.len(), 2);
}

#[tokio::test]
async fn test_file_read_not_found() {
    let (env, _temp_dir) = create_test_environment().await;
    let executor = FrameworkToolExecutor::new(env);
    let context = ToolCallContext::new("test-agent");

    let read_result = executor
        .execute(
            "file.read",
            json!({
                "path": "/nonexistent/file/path.txt",
            }),
            &context,
        )
        .await
        .unwrap();

    // 应该返回 success: false 并包含错误信息
    let success = read_result.data["success"].as_bool().unwrap();
    assert!(!success);
    assert!(read_result.data["error"].as_str().is_some());
}

// ==================== 命令执行测试 ====================

#[tokio::test]
async fn test_shell_exec_echo() {
    let (env, _temp_dir) = create_test_environment().await;
    let executor = FrameworkToolExecutor::new(env);
    let context = ToolCallContext::new("test-agent");

    let result = executor
        .execute(
            "shell.exec",
            json!({
                "command": "echo 'Hello, ImitatorT!'",
            }),
            &context,
        )
        .await
        .unwrap();

    assert!(result.success);
    let stdout = result.data["stdout"].as_str().unwrap();
    assert!(stdout.contains("Hello, ImitatorT!"));
    assert_eq!(result.data["exit_code"].as_i64().unwrap(), 0);
}

#[tokio::test]
async fn test_shell_exec_pwd() {
    let (env, _temp_dir) = create_test_environment().await;
    let executor = FrameworkToolExecutor::new(env);
    let context = ToolCallContext::new("test-agent");

    let result = executor
        .execute(
            "shell.exec",
            json!({
                "command": "pwd",
            }),
            &context,
        )
        .await
        .unwrap();

    assert!(result.success);
    let stdout = result.data["stdout"].as_str().unwrap();
    assert!(!stdout.is_empty());
}

#[tokio::test]
async fn test_shell_exec_with_timeout() {
    let (env, _temp_dir) = create_test_environment().await;
    let executor = FrameworkToolExecutor::new(env);
    let context = ToolCallContext::new("test-agent");

    // 快速完成的命令
    let result = executor
        .execute(
            "shell.exec",
            json!({
                "command": "echo 'fast'",
                "timeout": 5,
            }),
            &context,
        )
        .await
        .unwrap();

    assert!(result.success);
    assert_eq!(result.data["exit_code"].as_i64().unwrap(), 0);
}

#[cfg(not(target_os = "windows"))]
#[tokio::test]
async fn test_shell_exec_stderr() {
    let (env, _temp_dir) = create_test_environment().await;
    let executor = FrameworkToolExecutor::new(env);
    let context = ToolCallContext::new("test-agent");

    // 执行一个会产生 stderr 的命令
    let result = executor
        .execute(
            "shell.exec",
            json!({
                "command": "cat /nonexistent_file_12345",
            }),
            &context,
        )
        .await
        .unwrap();

    // 命令应该失败
    let success = result.data["success"].as_bool().unwrap();
    assert!(!success);
    let stderr = result.data["stderr"].as_str().unwrap();
    assert!(!stderr.is_empty());
}

// ==================== 网页 Fetch 测试 ====================

#[tokio::test]
async fn test_http_fetch_basic() {
    let (env, _temp_dir) = create_test_environment().await;
    let executor = FrameworkToolExecutor::new(env);
    let context = ToolCallContext::new("test-agent");

    // 使用一个简单的测试 URL（httpbin 或类似服务）
    let result = executor
        .execute(
            "http.fetch",
            json!({
                "url": "https://httpbin.org/html",
                "timeout": 30,
            }),
            &context,
        )
        .await
        .unwrap();

    // 注意：这个测试可能因为网络问题失败
    let success = result.data["success"].as_bool().unwrap();
    if success {
        let status = result.data["status"].as_u64().unwrap();
        assert_eq!(status, 200);
        let body = result.data["body"].as_str().unwrap();
        assert!(!body.is_empty());
    }
    // 如果网络不可用，测试可能失败，但不 assert false
}

#[tokio::test]
async fn test_http_fetch_headers() {
    let (env, _temp_dir) = create_test_environment().await;
    let executor = FrameworkToolExecutor::new(env);
    let context = ToolCallContext::new("test-agent");

    // 使用 httpbin 检查请求头
    let result = executor
        .execute(
            "http.fetch",
            json!({
                "url": "https://httpbin.org/headers",
            }),
            &context,
        )
        .await
        .unwrap();

    let success = result.data["success"].as_bool().unwrap();
    if success {
        let body = result.data["body"].as_str().unwrap();
        // 检查响应中是否包含 Chrome User-Agent
        assert!(body.contains("Chrome") || body.contains("Mozilla"));
    }
}

#[tokio::test]
async fn test_http_fetch_invalid_url() {
    let (env, _temp_dir) = create_test_environment().await;
    let executor = FrameworkToolExecutor::new(env);
    let context = ToolCallContext::new("test-agent");

    let result = executor
        .execute(
            "http.fetch",
            json!({
                "url": "not-a-valid-url",
            }),
            &context,
        )
        .await
        .unwrap();

    let success = result.data["success"].as_bool().unwrap();
    assert!(!success);
    assert!(result.data["error"].as_str().is_some());
}

#[tokio::test]
async fn test_http_fetch_missing_url() {
    let (env, _temp_dir) = create_test_environment().await;
    let executor = FrameworkToolExecutor::new(env);
    let context = ToolCallContext::new("test-agent");

    let result = executor
        .execute(
            "http.fetch",
            json!({}),
            &context,
        )
        .await
        .expect("Execute should not fail");

    // 应该返回错误
    let success = result.data["success"].as_bool();
    assert!(success.is_some() && !success.unwrap(), "Should return success: false when url is missing");
    assert!(result.data["error"].as_str().is_some());
}

// ==================== 工具定义测试 ====================

#[test]
fn test_framework_tools_definition() {
    use imitatort::core::tool_provider::FrameworkToolProvider;
    use imitatort::domain::tool::ToolProvider;

    let provider = FrameworkToolProvider::new();
    let tools = provider.list_tools();

    // 检查新工具是否已注册
    let tool_ids: Vec<&str> = tools.iter().map(|t| t.id.as_str()).collect();

    // 文件操作工具
    assert!(tool_ids.contains(&"file.read"));
    assert!(tool_ids.contains(&"file.write"));
    assert!(tool_ids.contains(&"file.delete"));
    assert!(tool_ids.contains(&"file.list"));

    // 命令执行工具
    assert!(tool_ids.contains(&"shell.exec"));

    // 网页请求工具
    assert!(tool_ids.contains(&"http.fetch"));
}

#[test]
fn test_tool_categories() {
    use imitatort::core::tool_provider::FrameworkToolProvider;
    use imitatort::domain::tool::ToolProvider;

    let provider = FrameworkToolProvider::new();

    // 检查文件操作分类 - 使用完整的分类路径
    let file_read_tools = provider.list_tools_by_category("file/read");
    assert!(!file_read_tools.is_empty(), "file/read category should not be empty");

    // 检查 shell 分类
    let shell_exec_tools = provider.list_tools_by_category("shell/execute");
    assert!(!shell_exec_tools.is_empty(), "shell/execute category should not be empty");

    // 检查 http 分类
    let http_fetch_tools = provider.list_tools_by_category("http/fetch");
    assert!(!http_fetch_tools.is_empty(), "http/fetch category should not be empty");
}
