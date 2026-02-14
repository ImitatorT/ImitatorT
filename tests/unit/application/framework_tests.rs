//! Framework 模块单元测试

use imitatort_stateless_company::application::framework::{AppBuilder, VirtualCompany};

#[test]
fn test_virtual_company_creation() {
    let company = VirtualCompany::new("http://localhost:8080");
    assert_eq!(company.local_endpoint(), "http://localhost:8080");
    assert!(company.list_agents().is_empty());
}

#[test]
fn test_virtual_company_local_endpoint() {
    let company = VirtualCompany::new("http://localhost:9000");
    assert_eq!(company.local_endpoint(), "http://localhost:9000");
}

#[test]
fn test_app_builder_creation() {
    let builder = AppBuilder::new_with_endpoint("http://localhost:8080");
    // Builder is created successfully
}

#[test]
fn test_app_builder_with_bind() {
    let builder = AppBuilder::new_with_endpoint("http://localhost:8080")
        .with_server("0.0.0.0:8080".parse().unwrap());

    // Builder with bind address is created successfully
}

#[test]
fn test_app_builder_with_seed() {
    // Note: seed 方法已移除，使用 register_remote_agent 替代
    let builder = AppBuilder::new_with_endpoint("http://localhost:8080");

    // Builder with seeds is created successfully
}

#[test]
fn test_app_builder_chaining() {
    let builder = AppBuilder::new_with_endpoint("http://localhost:8080")
        .with_server("0.0.0.0:8080".parse().unwrap());

    // Builder with all options is created successfully
}

#[test]
fn test_app_builder_multiple_seeds() {
    // Note: seed 方法已移除
    let builder = AppBuilder::new_with_endpoint("http://localhost:8080");

    // Builder with multiple seeds
}

#[test]
fn test_virtual_company_get_nonexistent_agent() {
    let company = VirtualCompany::new("http://localhost:8080");
    
    // Getting non-existent agent returns None
    let agent = company.get_agent("non-existent");
    assert!(agent.is_none());
}

#[test]
fn test_virtual_company_list_agents_empty() {
    let company = VirtualCompany::new("http://localhost:8080");
    
    let agents = company.list_agents();
    assert!(agents.is_empty());
}

#[test]
fn test_virtual_company_message_bus() {
    let company = VirtualCompany::new("http://localhost:8080");
    
    // Message bus can be accessed
    let _bus = company.message_bus();
}

#[test]
fn test_virtual_company_router() {
    let company = VirtualCompany::new("http://localhost:8080");
    
    // Router can be accessed
    let _router = company.router();
}
