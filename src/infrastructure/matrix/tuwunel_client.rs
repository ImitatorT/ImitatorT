//! Tuwunel Matrix 客户端
//!
//! 通过 Appservice API 注册虚拟用户到 Tuwunel Homeserver

use anyhow::{Context, Result};
use reqwest::{Client, StatusCode};
use serde_json::json;
use tracing::{debug, error, info, warn};
use urlencoding;

use super::config::MatrixConfig;

/// Tuwunel Matrix 客户端（通过 Appservice API 注册虚拟用户）
#[derive(Clone)]
pub struct TuwunelClient {
    client: Client,
    config: MatrixConfig,
}

impl TuwunelClient {
    /// 创建新的 Tuwunel 客户端
    pub fn new(config: &MatrixConfig) -> Self {
        Self {
            client: Client::new(),
            config: config.clone(),
        }
    }

    /// 注册虚拟用户到 Tuwunel
    ///
    /// 通过 Appservice API 注册，homeserver 会自动创建虚拟用户
    ///
    /// # 参数
    /// * `localpart` - 用户名本地部分（不含 @ 和服务器名）
    /// * `displayname` - 用户显示名称
    ///
    /// # 返回
    /// 返回完整的用户 ID
    pub async fn register_virtual_user(&self, localpart: &str, displayname: &str) -> Result<String> {
        let user_id = self.config.generate_user_id(localpart);

        // 检查用户是否已存在
        if self.user_exists(&user_id).await? {
            debug!("Virtual user {} already exists", user_id);
            return Ok(user_id);
        }

        // 通过 Appservice API 注册虚拟用户
        // Tuwunel/Conduit 风格的注册端点
        let url = format!(
            "{}/_matrix/client/v3/register?access_token={}",
            self.config.api_base_url(),
            self.config.as_token
        );

        let body = json!({
            "username": localpart,
            "type": "m.login.application_service",
        });

        debug!("Registering virtual user {} with type m.login.application_service", user_id);

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to send register request")?;

        if response.status().is_success() {
            info!("Registered virtual user: {}", user_id);

            // 设置显示名称
            if let Err(e) = self.set_displayname(&user_id, displayname).await {
                warn!("Failed to set displayname for {}: {}", user_id, e);
            }

            Ok(user_id)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();

            // 用户可能已存在，返回成功
            if status == StatusCode::BAD_REQUEST
                && (error_text.contains("already exists") || error_text.contains("M_USER_IN_USE"))
            {
                debug!("User {} already exists (confirmed)", user_id);
                return Ok(user_id);
            }

            error!("Failed to register user {}: {} - {}", user_id, status, error_text);
            anyhow::bail!("Failed to register user {}: {}", user_id, status);
        }
    }

    /// 设置用户显示名称
    pub async fn set_displayname(&self, user_id: &str, displayname: &str) -> Result<()> {
        let url = format!(
            "{}/_matrix/client/v3/profile/{}/displayname?access_token={}",
            self.config.api_base_url(),
            urlencoding::encode(user_id),
            self.config.as_token
        );

        let body = json!({
            "displayname": displayname,
        });

        let response = self
            .client
            .put(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to send displayname request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            warn!("Failed to set displayname: {} - {}", status, error_text);
        }

        Ok(())
    }

    /// 检查用户是否存在
    pub async fn user_exists(&self, user_id: &str) -> Result<bool> {
        let url = format!(
            "{}/_matrix/client/v3/profile/{}?access_token={}",
            self.config.api_base_url(),
            urlencoding::encode(user_id),
            self.config.as_token
        );

        let response = self.client.get(&url).send().await?;
        Ok(response.status().is_success())
    }

    /// 邀请用户加入房间
    pub async fn invite_user(&self, room_id: &str, user_id: &str) -> Result<()> {
        let url = format!(
            "{}/_matrix/client/v3/rooms/{}/invite?access_token={}",
            self.config.api_base_url(),
            urlencoding::encode(room_id),
            self.config.as_token
        );

        let body = json!({
            "user_id": user_id,
        });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to send invite request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();

            // 如果用户已经在房间中，返回成功
            if status == StatusCode::BAD_REQUEST
                && (error_text.contains("already joined") || error_text.contains("M_FORBIDDEN"))
            {
                debug!("User {} already in room {}", user_id, room_id);
                return Ok(());
            }

            warn!("Failed to invite user {} to room {}: {} - {}", user_id, room_id, status, error_text);
        }

