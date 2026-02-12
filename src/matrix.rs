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
