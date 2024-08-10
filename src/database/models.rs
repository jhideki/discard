use rusqlite;

use crate::utils::types::NodeId;

pub trait ToSqlStatement {
    fn to_sql(&self) -> Vec<(&str, String)>;
    fn table_name() -> &'static str;
}

pub trait FromRow {
    type Model;
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self::Model>;
    fn table_name() -> &'static str;
}

#[derive(Debug, PartialEq, Eq)]
pub struct User {
    pub user_id: i32,
    pub display_name: String,
    pub node_id: String,
    pub is_online: bool,
}

impl FromRow for User {
    type Model = User;
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<User> {
        Ok(Self {
            user_id: row.get("user_id")?,
            display_name: row.get("display_name")?,
            node_id: row.get("node_id")?,
            is_online: row.get("is_online")?,
        })
    }
    fn table_name() -> &'static str {
        "users"
    }
}

impl ToSqlStatement for User {
    fn to_sql(&self) -> Vec<(&str, String)> {
        vec![
            ("display_name", self.display_name.clone()),
            ("node_id", self.node_id.to_string()),
            ("is_online", self.is_online.to_string()),
        ]
    }
    fn table_name() -> &'static str {
        "users"
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Message {
    pub message_id: i32,
    pub content: String,
    pub sender_id: i32,
    pub received_ts: Option<String>,
    pub sent_ts: Option<String>,
    pub read_ts: Option<String>,
}

impl FromRow for Message {
    type Model = Message;
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Message> {
        Ok(Self {
            message_id: row.get("message_id")?,
            content: row.get("content")?,
            sender_id: row.get("sender_id")?,
            received_ts: row.get("received_ts")?,
            sent_ts: row.get("sent_ts")?,
            read_ts: row.get("read_ts")?,
        })
    }
    fn table_name() -> &'static str {
        "messages"
    }
}

impl ToSqlStatement for Message {
    fn to_sql(&self) -> Vec<(&str, String)> {
        vec![
            ("sender_id", self.sender_id.to_string()),
            ("content", self.content.clone()),
            ("received_ts", self.sender_id.to_string()),
            ("sent_ts", self.sender_id.to_string()),
            ("read_ts", self.sender_id.to_string()),
        ]
    }
    fn table_name() -> &'static str {
        "messages"
    }
}
