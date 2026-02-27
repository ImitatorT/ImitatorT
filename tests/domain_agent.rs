//! Agent 领域实体测试

use imitatort::{Agent, LLMConfig, Role};

#[test]
fn test_agent_creation() {
    let role = Role::simple("工程师", "你是一个软件工程师");
    let llm = LLMConfig::openai("test-key");
    let agent = Agent::new("agent-001", "张三", role, llm);

    assert_eq!(agent.id, "agent-001");
    assert_eq!(agent.name, "张三");
    assert_eq!(agent.role.title, "工程师");
}

#[test]
fn test_system_prompt() {
    let role = Role::simple("研究员", "你是一个AI研究员，专注于机器学习。");
    let llm = LLMConfig::openai("test-key");
    let agent = Agent::new("a1", "李四", role, llm);

    let prompt = agent.system_prompt();
    assert!(prompt.contains("AI研究员"));
    assert!(prompt.contains("机器学习"));
}

#[test]
fn test_llm_config() {
    let config = LLMConfig::openai("sk-test")
        .with_model("gpt-4")
        .with_base_url("http://localhost:8080");

    assert_eq!(config.model, "gpt-4");
    assert_eq!(config.api_key, "sk-test");
    assert_eq!(config.base_url, "http://localhost:8080");
}
