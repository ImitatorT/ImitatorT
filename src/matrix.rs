use anyhow::{Context, Result};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct MatrixClient {
    homeserver: String,
    token: String,
    http: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct MessagesResponse {
    chunk: Vec<TimelineEvent>,
}

#[derive(Debug, Deserialize)]
struct TimelineEvent {
    sender: Option<String>,
    content: Option<EventContent>,
}

#[derive(Debug, Deserialize)]
struct EventContent {
    body: Option<String>,
    msgtype: Option<String>,
}

impl MatrixClient {
    pub fn new(homeserver: String, token: String) -> Self {
        Self {
            homeserver,
            token,
            http: reqwest::Client::new(),
        }
    }

    pub async fn latest_context(&self, room_id: &str, limit: usize) -> Result<String> {
        let room = urlencoding::encode(room_id);
        let url = format!(
            "{}/_matrix/client/v3/rooms/{}/messages?dir=b&limit={}",
            self.homeserver, room, limit
        );

        let res = self
            .http
            .get(url)
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .header(CONTENT_TYPE, "application/json")
            .send()
            .await
            .context("failed to query matrix messages")?
            .error_for_status()
            .context("matrix returned non-success status")?;

        let body: MessagesResponse = res
            .json()
            .await
            .context("failed to deserialize matrix messages")?;

        let lines = body
            .chunk
            .iter()
            .filter_map(|ev| {
                let content = ev.content.as_ref()?;
                if content.msgtype.as_deref() != Some("m.text") {
                    return None;
                }
                Some(format!(
                    "{}: {}",
                    ev.sender.clone().unwrap_or_else(|| "unknown".to_string()),
                    content.body.clone().unwrap_or_default()
                ))
            })
            .collect::<Vec<_>>();

        Ok(lines.join("\n"))
    }

    pub async fn send_text_message(&self, room_id: &str, message: &str) -> Result<()> {
        let room = urlencoding::encode(room_id);
        let txn_id = uuid::Uuid::new_v4().to_string();
        let url = format!(
            "{}/_matrix/client/v3/rooms/{}/send/m.room.message/{}",
            self.homeserver, room, txn_id
        );

        let payload = serde_json::json!({
            "msgtype": "m.text",
            "body": message,
        });

        self.http
            .put(url)
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .header(CONTENT_TYPE, "application/json")
            .json(&payload)
            .send()
            .await
            .context("failed to send message")?
            .error_for_status()
            .context("matrix returned non-success status when sending")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix_client_new() {
        let client = MatrixClient::new(
            "http://localhost:8008".to_string(),
            "test_token".to_string(),
        );
        assert_eq!(client.homeserver, "http://localhost:8008");
        assert_eq!(client.token, "test_token");
    }

    #[test]
    fn test_messages_response_deserialization() {
        let json_str = r#"{
            "chunk": [
                {
                    "sender": "@user:example.com",
                    "content": {
                        "body": "Hello world",
                        "msgtype": "m.text"
                    }
                },
                {
                    "sender": "@bot:example.com",
                    "content": {
                        "body": "Hi there",
                        "msgtype": "m.text"
                    }
                }
            ]
        }"#;
        
        let resp: MessagesResponse = serde_json::from_str(json_str).unwrap();
        assert_eq!(resp.chunk.len(), 2);
        assert_eq!(resp.chunk[0].sender, Some("@user:example.com".to_string()));
        assert_eq!(resp.chunk[0].content.as_ref().unwrap().body, Some("Hello world".to_string()));
        assert_eq!(resp.chunk[0].content.as_ref().unwrap().msgtype, Some("m.text".to_string()));
    }

    #[test]
    fn test_messages_response_with_non_text() {
        let json_str = r#"{
            "chunk": [
                {
                    "sender": "@user:example.com",
                    "content": {
                        "body": "image.png",
                        "msgtype": "m.image"
                    }
                }
            ]
        }"#;
        
        let resp: MessagesResponse = serde_json::from_str(json_str).unwrap();
        assert_eq!(resp.chunk.len(), 1);
        // Non-text messages should be filtered out in latest_context
        assert_eq!(resp.chunk[0].content.as_ref().unwrap().msgtype, Some("m.image".to_string()));
    }

    #[test]
    fn test_messages_response_empty() {
        let json_str = r#"{"chunk": []}"#;
        let resp: MessagesResponse = serde_json::from_str(json_str).unwrap();
        assert!(resp.chunk.is_empty());
    }

    #[test]
    fn test_event_content_deserialization() {
        let json_str = r#"{"body": "test message", "msgtype": "m.text"}"#;
        let content: EventContent = serde_json::from_str(json_str).unwrap();
        assert_eq!(content.body, Some("test message".to_string()));
        assert_eq!(content.msgtype, Some("m.text".to_string()));
    }

    #[test]
    fn test_timeline_event_deserialization() {
        let json_str = r#"{
            "sender": "@test:matrix.org",
            "content": {
                "body": "Hello",
                "msgtype": "m.text"
            }
        }"#;
        let event: TimelineEvent = serde_json::from_str(json_str).unwrap();
        assert_eq!(event.sender, Some("@test:matrix.org".to_string()));
        assert!(event.content.is_some());
    }

    #[test]
    fn test_timeline_event_missing_sender() {
        let json_str = r#"{
            "content": {
                "body": "Hello",
                "msgtype": "m.text"
            }
        }"#;
        let event: TimelineEvent = serde_json::from_str(json_str).unwrap();
        assert_eq!(event.sender, None);
        assert!(event.content.is_some());
    }

    #[test]
    fn test_context_formatting() {
        // Test that context is formatted correctly from events
        let events = vec![
            TimelineEvent {
                sender: Some("@alice:example.com".to_string()),
                content: Some(EventContent {
                    body: Some("Hello".to_string()),
                    msgtype: Some("m.text".to_string()),
                }),
            },
            TimelineEvent {
                sender: Some("@bob:example.com".to_string()),
                content: Some(EventContent {
                    body: Some("Hi there".to_string()),
                    msgtype: Some("m.text".to_string()),
                }),
            },
        ];

        let lines: Vec<String> = events
            .iter()
            .filter_map(|ev| {
                let content = ev.content.as_ref()?;
                if content.msgtype.as_deref() != Some("m.text") {
                    return None;
                }
                Some(format!(
                    "{}: {}",
                    ev.sender.clone().unwrap_or_else(|| "unknown".to_string()),
                    content.body.clone().unwrap_or_default()
                ))
            })
            .collect();

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "@alice:example.com: Hello");
        assert_eq!(lines[1], "@bob:example.com: Hi there");
    }

    #[test]
    fn test_context_filter_non_text() {
        // Test that non-text messages are filtered out
        let events = vec![
            TimelineEvent {
                sender: Some("@alice:example.com".to_string()),
                content: Some(EventContent {
                    body: Some("Hello".to_string()),
                    msgtype: Some("m.text".to_string()),
                }),
            },
            TimelineEvent {
                sender: Some("@bob:example.com".to_string()),
                content: Some(EventContent {
                    body: Some("image.png".to_string()),
                    msgtype: Some("m.image".to_string()),
                }),
            },
        ];

        let lines: Vec<String> = events
            .iter()
            .filter_map(|ev| {
                let content = ev.content.as_ref()?;
                if content.msgtype.as_deref() != Some("m.text") {
                    return None;
                }
                Some(format!(
                    "{}: {}",
                    ev.sender.clone().unwrap_or_else(|| "unknown".to_string()),
                    content.body.clone().unwrap_or_default()
                ))
            })
            .collect();

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "@alice:example.com: Hello");
    }
}
