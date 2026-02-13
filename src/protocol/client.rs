//! A2A HTTP 客户端
//!
//! 用于与其他 Agent 的 HTTP 服务通信：
//! - 向其他 Agent 发送消息
//! - 发现远程 Agent
//! - 注册到远程 Agent

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;
use tracing::{debug, info, warn};

use crate::protocol::server::{
    AgentInfo, ApiResponse, CreateGroupRequest, InviteMemberRequest, RegisterAgentRequest,
    SendMessageRequest,
};
use crate::core::messaging::GroupInfo;

/// A2A HTTP 客户端
pub struct A2AClient {
    http: Client,
    local_endpoint: String,
}

impl A2AClient {
    /// 创建新的 A2A 客户端
    pub fn new(local_endpoint: impl Into<String>) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            http,
            local_endpoint: local_endpoint.into(),
        }
    }

    /// 发送 HTTP 请求
    async fn request<B: Serialize, R: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        url: &str,
        body: Option<&B>,
    ) -> Result<R> {
        let mut req = self.http.request(method, url);

        if let Some(b) = body {
            req = req.json(b);
        }

        let response = req.send().await.context("HTTP request failed")?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("HTTP {}: {}", status, text));
        }

        let api_response: ApiResponse<R> =
            response.json().await.context("Failed to parse response")?;

        if !api_response.success {
            return Err(anyhow::anyhow!(api_response
                .error
                .unwrap_or_else(|| "Unknown error".to_string())));
        }

        api_response.data.context("Empty response data")
    }

    /// 健康检查
    pub async fn health_check(&self, endpoint: &str) -> Result<bool> {
        let url = format!("{}/health", endpoint);

        match self.http.get(&url).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(e) => {
                debug!("Health check failed for {}: {}", endpoint, e);
                Ok(false)
            }
        }
    }

    /// 注册本地 Agent 到远程 Agent
    ///
    /// 用于让远程 Agent 发现自己
    pub async fn register_to_remote(
        &self,
        remote_endpoint: &str,
        agent_info: &AgentInfo,
    ) -> Result<()> {
        let url = format!("{}/agents/register", remote_endpoint);

        let req = RegisterAgentRequest {
            id: agent_info.id.clone(),
            name: agent_info.name.clone(),
            endpoint: self.local_endpoint.clone(),
            capabilities: agent_info.capabilities.clone(),
            metadata: agent_info.metadata.clone(),
        };

        let _: String = self
            .request(reqwest::Method::POST, &url, Some(&req))
            .await?;

        info!("Registered to remote agent at {}", remote_endpoint);
        Ok(())
    }

    /// 发现远程 Agent 的所有已知 Agent
    pub async fn discover_agents(&self, remote_endpoint: &str) -> Result<Vec<AgentInfo>> {
        let url = format!("{}/agents", remote_endpoint);

        let agents: Vec<AgentInfo> = self
            .request(reqwest::Method::GET, &url, None::<&()>)
            .await?;

        debug!(
            "Discovered {} agents from {}",
            agents.len(),
            remote_endpoint
        );
        Ok(agents)
    }

    /// 发送私聊消息给远程 Agent
    pub async fn send_private(
        &self,
        remote_endpoint: &str,
        from: &str,
        to: &str,
        content: &str,
    ) -> Result<()> {
        let url = format!("{}/messages", remote_endpoint);

        let req = SendMessageRequest {
            from: from.to_string(),
            to: vec![to.to_string()],
            content: content.to_string(),
            msg_type: "private".to_string(),
        };

        let _: String = self
            .request(reqwest::Method::POST, &url, Some(&req))
            .await?;

        debug!(
            "Sent private message from {} to {} via {}",
            from, to, remote_endpoint
        );
        Ok(())
    }

    /// 发送群聊消息
    pub async fn send_group(
        &self,
        remote_endpoint: &str,
        from: &str,
        group_id: &str,
        content: &str,
    ) -> Result<()> {
        let url = format!("{}/messages", remote_endpoint);

        let req = SendMessageRequest {
            from: from.to_string(),
            to: vec![group_id.to_string()],
            content: content.to_string(),
            msg_type: "group".to_string(),
        };

        let _: String = self
            .request(reqwest::Method::POST, &url, Some(&req))
            .await?;

        debug!("Sent group message to {} via {}", group_id, remote_endpoint);
        Ok(())
    }

    /// 发送广播消息
    pub async fn send_broadcast(
        &self,
        remote_endpoint: &str,
        from: &str,
        content: &str,
    ) -> Result<()> {
        let url = format!("{}/messages", remote_endpoint);

        let req = SendMessageRequest {
            from: from.to_string(),
            to: vec![],
            content: content.to_string(),
            msg_type: "broadcast".to_string(),
        };

        let _: String = self
            .request(reqwest::Method::POST, &url, Some(&req))
            .await?;

        debug!("Sent broadcast message via {}", remote_endpoint);
        Ok(())
    }

    /// 创建群聊
    pub async fn create_group(
        &self,
        remote_endpoint: &str,
        group_id: &str,
        name: &str,
        creator: &str,
        members: Vec<String>,
    ) -> Result<String> {
        let url = format!("{}/groups", remote_endpoint);

        let req = CreateGroupRequest {
            group_id: group_id.to_string(),
            name: name.to_string(),
            creator: creator.to_string(),
            members,
        };

        let group_id: String = self
            .request(reqwest::Method::POST, &url, Some(&req))
            .await?;

        info!("Created group {} at {}", group_id, remote_endpoint);
        Ok(group_id)
    }

    /// 获取群聊信息
    pub async fn get_group(
        &self,
        remote_endpoint: &str,
        group_id: &str,
    ) -> Result<Option<GroupInfo>> {
        let url = format!("{}/groups/{}", remote_endpoint, group_id);

        let group: Option<GroupInfo> = self
            .request(reqwest::Method::GET, &url, None::<&()>)
            .await?;

        Ok(group)
    }

    /// 邀请成员加入群聊
    pub async fn invite_to_group(
        &self,
        remote_endpoint: &str,
        group_id: &str,
        inviter: &str,
        invitee: &str,
    ) -> Result<()> {
        let url = format!("{}/groups/invite", remote_endpoint);

        let req = InviteMemberRequest {
            group_id: group_id.to_string(),
            inviter: inviter.to_string(),
            invitee: invitee.to_string(),
        };

        let _: String = self
            .request(reqwest::Method::POST, &url, Some(&req))
            .await?;

        info!(
            "Invited {} to group {} via {}",
            invitee, group_id, remote_endpoint
        );
        Ok(())
    }
}

