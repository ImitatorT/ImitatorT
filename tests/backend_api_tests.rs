//! ImitatorT Backend API Integration Tests

use axum::Router;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::Duration;

use imitatort::domain::{Agent, LLMConfig, Message, MessageTarget, Organization, Role};
use imitatort::infrastructure::web::{create_router, AppState};
use imitatort::infrastructure::auth::JwtService;

// 创建测试用的app状态
fn create_test_app_state() -> Arc<AppState> {
    let mut org = Organization::new();

    // 添加测试agent
    let agent1 = Agent::new(
        "test-agent-1",
        "Test Employee 1",
        Role::simple("Developer", "You are a developer"),
        LLMConfig::openai("test-key"),
    );
    let agent2 = Agent::new(
        "test-agent-2",
        "Test Employee 2",
        Role::simple("Manager", "You are a manager"),
        LLMConfig::openai("test-key"),
    );

    org.add_agent(agent1);
    org.add_agent(agent2);

    let agents = org.agents.clone();

    // 创建消息通道
    let (message_tx, _) = broadcast::channel::<Message>(100);

    // 创建存储
    let store = Arc::new(imitatort::infrastructure::store::SqliteStore::new_in_memory().unwrap());

    // 创建JWT服务
    let jwt_service = JwtService::new("test-secret-for-testing");

    Arc::new(AppState {
        agents,
        message_tx,
        store,
        jwt_service,
    })
}

#[tokio::test]
async fn test_health_check_endpoint() {
    let state = create_test_app_state();
    let app = create_router(state);

    let client = reqwest::Client::new();

    // 启动测试服务器
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // 等待服务器启动
    tokio::time::sleep(Duration::from_millis(100)).await;

    // 发起请求
    let response = client
        .get(format!("http://{}/api/health", addr))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let json_value: serde_json::Value = response.json().await.unwrap();
    assert_eq!(json_value["status"], "ok");
}

#[tokio::test]
async fn test_list_agents_endpoint() {
    let state = create_test_app_state();
    let app = create_router(state);

    let client = reqwest::Client::new();

    // 启动测试服务器
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // 等待服务器启动
    tokio::time::sleep(Duration::from_millis(100)).await;

    // 发起请求
    let response = client
        .get(format!("http://{}/api/agents", addr))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let agents: Vec<serde_json::Value> = response.json().await.unwrap();
    assert!(!agents.is_empty());
    assert!(agents.len() >= 2); // 至少有两个测试agent

    // 验证agent结构
    for agent in &agents {
        assert!(agent.get("id").is_some());
        assert!(agent.get("name").is_some());
        assert!(agent.get("role").is_some());
        assert!(agent.get("department").is_some());
    }
}

#[tokio::test]
async fn test_get_single_agent_endpoint() {
    let state = create_test_app_state();
    let app = create_router(state);

    let client = reqwest::Client::new();

    // 启动测试服务器
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // 等待服务器启动
    tokio::time::sleep(Duration::from_millis(100)).await;

    // 请求存在的agent
    let response = client
        .get(format!("http://{}/api/agents/test-agent-1", addr))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let agent: serde_json::Value = response.json().await.unwrap();
    assert_eq!(agent["id"], "test-agent-1");
    assert_eq!(agent["name"], "Test Employee 1");

    // 请求不存在的agent
    let response = client
        .get(format!("http://{}/api/agents/non-existent", addr))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn test_send_message_endpoint() {
    let state = create_test_app_state();
    let app = create_router(state.clone()); // Clone for use in server

    let client = reqwest::Client::new();

    // 启动测试服务器
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // 等待服务器启动
    tokio::time::sleep(Duration::from_millis(100)).await;

    // 发送消息
    let message_request = serde_json::json!({
        "from": "test-agent-1",
        "to": "test-agent-2",
        "content": "测试消息内容"
    });

    let response = client
        .post(format!("http://{}/api/messages", addr))
        .json(&message_request)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let result: serde_json::Value = response.json().await.unwrap();
    assert_eq!(result["status"], "sent");
    assert!(result.get("id").is_some());
    assert!(result.get("timestamp").is_some());
}

#[tokio::test]
async fn test_company_info_endpoint() {
    let state = create_test_app_state();
    let app = create_router(state);

    let client = reqwest::Client::new();

    // 启动测试服务器
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // 等待服务器启动
    tokio::time::sleep(Duration::from_millis(100)).await;

    let response = client
        .get(format!("http://{}/api/company", addr))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let company_info: serde_json::Value = response.json().await.unwrap();
    assert_eq!(company_info["name"], "ImitatorT Virtual Company");
    assert!(company_info["agent_count"].as_i64().unwrap() >= 2);
}