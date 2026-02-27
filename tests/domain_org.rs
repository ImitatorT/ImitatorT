//! Organization 领域实体测试

use imitatort::{Agent, Department, LLMConfig, Organization, Role};

#[test]
fn test_organization_building() {
    let mut org = Organization::new();

    // Add department
    let dept_tech = Department::top_level("tech", "Technology Department").with_leader("cto");
    let dept_fe = Department::child("fe", "Frontend Team", "tech").with_leader("lead-fe");

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

    org.add_department(Department::top_level("root", "Headquarters"));
    org.add_department(Department::child("d1", "Department 1", "root"));
    org.add_department(Department::child("d2", "Department 2", "root"));
    org.add_department(Department::child("d1-1", "Sub-department", "d1"));

    let tree = org.build_tree();
    assert_eq!(tree.len(), 1); // 一个根
    assert_eq!(tree[0].children.len(), 2); // two sub-departments
}

#[test]
fn test_get_members() {
    let mut org = Organization::new();

    org.add_department(Department::top_level("tech", "Technology Department"));

    let dev = Agent::new(
        "dev1",
        "Developer",
        Role::simple("Dev", "You are a developer"),
        LLMConfig::openai("test"),
    )
    .with_department("tech");

    org.add_agent(dev);

    let members = org.get_department_members("tech");
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].id, "dev1");
}
