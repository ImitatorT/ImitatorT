//! Matrix 事件处理
//!
//! 解析和处理来自 Homeserver 的事件

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, warn};

/// Matrix 事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MatrixEvent {
    #[serde(rename = "m.room.message")]
    RoomMessage(RoomMessageEvent),
    #[serde(rename = "m.room.member")]
    RoomMember(RoomMemberEvent),
    #[serde(rename = "m.room.create")]
    RoomCreate(Value),
    #[serde(rename = "m.room.join_rules")]
    RoomJoinRules(Value),
    #[serde(rename = "m.room.power_levels")]
    RoomPowerLevels(Value),
    #[serde(rename = "m.room.name")]
    RoomName(Value),
    #[serde(rename = "m.room.topic")]
    RoomTopic(Value),
    #[serde(rename = "m.room.avatar")]
    RoomAvatar(Value),
    #[serde(rename = "m.typing")]
    Typing(Value),
    #[serde(rename = "m.presence")]
    Presence(Value),
    #[serde(rename = "m.receipt")]
    Receipt(Value),
    #[serde(rename = "m.tag")]
    Tag(Value),
    #[serde(rename = "m.direct")]
    Direct(Value),
    #[serde(rename = "m.room.encrypted")]
    RoomEncrypted(Value),
    #[serde(untagged)]
    Unknown(Value),
}

/// 房间消息事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomMessageEvent {
    pub content: MessageContent,
    pub room_id: String,
    pub sender: String,
    pub event_id: String,
    pub origin_server_ts: u64,
    pub unsigned: Option<MessageUnsigned>,
}

/// 消息内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageContent {
    #[serde(rename = "msgtype")]
    pub msgtype: String,
    pub body: String,
    #[serde(default)]
    pub mentions: Option<Mentions>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub formatted_body: Option<String>,
}

/// 提及信息
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Mentions {
    #[serde(default)]
    pub user_ids: Vec<String>,
    #[serde(default)]
    pub room: bool,
}

/// 消息 unsigned 字段
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessageUnsigned {
    pub age: Option<u64>,
    pub transaction_id: Option<String>,
    pub membership: Option<String>,
    pub prev_content: Option<Value>,
}

/// 房间成员事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomMemberEvent {
    pub content: MemberContent,
    pub room_id: String,
    pub sender: String,
    pub state_key: String,
    pub event_id: Option<String>,
    pub origin_server_ts: Option<u64>,
}

/// 成员内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberContent {
    pub membership: String,
    pub displayname: Option<String>,
    pub avatar_url: Option<String>,
    pub reason: Option<String>,
}

impl MatrixEvent {
    /// 从原始 JSON 解析事件
    pub fn from_raw(event: &Value) -> Self {
        let event_type = event["type"].as_str().unwrap_or("");

        match event_type {
            "m.room.message" => {
                match serde_json::from_value(event.clone()) {
                    Ok(e) => MatrixEvent::RoomMessage(e),
                    Err(err) => {
                        warn!("Failed to parse room message event: {}", err);
                        MatrixEvent::Unknown(event.clone())
                    }
                }
            }
            "m.room.member" => {
                match serde_json::from_value(event.clone()) {
                    Ok(e) => MatrixEvent::RoomMember(e),
                    Err(err) => {
                        warn!("Failed to parse room member event: {}", err);
                        MatrixEvent::Unknown(event.clone())
                    }
                }
            }
            _ => MatrixEvent::Unknown(event.clone()),
        }
    }

    /// 获取事件类型
    pub fn event_type(&self) -> &str {
        match self {
            MatrixEvent::RoomMessage(_) => "m.room.message",
            MatrixEvent::RoomMember(_) => "m.room.member",
            MatrixEvent::RoomCreate(_) => "m.room.create",
            MatrixEvent::RoomJoinRules(_) => "m.room.join_rules",
            MatrixEvent::RoomPowerLevels(_) => "m.room.power_levels",
            MatrixEvent::RoomName(_) => "m.room.name",
            MatrixEvent::RoomTopic(_) => "m.room.topic",
            MatrixEvent::RoomAvatar(_) => "m.room.avatar",
            MatrixEvent::Typing(_) => "m.typing",
            MatrixEvent::Presence(_) => "m.presence",
            MatrixEvent::Receipt(_) => "m.receipt",
            MatrixEvent::Tag(_) => "m.tag",
            MatrixEvent::Direct(_) => "m.direct",
            MatrixEvent::RoomEncrypted(_) => "m.room.encrypted",
            MatrixEvent::Unknown(_) => "unknown",
        }
    }

    /// 获取房间 ID
    pub fn room_id(&self) -> Option<&str> {
        match self {
            MatrixEvent::RoomMessage(e) => Some(&e.room_id),
            MatrixEvent::RoomMember(e) => Some(&e.room_id),
            _ => None,
        }
    }

