use crate::utils::enums::UserStatus;
use rusqlite::{
    self,
    types::FromSqlError,
    types::{FromSql, FromSqlResult, ToSqlOutput},
    ToSql,
};

impl ToSql for UserStatus {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(self.to_string().into())
    }
}
impl FromSql for UserStatus {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> FromSqlResult<Self> {
        value
            .as_str()?
            .parse()
            .map_err(|e| FromSqlError::Other(Box::new(e)))
    }
}

pub trait FromRow {
    type Model;
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self::Model>;
    fn table_name() -> &'static str;
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct User {
    pub user_id: i32,
    pub display_name: String,
    pub node_id: String,
    pub status: UserStatus,
}

impl FromRow for User {
    type Model = User;
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<User> {
        Ok(Self {
            user_id: row.get("user_id")?,
            display_name: row.get("display_name")?,
            node_id: row.get("node_id")?,
            status: row.get("status")?,
        })
    }
    fn table_name() -> &'static str {
        "users"
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Message {
    pub message_id: i32,
    pub content: String,
    pub sender_node_id: String,
    pub received_ts: Option<String>,
    pub sent_ts: Option<String>,
    pub read_ts: Option<String>,
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TextMessage: {}", self.content)
    }
}

impl FromRow for Message {
    type Model = Message;
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Message> {
        Ok(Self {
            message_id: row.get("message_id")?,
            content: row.get("content")?,
            sender_node_id: row.get("sender_node_id")?,
            received_ts: row.get("received_ts")?,
            sent_ts: row.get("sent_ts")?,
            read_ts: row.get("read_ts")?,
        })
    }
    fn table_name() -> &'static str {
        "messages"
    }
}
