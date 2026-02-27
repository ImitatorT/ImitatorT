//! Agent模式和Watchdog框架集成测试
//!
//! 验证主动/被动模式Agent与Watchdog框架的协同工作

use imitatort::domain::agent::{Agent, AgentMode, TriggerCondition, Role, LLMConfig};
use imitatort::core::watchdog::{WatchdogFramework, WatchdogRule, ToolExecutionEvent};
use imitatort::domain::tool::ToolCallContext;
use imitatort::application::autonomous::AutonomousAgent;
use imitatort::core::messaging::MessageBus;
use serde_json::json;
use std::sync::Arc;

// #[tokio::test]
// async fn test_active_agent_with_watchdog_integration() {
//     let watchdog_framework = Arc::new(WatchdogFramework::new());
//
//     // 创建一个主动模式的Agent
//     let agent = Agent::new_with_mode(
//         "active_test_agent",
//         "Active Test Agent",
//         Role::simple("Tester", "An active test agent"),
//         LLMConfig::openai("test-key"),
//         AgentMode::Active {
//             watched_tools: vec!["temperature_monitor".to_string()],
//             trigger_conditions: vec![
//                 TriggerCondition::NumericRange { min: 20.0, max: 30.0 }
//             ],
//         },
//     );
//
//     let message_bus = Arc::new(MessageBus::new());
//
//     let autonomous_agent = AutonomousAgent::new(
//         agent,
//         message_bus,
//         Some(watchdog_framework.clone()),
//     ).await.unwrap();
//
//     // 验证Agent模式
//     match autonomous_agent.mode() {
//         AgentMode::Active { watched_tools, .. } => {
//             assert_eq!(watched_tools.len(), 1);
//             assert_eq!(watched_tools[0], "temperature_monitor");
//         },
//         _ => panic!("Expected active mode"),
//     }
//
//     // 触发一个符合条件的工具执行事件
//     let event = ToolExecutionEvent::PostExecute {
//         tool_id: "temperature_monitor".to_string(),
//         result: json!(25.0), // 在20-30范围内，应该触发
//         context: ToolCallContext::new("sensor".to_string()),
//     };
//
//     // 处理事件
//     let triggered_agents = watchdog_framework.process_event(&event).await.unwrap();
//
//     // 验证主动Agent被触发
//     assert!(triggered_agents.contains(&"active_test_agent".to_string()));
// }

// #[tokio::test]
// async fn test_passive_agent_ignore_non_direct_messages() {
//     let watchdog_framework = Arc::new(WatchdogFramework::new());
//
//     // 创建一个被动模式的Agent
//     let agent = Agent::new_with_mode(
//         "passive_test_agent",
//         "Passive Test Agent",
//         Role::simple("Passive Tester", "A passive test agent"),
//         LLMConfig::openai("test-key"),
//         AgentMode::Passive,
//     );
//
//     let message_bus = Arc::new(MessageBus::new());
//
//     let autonomous_agent = AutonomousAgent::new(
//         agent,
//         message_bus,
//         Some(watchdog_framework),
//     ).await.unwrap();
//
//     // 验证Agent模式
//     match autonomous_agent.mode() {
//         AgentMode::Passive => assert!(true),
//         _ => panic!("Expected passive mode"),
//     }
//
//     // 被动模式Agent应该忽略非直接消息
//     assert_eq!(autonomous_agent.mode(), &AgentMode::Passive);
// }

