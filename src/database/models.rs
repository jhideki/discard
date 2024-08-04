use crate::utils::types::NodeId;
use anyhow::Result;
use rusqlite;

pub trait ToSqlStatement {
    fn to_sql(&self) -> Vec<(&str, String)>;
    fn table_name() -> &'static str;
}

pub trait FromRow {
    type Model;
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self::Model>;
}

#[derive(Debug, PartialEq, Eq)]
pub struct User {
    pub user_id: i32,
    pub display_name: String,
    pub node_id: String,
}

impl FromRow for User {
    type Model = User;
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<User> {
        Ok(Self {
            user_id: row.get("id")?,
            display_name: row.get("display_name")?,
            node_id: row.get("node_id")?,
        })
    }
}

impl ToSqlStatement for User {
    fn to_sql(&self) -> Vec<(&str, String)> {
        vec![
            ("id", self.user_id.to_string()),
            ("display_name", self.display_name.clone()),
            ("node_id", self.node_id.to_string()),
        ]
    }
    fn table_name() -> &'static str {
        "user"
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Message {
    pub message_id: i32,
    pub content: String,
    pub sender_id: i32,
}

impl FromRow for Message {
    type Model = Message;
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Message> {
        Ok(Self {
            message_id: row.get("id")?,
            content: row.get("content")?,
            sender_id: row.get("display_name")?,
        })
    }
}

impl ToSqlStatement for Message {
    fn to_sql(&self) -> Vec<(&str, String)> {
        vec![
            ("id", self.message_id.to_string()),
            ("display_name", self.content.clone()),
            ("sender_id", self.sender_id.to_string()),
        ]
    }
    fn table_name() -> &'static str {
        "message"
    }
}