    /// 获取发送者
    pub fn sender(&self) -> Option<&str> {
        match self {
            MatrixEvent::RoomMessage(e) => Some(&e.sender),
            MatrixEvent::RoomMember(e) => Some(&e.sender),
            _ => None,
        }
    }

    /// 获取消息体（如果是消息事件）
    pub fn message_body(&self) -> Option<&str> {
        match self {
            MatrixEvent::RoomMessage(e) => Some(&e.content.body),
            _ => None,
        }
    }

    /// 检查是否是文本消息
    pub fn is_text_message(&self) -> bool {
        match self {
            MatrixEvent::RoomMessage(e) => {
                e.content.msgtype == "m.text" || e.content.msgtype == "m.notice"
            }
            _ => false,
        }
    }

    /// 检查是否提及了某个用户
    pub fn mentions_user(&self, user_id: &str) -> bool {
        match self {
            MatrixEvent::RoomMessage(e) => {
                // 检查内容中的提及
                if let Some(ref mentions) = e.content.mentions {
                    if mentions.user_ids.contains(&user_id.to_string()) {
                        return true;
                    }
                }

                // 检查 body 中的 @mention
                e.content.body.contains(&format!("@{}", user_id))
            }
            _ => false,
        }
    }

    /// 检查是否是机器人自己的消息
    pub fn is_from_virtual_user(&self) -> bool {
        if let Some(sender) = self.sender() {
            return sender.starts_with("@_");
        }
        false
    }
}

/// 事件过滤器
pub struct EventFilter {
    /// 是否允许来自虚拟用户的消息
    pub allow_virtual_users: bool,
    /// 是否允许来自机器人的消息
    pub allow_bot_messages: bool,
    /// 只处理提及自己的消息
    pub only_mentions: bool,
    /// 自己的用户 ID
    pub self_user_id: Option<String>,
}

impl Default for EventFilter {
    fn default() -> Self {
        Self {
            allow_virtual_users: false,
            allow_bot_messages: true,
            only_mentions: false,
            self_user_id: None,
        }
    }
}

impl EventFilter {
    /// 检查事件是否应该被处理
    pub fn should_process(&self, event: &MatrixEvent) -> bool {
        // 只处理房间消息事件
        if !event.is_text_message() {
            debug!("Ignoring non-text message event");
            return false;
        }

        // 检查是否来自虚拟用户
        if event.is_from_virtual_user() && !self.allow_virtual_users {
            debug!("Ignoring message from virtual user");
            return false;
        }

        // 检查是否只处理提及自己的消息
        if self.only_mentions {
            if let Some(ref self_id) = self.self_user_id {
                if !event.mentions_user(self_id) {
                    debug!("Ignoring message that doesn't mention self");
                    return false;
                }
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_room_message() {
        let event_json = json!({
            "type": "m.room.message",
            "content": {
                "msgtype": "m.text",
                "body": "Hello, World!"
            },
            "room_id": "!room123:localhost",
            "sender": "@user:localhost",
            "event_id": "$event123:localhost",
            "origin_server_ts": 1234567890
        });

        let event = MatrixEvent::from_raw(&event_json);

        assert!(event.is_text_message());
        assert_eq!(event.message_body(), Some("Hello, World!"));
        assert_eq!(event.room_id(), Some("!room123:localhost"));
        assert_eq!(event.sender(), Some("@user:localhost"));
    }

    #[test]
    fn test_virtual_user_detection() {
        let event_json = json!({
            "type": "m.room.message",
            "content": {
                "msgtype": "m.text",
                "body": "Hello"
            },
            "room_id": "!room123:localhost",
            "sender": "@_ceo:localhost",
            "event_id": "$event123:localhost",
            "origin_server_ts": 1234567890
        });

        let event = MatrixEvent::from_raw(&event_json);
        assert!(event.is_from_virtual_user());
    }

    #[test]
    fn test_filter() {
        let filter = EventFilter {
            allow_virtual_users: false,
            ..Default::default()
        };

        let virtual_user_event = MatrixEvent::from_raw(&json!({
            "type": "m.room.message",
            "content": {"msgtype": "m.text", "body": "Hello"},
            "room_id": "!room:localhost",
            "sender": "@_ceo:localhost",
            "event_id": "$event:localhost",
            "origin_server_ts": 1234567890
        }));

        assert!(!filter.should_process(&virtual_user_event));

        let human_event = MatrixEvent::from_raw(&json!({
            "type": "m.room.message",
            "content": {"msgtype": "m.text", "body": "Hello"},
            "room_id": "!room:localhost",
            "sender": "@human:localhost",
            "event_id": "$event:localhost",
            "origin_server_ts": 1234567890
        }));

        assert!(filter.should_process(&human_event));
    }
}