/// Agent 网络管理器
///
/// 管理与其他 Agent 的连接
pub struct AgentNetwork {
    client: A2AClient,
    known_agents: dashmap::DashMap<String, AgentInfo>,
}

impl AgentNetwork {
    /// 创建新的 Agent 网络管理器
    pub fn new(local_endpoint: impl Into<String>) -> Self {
        Self {
            client: A2AClient::new(local_endpoint),
            known_agents: dashmap::DashMap::new(),
        }
    }

    /// 发现并连接到一个种子 Agent
    ///
    /// 从种子 Agent 获取所有已知 Agent 列表
    pub async fn connect_to_seed(&self, seed_endpoint: &str) -> Result<()> {
        // 健康检查
        if !self.client.health_check(seed_endpoint).await? {
            return Err(anyhow::anyhow!("Seed agent is not healthy"));
        }

        // 发现 Agent
        let agents = self.client.discover_agents(seed_endpoint).await?;

        for agent in agents {
            self.known_agents.insert(agent.id.clone(), agent);
        }

        info!(
            "Connected to seed {}, discovered {} agents",
            seed_endpoint,
            self.known_agents.len()
        );
        Ok(())
    }

    /// 向所有已知 Agent 广播自己的信息
    pub async fn announce_self(&self, agent_info: &AgentInfo) -> Result<()> {
        for entry in self.known_agents.iter() {
            let agent = entry.value();
            if let Err(e) = self
                .client
                .register_to_remote(&agent.endpoint, agent_info)
                .await
            {
                warn!("Failed to announce to {}: {}", agent.id, e);
            }
        }

        info!("Announced self to {} agents", self.known_agents.len());
        Ok(())
    }

    /// 获取已知 Agent
    pub fn get_agent(&self, id: &str) -> Option<AgentInfo> {
        self.known_agents.get(id).map(|a| a.clone())
    }

    /// 列出所有已知 Agent
    pub fn list_agents(&self) -> Vec<AgentInfo> {
        self.known_agents.iter().map(|a| a.clone()).collect()
    }

    /// 获取客户端
    pub fn client(&self) -> &A2AClient {
        &self.client
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_a2a_client_creation() {
        let client = A2AClient::new("http://localhost:8080");
        assert_eq!(client.local_endpoint, "http://localhost:8080");
    }

    #[test]
    fn test_agent_network_creation() {
        let network = AgentNetwork::new("http://localhost:8080");
        assert!(network.known_agents.is_empty());
    }
}
