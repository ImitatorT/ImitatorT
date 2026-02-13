//! Output 模块单元测试

use imitatort_stateless_company::application::output::{
    A2AOutput, CliOutput, MatrixOutput, Output, OutputBridge, OutputFactory, OutputMode,
};
use imitatort_stateless_company::core::store::MessageStore;
use std::sync::Arc;

#[test]
fn test_output_mode_from_str() {
    assert_eq!("matrix".parse::<OutputMode>().unwrap(), OutputMode::Matrix);
    assert_eq!("cli".parse::<OutputMode>().unwrap(), OutputMode::Cli);
    assert_eq!("a2a".parse::<OutputMode>().unwrap(), OutputMode::A2A);

    // 无效值
    assert!("unknown".parse::<OutputMode>().is_err());
    assert!("".parse::<OutputMode>().is_err());
}

#[test]
fn test_output_mode_display() {
    assert_eq!(OutputMode::Matrix.to_string(), "matrix");
    assert_eq!(OutputMode::Cli.to_string(), "cli");
    assert_eq!(OutputMode::A2A.to_string(), "a2a");
}

#[test]
fn test_output_mode_equality() {
    assert_eq!(OutputMode::Matrix, OutputMode::Matrix);
    assert_ne!(OutputMode::Matrix, OutputMode::Cli);
}

#[test]
fn test_output_mode_clone() {
    let mode = OutputMode::A2A;
    let cloned = mode.clone();
    assert_eq!(mode, cloned);
}

#[test]
fn test_output_mode_debug() {
    let mode = OutputMode::Cli;
    let debug_str = format!("{:?}", mode);
    assert!(debug_str.contains("Cli"));
}

#[tokio::test]
async fn test_cli_output() {
    let store = MessageStore::new(10);
    let cli = CliOutput::new(store.clone(), false);

    cli.send_message("test-agent", "Hello CLI").await.unwrap();

    let context = cli.get_context(10).await.unwrap();
    assert!(context.contains("Hello CLI"));
}

#[tokio::test]
async fn test_cli_output_with_echo() {
    let store = MessageStore::new(10);
    let cli = CliOutput::new(store.clone(), true);

    // When echo is true, it should print to stdout
    cli.send_message("test-agent", "Test message")
        .await
        .unwrap();

    let context = cli.get_context(10).await.unwrap();
    assert!(context.contains("Test message"));
}

#[tokio::test]
async fn test_a2a_output_context() {
    use imitatort_stateless_company::protocol::types::{create_default_agent_card, A2AAgent};

    let card = create_default_agent_card("agent-1", "Test Agent");
    let agent = Arc::new(A2AAgent::new(card));
    let store = MessageStore::new(10);
    let a2a = A2AOutput::new(agent, store.clone(), None);

    a2a.send_message("test-agent", "Hello A2A").await.unwrap();

    let context = a2a.get_context(10).await.unwrap();
    assert!(context.contains("Hello A2A"));
}

#[tokio::test]
async fn test_output_mode_methods() {
    let store = MessageStore::new(10);
    let cli = CliOutput::new(store, false);

    assert_eq!(cli.mode(), OutputMode::Cli);
}

#[test]
fn test_output_factory_create_cli() {
    let store = MessageStore::new(10);
    let output = OutputFactory::create_cli(store, false);

    assert_eq!(output.mode(), OutputMode::Cli);
}

#[tokio::test]
async fn test_cli_output_empty_context() {
    let store = MessageStore::new(10);
    let cli = CliOutput::new(store, false);

    let context = cli.get_context(10).await.unwrap();
    assert_eq!(context, "(No previous context)");
}

#[tokio::test]
async fn test_a2a_output_empty_context() {
    use imitatort_stateless_company::protocol::types::{create_default_agent_card, A2AAgent};

    let card = create_default_agent_card("agent-1", "Test Agent");
    let agent = Arc::new(A2AAgent::new(card));
    let store = MessageStore::new(10);
    let a2a = A2AOutput::new(agent, store, None);

    let context = a2a.get_context(10).await.unwrap();
    assert_eq!(context, "(No previous context)");
}

#[tokio::test]
async fn test_cli_output_multiple_messages() {
    let store = MessageStore::new(10);
    let cli = CliOutput::new(store.clone(), false);

    cli.send_message("agent-1", "Message 1").await.unwrap();
    cli.send_message("agent-2", "Message 2").await.unwrap();
    cli.send_message("agent-1", "Message 3").await.unwrap();

    let context = cli.get_context(10).await.unwrap();
    assert!(context.contains("Message 1"));
    assert!(context.contains("Message 2"));
    assert!(context.contains("Message 3"));
}

#[tokio::test]
async fn test_cli_output_context_limit() {
    let store = MessageStore::new(10);
    let cli = CliOutput::new(store.clone(), false);

    for i in 1..=5 {
        cli.send_message("agent", &format!("Message {}", i))
            .await
            .unwrap();
    }

    // Request only recent 3
    let context = cli.get_context(3).await.unwrap();
    assert!(!context.contains("Message 1"));
    assert!(!context.contains("Message 2"));
    assert!(context.contains("Message 3"));
    assert!(context.contains("Message 4"));
    assert!(context.contains("Message 5"));
}

#[test]
fn test_output_bridge_creation() {
    use imitatort_stateless_company::protocol::types::{create_default_agent_card, A2AAgent};
    use tokio::sync::mpsc;

    let store = MessageStore::new(10);
    let cli = CliOutput::new(store, false);

    let card = create_default_agent_card("agent-1", "Test Agent");
    let agent = Arc::new(A2AAgent::new(card));
    let a2a_store = MessageStore::new(10);
    let a2a = A2AOutput::new(agent, a2a_store, None);

    let (_bridge, _tx): (OutputBridge, mpsc::Sender<imitatort_stateless_company::protocol::types::A2AMessage>) =
        OutputBridge::new(Some(Arc::new(cli)), Some(Arc::new(a2a)));
}