        Ok(())
    }

    /// 创建房间
    ///
    /// # 参数
    /// * `user_id` - 创建者用户 ID
    /// * `alias` - 可选的房间别名（本地部分）
    /// * `name` - 可选的房间名称
    pub async fn create_room(
        &self,
        user_id: &str,
        alias: Option<&str>,
        name: Option<&str>,
    ) -> Result<String> {
        let url = format!(
            "{}/_matrix/client/v3/createRoom?access_token={}",
            self.config.api_base_url(),
            self.config.as_token
        );

        let mut body = json!({
            "visibility": "private",
            "preset": "private_chat",
        });

        if let Some(a) = alias {
            body["room_alias_name"] = json!(a.trim_start_matches('#').split(':').next().unwrap_or(a));
        }
        if let Some(n) = name {
            body["name"] = json!(n);
        }

        debug!("Creating room as {} with alias {:?}", user_id, alias);

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to send create room request")?;

        if response.status().is_success() {
            let result: serde_json::Value = response
                .json()
                .await
                .context("Failed to parse response")?;
            let room_id = result["room_id"]
                .as_str()
                .context("No room_id in response")?
                .to_string();
            info!("Created room {} as {}", room_id, user_id);
            Ok(room_id)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();

            // 如果房间已存在，尝试加入
            if status == StatusCode::BAD_REQUEST
                && error_text.contains("already exists")
            {
                let room_alias = alias
                    .map(|a| self.config.generate_room_alias(a))
                    .unwrap_or_default();
                info!("Room already exists, trying to join: {}", room_alias);
                return self.join_room(&room_alias, user_id).await;
            }

            error!("Failed to create room: {} - {}", status, error_text);
            anyhow::bail!("Failed to create room: {}", status);
        }
    }

    /// 加入房间
    pub async fn join_room(&self, room_id_or_alias: &str, user_id: &str) -> Result<String> {
        let url = format!(
            "{}/_matrix/client/v3/rooms/{}/join?access_token={}",
            self.config.api_base_url(),
            urlencoding::encode(room_id_or_alias),
            self.config.as_token
        );

        let response = self
            .client
            .post(&url)
            .send()
            .await
            .context("Failed to send join room request")?;

        if response.status().is_success() {
            let result: serde_json::Value = response
                .json()
                .await
                .context("Failed to parse response")?;
            let joined_room_id = result["room_id"]
                .as_str()
                .unwrap_or(room_id_or_alias)
                .to_string();
            info!("Joined room {} as {}", room_id_or_alias, user_id);
            Ok(joined_room_id)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();

            // 如果已经加入，返回成功
            if status == StatusCode::BAD_REQUEST
                && error_text.contains("already joined")
            {
                info!("Already in room {}", room_id_or_alias);
                return Ok(room_id_or_alias.to_string());
            }

            error!("Failed to join room: {} - {}", status, error_text);
            anyhow::bail!("Failed to join room: {}", status);
        }
    }

    /// 获取房间 ID（通过别名）
    pub async fn get_room_id(&self, room_alias: &str) -> Result<String> {
        let url = format!(
            "{}/_matrix/client/v3/directory/room/{}?access_token={}",
            self.config.api_base_url(),
            urlencoding::encode(room_alias),
            self.config.as_token
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to get room ID")?;

        if response.status().is_success() {
            let result: serde_json::Value = response
                .json()
                .await
                .context("Failed to parse response")?;
            let room_id = result["room_id"]
                .as_str()
                .context("No room_id in response")?
                .to_string();
            Ok(room_id)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Failed to get room ID: {} - {}", status, error_text);
            anyhow::bail!("Failed to get room ID: {}", status);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_user_id() {
        let config = MatrixConfig {
            homeserver_url: "http://localhost:6167".to_string(),
            server_name: "matrix.zhengui.cc.cd".to_string(),
            as_token: "test_token".to_string(),
            hs_token: "test_hs".to_string(),
            sender_localpart: "_bot".to_string(),
            appservice_port: 9000,
        };

        let client = TuwunelClient::new(&config);
        let user_id = client.config.generate_user_id("agent_123");

        assert_eq!(user_id, "@_agent_123:matrix.zhengui.cc.cd");
    }
}