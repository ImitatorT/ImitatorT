//! ToolProvider 实现测试

use imitatort_stateless_company::core::tool_provider::FrameworkToolProvider;
use imitatort_stateless_company::domain::tool::{MatchType, ToolProvider};

#[test]
fn test_framework_tool_provider() {
    let provider = FrameworkToolProvider::new();
    let tools = provider.list_tools();

    assert!(!tools.is_empty());
    assert!(tools.iter().any(|t| t.id == "tool.search"));
    assert!(tools.iter().any(|t| t.id == "time.now"));
    assert!(tools.iter().any(|t| t.id == "org.find_agents"));
}

#[test]
fn test_search_tools() {
    let provider = FrameworkToolProvider::new();

    let results = provider.search_tools("tool", MatchType::Fuzzy);
    assert!(!results.is_empty());

    let results = provider.search_tools("time.now", MatchType::Exact);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "time.now");
}

#[test]
fn test_list_by_category() {
    let provider = FrameworkToolProvider::new();

    let tools = provider.list_tools_by_category("tool/query");
    assert!(!tools.is_empty());
    assert!(tools.iter().all(|t| t.category.to_path_string().starts_with("tool")));
}
