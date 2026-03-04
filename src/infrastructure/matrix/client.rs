//! Matrix API 客户端
//!
//! 提供与 Matrix Homeserver 交互的底层 API

use anyhow::{Context, Result};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::{debug, error, info};
use uuid::Uuid;

use super::config::MatrixConfig;

/// Matrix 客户端
#[derive(Clone)]
pub struct MatrixClient {
    client: Client,
    config: MatrixConfig,
}

impl MatrixClient {
    /// 创建新的 Matrix 客户端
    pub fn new(config: &MatrixConfig) -> Self {
        let client = Client::builder()
            .user_agent("ImitatorT/0.1.0 (Matrix Appservice)")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            config: config.clone(),
        }
    }

    /// 获取完整的 API URL
    fn api_url(&self, path: &str) -> String {
        format!(
            "{}/{}?access_token={}",
            self.config.api_base_url(),
            path.trim_start_matches('/'),
            self.config.as_token
        )
    }

    /// 发送消息到房间
    ///
    /// # 参数
    /// * `room_id` - 房间 ID (如：!room123:localhost) 或房间别名 (如：#general:localhost)
    /// * `user_id` - 发送者虚拟用户 ID (如：@_ceo:localhost)
    /// * `content` - 消息内容
    /// * `msgtype` - 消息类型 (默认："m.text")
    ///
    /// # 返回
    /// 返回事件 ID
    pub async fn send_message(
        &self,
        room_id: &str,
        user_id: &str,
        content: &str,
        msgtype: &str,
    ) -> Result<String> {
        let txn_id = Uuid::new_v4().to_string();
        let event_type = "m.room.message";

        let body = json!({
            "msgtype": msgtype,
            "body": content,
        });

        let url = self.api_url(&format!(
            "rooms/{}/send/{}/{}",
            urlencoding::encode(room_id),
            event_type,
            txn_id
        ));

        debug!("Sending message to {} as {}", room_id, user_id);

        let response = self
            .client
            .put(&url)
            .header("Authorization", format!("Bearer {}", self.config.as_token))
            .query(&[("user_id", user_id)])
            .json(&body)
            .send()
            .await
            .context("Failed to send message request")?;

        if response.status().is_success() {
            let result: Value = response
                .json()
                .await
                .context("Failed to parse response")?;
            let event_id = result["event_id"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();
            info!("Message sent to {} as {}, event_id: {}", room_id, user_id, event_id);
            Ok(event_id)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Failed to send message: {} - {}", status, error_text);
            anyhow::bail!("Failed to send message: {} - {}", status, error_text);
        }
    }

    /// 发送文本消息
    pub async fn send_text_message(
        &self,
        room_id: &str,
        user_id: &str,
        content: &str,
    ) -> Result<String> {
        self.send_message(room_id, user_id, content, "m.text").await
    }

    /// 加入房间
    ///
    /// # 参数
    /// * `room_id_or_alias` - 房间 ID 或房间别名
    /// * `user_id` - 要加入的用户 ID
    pub async fn join_room(&self, room_id_or_alias: &str, user_id: &str) -> Result<String> {
        let url = self.api_url(&format!(
            "rooms/{}/join",
            urlencoding::encode(room_id_or_alias)
        ));

        debug!("Joining room {} as {}", room_id_or_alias, user_id);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.as_token))
            .query(&[("user_id", user_id)])
            .send()
            .await
            .context("Failed to join room request")?;

        if response.status().is_success() {
            let result: Value = response
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
            anyhow::bail!("Failed to join room: {} - {}", status, error_text);
        }
    }

    /// 创建房间
    ///
    /// # 参数
    /// * `user_id` - 创建者用户 ID
    /// * `alias` - 可选的房间别名 (本地部分，不含服务器名)
    /// * `name` - 可选的房间名称
    /// * `invite` - 可选的邀请用户列表
    pub async fn create_room(
        &self,
        user_id: &str,
        alias: Option<&str>,
        name: Option<&str>,
        invite: Option<&[String]>,
    ) -> Result<String> {
        let url = self.api_url("createRoom");

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
        if let Some(inv) = invite {
            body["invite"] = json!(inv);
        }

        debug!("Creating room as {} with alias {:?}", user_id, alias);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.as_token))
            .query(&[("user_id", user_id)])
            .json(&body)
            .send()
            .await
            .context("Failed to create room request")?;

        if response.status().is_success() {
            let result: Value = response
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
                let room_alias = alias.map(|a| self.config.generate_room_alias(a)).unwrap_or_default();
                info!("Room already exists, trying to join: {}", room_alias);
                return self.join_room(&room_alias, user_id).await;
            }
            error!("Failed to create room: {} - {}", status, error_text);
            anyhow::bail!("Failed to create room: {} - {}", status, error_text);
        }
    }

    /// 获取房间 ID（通过别名）
    pub async fn get_room_id(&self, room_alias: &str) -> Result<String> {
        let url = self.api_url(&format!(
            "directory/room/{}",
            urlencoding::encode(room_alias)
        ));

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.as_token))
            .send()
            .await
            .context("Failed to get room ID")?;

        if response.status().is_success() {
            let result: Value = response
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
            anyhow::bail!("Failed to get room ID: {} - {}", status, error_text);
        }
    }

    /// 获取用户信息
    #[allow(dead_code)]
    pub async fn get_user_info(&self, user_id: &str) -> Result<UserInfo> {
        let url = self.api_url(&format!(
            "profile/{}",
            urlencoding::encode(user_id)
        ));

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.as_token))
            .send()
            .await
            .context("Failed to get user info")?;

        if response.status().is_success() {
            let result: Value = response
                .json()
                .await
                .context("Failed to parse response")?;
            Ok(UserInfo {
                displayname: result["displayname"].as_str().map(String::from),
                avatar_url: result["avatar_url"].as_str().map(String::from),
            })
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Failed to get user info: {} - {}", status, error_text);
            anyhow::bail!("Failed to get user info: {} - {}", status, error_text);
        }
    }

    /// 设置用户显示名称
    pub async fn set_user_displayname(
        &self,
        user_id: &str,
        displayname: &str,
    ) -> Result<()> {
        let url = self.api_url(&format!(
            "profile/{}/displayname",
            urlencoding::encode(user_id)
        ));

        let body = json!({
            "displayname": displayname,
        });

        let response = self
            .client
            .put(&url)
            .header("Authorization", format!("Bearer {}", self.config.as_token))
            .query(&[("user_id", user_id)])
            .json(&body)
            .send()
            .await
            .context("Failed to set displayname")?;

        if response.status().is_success() {
            info!("Set displayname '{}' for {}", displayname, user_id);
            Ok(())
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Failed to set displayname: {} - {}", status, error_text);
            anyhow::bail!("Failed to set displayname: {} - {}", status, error_text);
        }
    }

    /// 检查用户是否存在
    pub async fn user_exists(&self, user_id: &str) -> Result<bool> {
        // 通过获取用户信息来检查是否存在
        match self.get_user_info(user_id).await {
            Ok(_) => Ok(true),
            Err(e) => {
                // 如果是 404 错误，说明用户不存在
                if e.to_string().contains("404") {
                    Ok(false)
                } else {
                    Err(e)
                }
            }
        }
    }

    /// 注册虚拟用户
    ///
    /// 注意：这需要 Homeserver 配置允许 Appservice 注册用户
    pub async fn register_user(&self, localpart: &str) -> Result<String> {
        let user_id = self.config.generate_user_id(localpart);
        let _url = self.api_url(&format!(
            "admin/whois/{}",
            urlencoding::encode(&user_id)
        ));

        // 先检查用户是否已存在
        if self.user_exists(&user_id).await? {
            info!("User {} already exists", user_id);
            return Ok(user_id);
        }

        // 尝试通过注册端点注册
        // 注意：Conduit 和其他 Homeserver 可能有不同的注册方式
        // 对于 Appservice 管理的用户，通常由 Homeserver 自动处理
        info!("Virtual user {} will be auto-created by homeserver when first used", user_id);
        Ok(user_id)
    }

    /// 列出所有房间
    #[allow(dead_code)]
    pub async fn list_rooms(&self, user_id: &str) -> Result<Vec<RoomInfo>> {
        let url = self.api_url("joined_rooms");

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.as_token))
            .query(&[("user_id", user_id)])
            .send()
            .await
            .context("Failed to list rooms")?;

        if response.status().is_success() {
            let result: Value = response
                .json()
                .await
                .context("Failed to parse response")?;
            let rooms: Vec<RoomInfo> = result["joined_rooms"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|r| {
                    Some(RoomInfo {
                        room_id: r["room_id"].as_str()?.to_string(),
                    })
                })
                .collect();
            Ok(rooms)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Failed to list rooms: {} - {}", status, error_text);
            anyhow::bail!("Failed to list rooms: {} - {}", status, error_text);
        }
    }

    /// 发送打字通知
    #[allow(dead_code)]
    pub async fn send_typing(
        &self,
        room_id: &str,
        user_id: &str,
        typing: bool,
    ) -> Result<()> {
        let url = self.api_url(&format!(
            "rooms/{}/typing/{}",
            urlencoding::encode(room_id),
            urlencoding::encode(user_id)
        ));

        let body = json!({
            "typing": typing,
        });

        let response = self
            .client
            .put(&url)
            .header("Authorization", format!("Bearer {}", self.config.as_token))
            .query(&[("user_id", user_id)])
            .json(&body)
            .send()
            .await
            .context("Failed to send typing notification")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Failed to send typing: {} - {}", status, error_text);
        }

        Ok(())
    }
}

/// 用户信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub displayname: Option<String>,
    pub avatar_url: Option<String>,
}

/// 房间信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomInfo {
    pub room_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_url() {
        let config = MatrixConfig {
            homeserver_url: "http://localhost:5141".to_string(),
            server_name: "localhost".to_string(),
            as_token: "test_token".to_string(),
            hs_token: "test_hs".to_string(),
            sender_localpart: "_bot".to_string(),
            appservice_port: 9000,
        };

        let client = MatrixClient::new(&config);
        let url = client.api_url("rooms/!room123:localhost/send/m.room.message/txn123");

        assert!(url.contains("http://localhost:5141/_matrix/client/v3/rooms"));
        assert!(url.contains("access_token=test_token"));
    }
}
