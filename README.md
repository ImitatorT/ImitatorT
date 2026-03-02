# ImitatorT - Multi-Agent Company Simulation Framework

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.85+-orange.svg)](https://www.rust-lang.org/)
[![Version](https://img.shields.io/badge/version-0.2.0-green.svg)](Cargo.toml)

A lightweight, production-ready multi-agent framework that simulates real company operations with autonomous AI agents. Built with Rust for performance and reliability, featuring a clean layered architecture and out-of-the-box functionality.

## 🚀 Features

- **Self-Organizing Agents**: Agents autonomously decide to create groups, initiate private chats, and execute tasks
- **Built-in Web Service**: Integrated web server with REST API and WebSocket support
- **Hierarchical Organization**: Support for departments and reporting structures
- **Message-Driven Architecture**: Agent collaboration through realistic communication patterns
- **Spring Boot Inspired**: Convention over configuration, auto-configuration capabilities
- **Production Ready**: Designed for stability, performance, and maintainability

## 📋 Prerequisites

- Rust 1.85 or higher
- An OpenAI-compatible API key (OpenAI, Azure OpenAI, or compatible providers)

## 🚀 Quick Start

### Method 1: Quick Start (Recommended)

```rust
use imitatort::quick_start;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    quick_start().await?;
    Ok(())
}
```

### Method 2: Manual Configuration

```rust
use imitatort::{VirtualCompany, CompanyBuilder, CompanyConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = CompanyConfig::test_config(); // Or load from YAML file
    let company = CompanyBuilder::from_config(config)?
        .build_and_save()
        .await?;

    company.run().await?;
    Ok(())
}
```

### Method 3: Custom Configuration

```rust
use imitatort::{FrameworkLauncher, AppConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::from_env();
    let launcher = FrameworkLauncher::with_config(config);
    launcher.launch().await?;
    Ok(())
}
```

## ⚙️ Configuration

### Environment Variables

```bash
# Database path
DB_PATH=imitatort.db

# Web server binding address
WEB_BIND=0.0.0.0:8080

# Output mode (cli or web)
OUTPUT_MODE=web

# Whether to run agent autonomous loops
RUN_AGENT_LOOPS=true

# Default API base URL
DEFAULT_API_BASE_URL=https://api.openai.com/v1

# Default model name
DEFAULT_MODEL=gpt-4o-mini

# Log level
LOG_LEVEL=info
```

### Company Configuration (YAML)

Create `company_config.yaml` in your project root:

```yaml
name: "AI Research Company"
organization:
  departments:
    - id: "research"
      name: "Research Department"
    - id: "engineering"
      name: "Engineering Department"
      parent_id: "research"
    - id: "marketing"
      name: "Marketing Department"

  agents:
    - id: "ceo"
      name: "CEO"
      role:
        title: "Chief Executive Officer"
        responsibilities:
          - "Strategic decision making"
          - "Company direction"
        expertise:
          - "Business strategy"
          - "Leadership"
        system_prompt: |
          You are the CEO of an AI research company. Your role is to make strategic decisions,
          guide the company direction, and coordinate between departments. You should be decisive
          yet collaborative, considering input from various team members.
      llm_config:
        model: "gpt-4o-mini"
        api_key: "${OPENAI_API_KEY}"  # Will be loaded from environment
        base_url: "https://api.openai.com/v1"
      mode: "passive"  # Options: "passive" or "active"

    - id: "cto"
      name: "CTO"
      role:
        title: "Chief Technology Officer"
        responsibilities:
          - "Technical architecture"
          - "Development oversight"
        expertise:
          - "Software architecture"
          - "AI/ML systems"
        system_prompt: |
          You are the CTO responsible for technical decisions and development oversight.
          Coordinate with the engineering team and report to the CEO on technical matters.
      llm_config:
        model: "gpt-4o-mini"
        api_key: "${OPENAI_API_KEY}"
        base_url: "https://api.openai.com/v1"
      mode: "passive"
```

## 🏗️ Architecture

The framework follows a clean, layered architecture based on Domain-Driven Design principles:

```
imitatort/
├── src/
│   ├── domain/              # Domain entities (Agent, Message, Organization, Tool, Capability)
│   │   ├── agent.rs         # Agent entity and related types
│   │   ├── message.rs       # Message entity and communication types
│   │   ├── org.rs           # Organization structure
│   │   ├── user.rs          # User management
│   │   ├── tool.rs          # Tool definitions
│   │   ├── capability.rs    # Capability definitions
│   │   └── skill.rs         # Skill definitions
│   ├── core/                # Core capabilities (AgentRuntime, Messaging, Store, Tool/Capability Registries)
│   │   ├── agent.rs         # Agent runtime and decision-making (now uses structured JSON responses)
│   │   ├── messaging.rs     # Message bus and communication
│   │   ├── store/           # Storage abstractions
│   │   ├── tool.rs          # Tool registry and management
│   │   ├── capability.rs    # Capability registry and management
│   │   ├── skill.rs         # Skill management
│   │   ├── config.rs        # Company configuration
│   │   ├── watchdog/        # Watchdog monitoring system
│   │   └── ...              # Other core modules
│   ├── application/         # Application logic (VirtualCompany, AutonomousAgent)
│   │   ├── autonomous/      # Autonomous agent implementation
│   │   ├── company_runtime.rs # Company runtime components
│   │   ├── framework.rs     # Main framework API (VirtualCompany)
│   │   └── organization.rs  # Organization management
│   ├── infrastructure/      # External integrations (LLM, Web, Storage, Auth)
│   │   ├── llm.rs           # LLM client implementations
│   │   ├── web/             # HTTP API and WebSocket server
│   │   ├── store/           # Storage implementations (SQLite, etc.)
│   │   ├── tool/            # Tool execution system
│   │   ├── capability/      # Capability execution system
│   │   ├── auth.rs          # Authentication system
│   │   └── logger.rs        # Logging setup
│   ├── bootstrap.rs         # Auto-configuration module
│   ├── config.rs            # Application configuration management
│   ├── errors.rs            # Error types and handling
│   └── lib.rs               # Public API exports
├── tests/                   # Integration and unit tests
├── examples/                # Usage examples
├── frontend/                # Web interface (React + TypeScript)
├── Cargo.toml               # Dependencies and project metadata
├── README.md                # Project documentation
└── LICENSE                  # Apache 2.0 License
```

### Key Components

- **Domain Layer**: Pure business entities without external dependencies, defining the core concepts of the system
- **Core Layer**: Essential runtime capabilities, business logic, and service abstractions
- **Application Layer**: Business logic orchestration and main framework APIs
- **Infrastructure Layer**: External system integrations (LLMs, databases, web services, authentication)

### Advanced Features

- **Tool System**: Extensible framework for adding custom tools that agents can use
- **Capability System**: Advanced functionality accessible through MCP protocol
- **Skill System**: Higher-level composite behaviors built from tools and capabilities
- **Watchdog System**: Monitoring and triggering system for automated responses
- **Structured Decision Making**: JSON-based communication between agents and LLMs for more reliable parsing

## 🔧 Running the Framework

### Development Mode

```bash
# Install dependencies
cargo build

# Run with web interface
cargo run

# Or run with custom configuration
OUTPUT_MODE=web WEB_BIND=0.0.0.0:8080 cargo run
```

### Production Mode

```bash
# Build for release
cargo build --release

# Run production binary
./target/release/imitatort
```

## 🧪 Testing

Run all tests:

```bash
# Run unit and integration tests
cargo test

# Run tests with specific filters
cargo test -- --nocapture

# Run with specific features
cargo test --features "testing"
```

## 📦 Usage as Dependency

Add to your `Cargo.toml`:

```toml
[dependencies]
imitatort = "0.2.0"
tokio = { version = "1", features = ["full"] }
anyhow = "1"
serde = { version = "1", features = ["derive"] }
```

Or install via cargo:

```bash
cargo add imitatort
```

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Add tests for your changes
5. Run the test suite (`cargo test`)
6. Commit your changes (`git commit -m 'Add amazing feature'`)
7. Push to the branch (`git push origin feature/amazing-feature`)
8. Open a Pull Request

## 📄 License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Inspired by Spring Boot's auto-configuration philosophy
- Built with the amazing Rust ecosystem
- Special thanks to the AI research community

## 🐛 Issues

If you encounter any issues, please file them in our [Issue Tracker](https://github.com/imitatort/imitatort/issues).

---

Made with ❤️ using Rust
