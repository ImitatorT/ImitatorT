//! Organization 领域实体测试

use imitatort_stateless_company::{Agent, Department, LLMConfig, Organization, Role};

#[test]
fn test_organization_building() {
    let mut org = Organization::new();

    // 添加部门
    let dept_tech = Department::top_level("tech", "技术部").with_leader("cto");
    let dept_fe = Department::child("fe", "前端组", "tech").with_leader("lead-fe");

    org.add_department(dept_tech);
    org.add_department(dept_fe);

    // 添加Agent
    let cto = Agent::new(
        "cto",
        "CTO",
        Role::simple("CTO", "你是CTO"),
        LLMConfig::openai("test"),
    )
    .with_department("tech");

    org.add_agent(cto);

    // 验证
    assert_eq!(org.departments.len(), 2);
    assert_eq!(org.agents.len(), 1);
    assert!(org.find_agent("cto").is_some());
    assert!(org.find_department("tech").is_some());
}

#[test]
fn test_department_tree() {
    let mut org = Organization::new();

    org.add_department(Department::top_level("root", "总部"));
    org.add_department(Department::child("d1", "部门1", "root"));
    org.add_department(Department::child("d2", "部门2", "root"));
    org.add_department(Department::child("d1-1", "子部门", "d1"));

    let tree = org.build_tree();
    assert_eq!(tree.len(), 1); // 一个根
    assert_eq!(tree[0].children.len(), 2); // 两个子部门
}

#[test]
fn test_get_members() {
    let mut org = Organization::new();

    org.add_department(Department::top_level("tech", "技术部"));

    let dev = Agent::new(
        "dev1",
        "开发者",
        Role::simple("Dev", "你是开发者"),
        LLMConfig::openai("test"),
    )
    .with_department("tech");

    org.add_agent(dev);

    let members = org.get_department_members("tech");
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].id, "dev1");
}
