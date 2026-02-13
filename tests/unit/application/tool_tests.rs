//! Tool 模块单元测试

use imitatort_stateless_company::application::tool::{
    Function, FunctionCall, Tool, ToolCall, ToolRegistry,
};
use serde_json::json;

#[test]
fn test_get_tools() {
    let tools = ToolRegistry::get_tools();
    assert_eq!(tools.len(), 2);

    // Check execute_command tool
    assert_eq!(tools[0].function.name, "execute_command");
    assert_eq!(tools[0].r#type, "function");

    // Check fetch_url tool
    assert_eq!(tools[1].function.name, "fetch_url");
    assert_eq!(tools[1].r#type, "function");
}

#[test]
fn test_tool_creation() {
    let tool = Tool {
        r#type: "function".to_string(),
        function: Function {
            name: "test_function".to_string(),
            description: "A test function".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "arg1": { "type": "string" }
                }
            }),
        },
    };

    assert_eq!(tool.r#type, "function");
    assert_eq!(tool.function.name, "test_function");
}

#[test]
fn test_function_call_deserialize() {
    let json = r#"{"name": "test_func", "arguments": "{\"arg\": 1}"}"#;
    let func: FunctionCall = serde_json::from_str(json).unwrap();
    assert_eq!(func.name, "test_func");
    assert_eq!(func.arguments, "{\"arg\": 1}");
}

#[test]
fn test_tool_call_deserialize() {
    let json = r#"{
        "id": "call_123",
        "type": "function",
        "function": {
            "name": "execute_command",
            "arguments": "{\"command\": \"ls\"}"
        }
    }"#;
    let tool_call: ToolCall = serde_json::from_str(json).unwrap();
    assert_eq!(tool_call.id, "call_123");
    assert_eq!(tool_call.r#type, "function");
    assert_eq!(tool_call.function.name, "execute_command");
}

#[test]
fn test_tool_call_clone() {
    let tool_call = ToolCall {
        id: "call_123".to_string(),
        r#type: "function".to_string(),
        function: FunctionCall {
            name: "execute_command".to_string(),
            arguments: r#"{"command": "ls"}"#.to_string(),
        },
    };

    let cloned = tool_call.clone();
    assert_eq!(tool_call.id, cloned.id);
    assert_eq!(tool_call.function.name, cloned.function.name);
}

#[test]
fn test_function_clone() {
    let func = Function {
        name: "test".to_string(),
        description: "Test".to_string(),
        parameters: json!({}),
    };

    let cloned = func.clone();
    assert_eq!(func.name, cloned.name);
    assert_eq!(func.description, cloned.description);
}

#[test]
fn test_tool_clone() {
    let tool = Tool {
        r#type: "function".to_string(),
        function: Function {
            name: "test".to_string(),
            description: "Test".to_string(),
            parameters: json!({}),
        },
    };

    let cloned = tool.clone();
    assert_eq!(tool.r#type, cloned.r#type);
}

#[test]
fn test_tool_serialization() {
    let tool = Tool {
        r#type: "function".to_string(),
        function: Function {
            name: "test".to_string(),
            description: "Test function".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "arg": { "type": "string" }
                }
            }),
        },
    };

    let json = serde_json::to_string(&tool).unwrap();
    assert!(json.contains("function"));
    assert!(json.contains("test"));
    assert!(json.contains("Test function"));
}

#[test]
fn test_tool_call_debug() {
    let tool_call = ToolCall {
        id: "call_123".to_string(),
        r#type: "function".to_string(),
        function: FunctionCall {
            name: "execute_command".to_string(),
            arguments: "{}".to_string(),
        },
    };

    let debug_str = format!("{:?}", tool_call);
    assert!(debug_str.contains("call_123"));
    assert!(debug_str.contains("execute_command"));
}

#[test]
fn test_function_call_debug() {
    let func_call = FunctionCall {
        name: "test".to_string(),
        arguments: "{}".to_string(),
    };

    let debug_str = format!("{:?}", func_call);
    assert!(debug_str.contains("test"));
}

#[test]
fn test_function_debug() {
    let func = Function {
        name: "test".to_string(),
        description: "Test".to_string(),
        parameters: json!({}),
    };

    let debug_str = format!("{:?}", func);
    assert!(debug_str.contains("test"));
}
