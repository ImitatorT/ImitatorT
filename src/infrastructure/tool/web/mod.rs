//! Web 工具模块
//!
//! 提供网络搜索、网页访问、特定网站信息获取等能力

pub mod search;
pub mod visit;
pub mod wikipedia;
pub mod reddit;
pub mod bloomberg;
pub mod polymarket;

// 重新导出主要类型和函数
pub use search::{WebSearchTool, execute_web_search};
pub use visit::{WebVisitTool, execute_web_visit};
pub use wikipedia::{WikipediaTool, execute_wikipedia};
pub use reddit::{RedditTool, execute_reddit};
pub use bloomberg::{BloombergTool, execute_bloomberg};
pub use polymarket::{PolymarketTool, execute_polymarket};
