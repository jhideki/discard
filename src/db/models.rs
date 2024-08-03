use crate::utils::types::NodeId;

pub trait ToSqlStatement {
    fn to_sql(&self) -> Vec<(&str, String)>;
    fn table_name() -> &'static str;
}

#[derive(Debug)]
pub struct User {
    id: i32,
    display_name: String,
    node_id: NodeId,
}

impl ToSqlStatement for User {
    fn to_sql(&self) -> Vec<(&str, String)> {
        vec![
            ("id", self.id.to_string()),
            ("display_name", self.display_name.clone()),
            ("node_id", self.node_id.to_string()),
        ]
    }
    fn table_name() -> &'static str {
        "user"
    }
}

#[derive(Debug)]
pub struct Message {
    id: i32,
    content: String,
    sender_id: i32,
}

impl ToSqlStatement for Message {
    fn to_sql(&self) -> Vec<(&str, String)> {
        vec![
            ("id", self.id.to_string()),
            ("display_name", self.content.clone()),
            ("sender_id", self.sender_id.to_string()),
        ]
    }
    fn table_name() -> &'static str {
        "message"
    }
}
