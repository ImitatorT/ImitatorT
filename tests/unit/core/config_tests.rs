//! Config 模块单元测试

use imitatort_stateless_company::core::config::{AppConfig, OutputMode, StoreType};

#[test]
fn test_output_mode_parse() {
    assert_eq!("matrix".parse::<OutputMode>().unwrap(), OutputMode::Matrix);
    assert_eq!("cli".parse::<OutputMode>().unwrap(), OutputMode::Cli);
    assert_eq!("a2a".parse::<OutputMode>().unwrap(), OutputMode::A2A);
    assert_eq!("hybrid".parse::<OutputMode>().unwrap(), OutputMode::Hybrid);
    
    // 测试大小写不敏感
    assert_eq!("MATRIX".parse::<OutputMode>().unwrap(), OutputMode::Matrix);
    assert_eq!("Cli".parse::<OutputMode>().unwrap(), OutputMode::Cli);
    
    // 测试无效值
    assert!("unknown".parse::<OutputMode>().is_err());
    assert!("".parse::<OutputMode>().is_err());
}

#[test]
fn test_output_mode_display() {
    assert_eq!(OutputMode::Matrix.to_string(), "matrix");
    assert_eq!(OutputMode::Cli.to_string(), "cli");
    assert_eq!(OutputMode::A2A.to_string(), "a2a");
    assert_eq!(OutputMode::Hybrid.to_string(), "hybrid");
}

#[test]
fn test_store_type_parse() {
    assert_eq!("memory".parse::<StoreType>().unwrap(), StoreType::Memory);
    
    // 测试大小写不敏感
    assert_eq!("MEMORY".parse::<StoreType>().unwrap(), StoreType::Memory);
    assert_eq!("Memory".parse::<StoreType>().unwrap(), StoreType::Memory);
    
    // 测试无效值
    assert!("unknown".parse::<StoreType>().is_err());
    assert!("disk".parse::<StoreType>().is_err());
}

#[test]
fn test_store_type_display() {
    assert_eq!(StoreType::Memory.to_string(), "memory");
}

#[test]
fn test_output_mode_equality() {
    assert_eq!(OutputMode::Matrix, OutputMode::Matrix);
    assert_ne!(OutputMode::Matrix, OutputMode::Cli);
}

#[test]
fn test_store_type_equality() {
    assert_eq!(StoreType::Memory, StoreType::Memory);
}

#[test]
fn test_output_mode_clone() {
    let mode = OutputMode::A2A;
    let cloned = mode.clone();
    assert_eq!(mode, cloned);
}

#[test]
fn test_store_type_clone() {
    let store_type = StoreType::Memory;
    let cloned = store_type.clone();
    assert_eq!(store_type, cloned);
}

#[test]
fn test_output_mode_debug() {
    let mode = OutputMode::Hybrid;
    let debug_str = format!("{:?}", mode);
    assert!(debug_str.contains("Hybrid"));
}

#[test]
fn test_store_type_debug() {
    let store_type = StoreType::Memory;
    let debug_str = format!("{:?}", store_type);
    assert!(debug_str.contains("Memory"));
}