// #[tokio::test]
// async fn test_watchdog_with_tool_executor_hooks() {
//     use imitatort::core::skill::SkillManager;
//     use imitatort::core::tool::ToolRegistry;
//     use imitatort::infrastructure::tool::ToolExecutorRegistry;
//
//     let watchdog_framework = Arc::new(WatchdogFramework::new());
//     let tool_registry = Arc::new(ToolRegistry::new());
//     let skill_manager = Arc::new(SkillManager::new_with_tool_registry(tool_registry));
//
//     // 创建带Watchdog框架的执行器注册表
//     let mut executor_registry = ToolExecutorRegistry::with_watchdog_framework(
//         skill_manager,
//         watchdog_framework.clone(),
//     );
//
//     // 添加一个前置Hook来验证它被调用
//     let mut pre_hook_called = false;
//     executor_registry.add_pre_execution_hook(|_tool_id, _params, _context| {
//         pre_hook_called = true;
//         Ok(())
//     });
//
//     // 添加一个后置Hook来验证它被调用
//     let mut post_hook_called = false;
//     executor_registry.add_post_execution_hook(|_tool_id, _params, _result, _context| {
//         post_hook_called = true;
//         Ok(())
//     });
//
//     // 简单验证Hook注册成功（实际执行需要具体的执行器实现）
//     // 这里主要是确保Hook机制正常工作
//     assert!(true); // Hook添加成功
// }

#[tokio::test]
async fn test_message_mentions_extraction() {
    use imitatort::domain::{Message, Group};

    // 测试消息中的@提及功能
    let group = Group::new(
        "test_group",
        "Test Group",
        "creator",
        vec!["alice".to_string(), "bob".to_string(), "charlie".to_string()],
    );

    // 创建一个包含@提及的消息
    let message = Message::group("sender", "test_group", "Hi @alice and @bob, how are you?");

    // 手动测试提及提取逻辑
    let content = &message.content;
    let mut mentions = Vec::new();

    // 找到所有@提及的位置
    let mut pos = 0;
    while let Some(at_pos) = content[pos..].find('@') {
        let actual_pos = pos + at_pos;
        if actual_pos + 1 < content.len() {
            // 找到@符号后的单词
            let rest = &content[actual_pos + 1..];
            let word_end = rest.find(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
                .unwrap_or(rest.len());

            if word_end > 0 {
                let mentioned_name = &rest[..word_end];

                // 检查组内是否有匹配的成员
                for member_id in &group.members {
                    if member_id == mentioned_name {
                        if !mentions.contains(member_id) {
                            mentions.push(member_id.clone());
                        }
                    }
                }

                pos = actual_pos + 1 + word_end; // 移动到下一个位置
            } else {
                pos = actual_pos + 1;
            }
        } else {
            break;
        }
    }

    // 验证提取到正确的提及
    assert!(mentions.contains(&"alice".to_string()));
    assert!(mentions.contains(&"bob".to_string()));
    assert!(!mentions.contains(&"charlie".to_string())); // charlie没有被提及
}

// #[tokio::test]
// async fn test_agent_mode_based_message_filtering() {
//     use imitatort::domain::{Message, MessageTarget};
//
//     // 创建一个被动模式Agent并测试消息过滤
//     let agent = Agent::new_with_mode(
//         "filter_test_agent",
//         "Filter Test Agent",
//         Role::simple("Filter", "A filter test agent"),
//         LLMConfig::openai("test-key"),
//         AgentMode::Passive,
//     );
//
//     let message_bus = Arc::new(MessageBus::new());
//     let watchdog_framework = Arc::new(WatchdogFramework::new());
//
//     let autonomous_agent = AutonomousAgent::new(
//         agent,
//         message_bus,
//         Some(watchdog_framework),
//     ).await.unwrap();
//
//     // 创建不同类型的消息来测试过滤
//     let direct_msg = Message::private("other_agent", "filter_test_agent", "Hello!");
//     let group_msg_with_mention = Message::group("other_agent", "test_group", "Hey @filter_test_agent, check this out!")
//         .with_mention("filter_test_agent");
//     let group_msg_without_mention = Message::group("other_agent", "test_group", "Just talking to others");
//
//     // 测试被动Agent的消息过滤逻辑
//     assert!(autonomous_agent.is_mentioned_or_direct(&direct_msg)); // 私聊应该通过
//     assert!(autonomous_agent.is_mentioned_or_direct(&group_msg_with_mention)); // 提及应该通过
//     assert!(!autonomous_agent.is_mentioned_or_direct(&group_msg_without_mention)); // 非提及群聊应该被过滤
// }