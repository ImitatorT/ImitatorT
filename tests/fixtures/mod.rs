//! 测试 Fixtures
//!
//! 提供测试数据和模拟对象

use std::collections::HashMap;

/// 创建测试用的 Agent 配置
pub fn test_agent_config(id: &str, name: &str) -> imitatort_stateless_company::core::agent::AgentConfig {
    let mut metadata = serde_json::Map::new();
    metadata.insert("test".to_string(), serde_json::json!(true));

    imitatort_stateless_company::core::agent::AgentConfig {
        id: id.to_string(),
        name: name.to_string(),
        system_prompt: "You are a test agent.".to_string(),
        model: "gpt-4o-mini".to_string(),
        api_key: "sk-test".to_string(),
        base_url: "http://localhost:8080".to_string(),
        metadata,
    }
}

/// 创建测试用的 AgentInfo
pub fn test_agent_info(id: &str, endpoint: &str) -> imitatort_stateless_company::protocol::server::AgentInfo {
    imitatort_stateless_company::protocol::server::AgentInfo {
        id: id.to_string(),
        name: format!("Test Agent {}", id),
        endpoint: endpoint.to_string(),
        capabilities: vec!["chat".to_string(), "test".to_string()],
        metadata: None,
    }
}

/// 创建测试用的 AgentCard
pub fn test_agent_card(id: &str) -> imitatort_stateless_company::protocol::types::AgentCard {
    imitatort_stateless_company::protocol::types::AgentCard {
        id: id.to_string(),
        name: format!("Test Agent {}", id),
        description: "A test agent for unit tests".to_string(),
        version: "0.1.0".to_string(),
        capabilities: vec!["chat".to_string()],
        endpoints: imitatort_stateless_company::protocol::types::AgentEndpoints {
            a2a_endpoint: format!("/a2a/agents/{}", id),
            webhook_endpoint: None,
        },
        skills: vec![imitatort_stateless_company::protocol::types::Skill {
            name: "conversation".to_string(),
            description: "Test conversation skill".to_string(),
            parameters: serde_json::json!({"type": "object"}),
        }],
    }
}

/// 创建测试消息
pub fn test_message(from: &str, to: &str, content: &str) -> imitatort_stateless_company::core::messaging::Message {
    imitatort_stateless_company::core::messaging::Message::private(from, to, content)
}

/// 创建测试群聊信息
pub fn test_group_info(id: &str, creator: &str) -> imitatort_stateless_company::core::messaging::GroupInfo {
    imitatort_stateless_company::core::messaging::GroupInfo::new(
        id,
        &format!("Test Group {}", id),
        creator,
        vec![creator.to_string()],
    )
}

/// 创建测试 ChatMessage
pub fn test_chat_message(id: &str, sender: &str, content: &str) -> imitatort_stateless_company::core::store::ChatMessage {
    imitatort_stateless_company::core::store::ChatMessage {
        id: id.to_string(),
        sender: sender.to_string(),
        content: content.to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        message_type: imitatort_stateless_company::core::store::MessageType::Text,
    }
}

/// 创建测试用的 LLM Message
pub fn test_llm_message(role: &str, content: &str) -> imitatort_stateless_company::infrastructure::llm::Message {
    imitatort_stateless_company::infrastructure::llm::Message {
        role: role.to_string(),
        content: Some(content.to_string()),
        tool_calls: None,
        tool_call_id: None,
    }
}

/// 测试用的 API 响应数据
pub fn test_api_response<T: serde::Serialize>(data: T) -> serde_json::Value {
    serde_json::json!({
        "success": true,
        "data": data,
        "error": null::<String>
    })
}

/// 测试用的 API 错误响应
pub fn test_api_error(error: &str) -> serde_json::Value {
    serde_json::json!({
        "success": false,
        "data": null::<String>,
        "error": error
    })
}
