//! Canonical request payloads for the Microsoft 365 BASIC keywords.
//!
//! `GET SHAREPOINT LISTS`, `CREATE SHAREPOINT ITEM` and `SEND TEAMS MESSAGE`
//! only bind arguments; the Graph endpoint each command addresses — and the
//! body it carries — belongs to this crate (#1506).

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// What a Microsoft 365 request asks for. Serializes to the `kind` field the
/// runtime routes on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum M365RequestKind {
    SharePointLists,
    SharePointItem,
    TeamsMessage,
}

/// A Microsoft 365 command ready to be dispatched over Microsoft Graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct M365Request {
    pub kind: M365RequestKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,
    pub endpoint: String,
    pub method: &'static str,
}

impl M365Request {
    fn empty(kind: M365RequestKind, endpoint: String, method: &'static str) -> Self {
        Self {
            kind,
            action: None,
            site_id: None,
            list_id: None,
            fields: None,
            chat_id: None,
            body: None,
            endpoint,
            method,
        }
    }

    /// `GET SHAREPOINT LISTS <site>` — list collections of a SharePoint site.
    pub fn sharepoint_lists(site_id: &str) -> Self {
        Self::empty(
            M365RequestKind::SharePointLists,
            format!("/v1.0/sites/{site_id}/lists"),
            "GET",
        )
        .with_site(site_id)
    }

    /// `CREATE SHAREPOINT ITEM <site>, <list>, <fields>` — create a list item.
    pub fn sharepoint_item(site_id: &str, list_id: &str, fields: &str) -> Self {
        let mut request = Self::empty(
            M365RequestKind::SharePointItem,
            format!("/v1.0/sites/{site_id}/lists/{list_id}/items"),
            "POST",
        );
        request.action = Some("create");
        request.site_id = Some(site_id.to_string());
        request.list_id = Some(list_id.to_string());
        request.fields = Some(fields.to_string());
        request
    }

    /// `SEND TEAMS MESSAGE <chat>, <message>` — post an HTML message to a chat.
    pub fn teams_message(chat_id: &str, message: &str) -> Self {
        let mut request = Self::empty(
            M365RequestKind::TeamsMessage,
            format!("/v1.0/chats/{chat_id}/messages"),
            "POST",
        );
        request.action = Some("send");
        request.chat_id = Some(chat_id.to_string());
        request.body = Some(json!({ "contentType": "html", "content": message }));
        request
    }

    fn with_site(mut self, site_id: &str) -> Self {
        self.site_id = Some(site_id.to_string());
        self
    }

    /// JSON string handed back to the BASIC runtime.
    pub fn to_json_string(&self) -> String {
        serde_json::to_value(self)
            .map(|value| value.to_string())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(request: &M365Request) -> Value {
        serde_json::to_value(request).expect("serialize")
    }

    #[test]
    fn sharepoint_lists_builds_get_endpoint() {
        let json = value(&M365Request::sharepoint_lists("contoso"));
        assert_eq!(json["kind"], "sharepoint_lists");
        assert_eq!(json["site_id"], "contoso");
        assert_eq!(json["endpoint"], "/v1.0/sites/contoso/lists");
        assert_eq!(json["method"], "GET");
        assert!(json.get("action").is_none());
    }

    #[test]
    fn sharepoint_item_targets_the_list_collection() {
        let json = value(&M365Request::sharepoint_item(
            "s1",
            "l1",
            "{\"Title\":\"x\"}",
        ));
        assert_eq!(json["kind"], "sharepoint_item");
        assert_eq!(json["action"], "create");
        assert_eq!(json["list_id"], "l1");
        assert_eq!(json["fields"], "{\"Title\":\"x\"}");
        assert_eq!(json["endpoint"], "/v1.0/sites/s1/lists/l1/items");
        assert_eq!(json["method"], "POST");
    }

    #[test]
    fn teams_message_carries_html_body() {
        let json = value(&M365Request::teams_message("c1", "hello"));
        assert_eq!(json["kind"], "teams_message");
        assert_eq!(json["action"], "send");
        assert_eq!(json["chat_id"], "c1");
        assert_eq!(json["body"]["contentType"], "html");
        assert_eq!(json["body"]["content"], "hello");
        assert_eq!(json["endpoint"], "/v1.0/chats/c1/messages");
    }
}
