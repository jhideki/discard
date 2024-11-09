use crate::database::models::{Message, User};
use crate::utils::enums::UserStatus;
use crate::utils::types::NodeId;

use anyhow::Result;
use rusqlite::{params, Connection};
use tracing::{error, info, warn};

#[derive(Debug)]
pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(root: &str, init_script: &str) -> Result<Self> {
        info!("Creating new db conn");

        let conn = Connection::open(root).expect("Error creating db");
        {
            conn.execute(
                "create table if not exists _meta (key text primary key, value text)",
                [],
            )?;
            info!("meta table created");
            let mut check = conn.prepare("select value from _meta where key = ?1")?;
            match check.query_row(["initialized"], |row| row.get::<_, String>(0)) {
                Ok(initialized) => {
                    info!("initialized: {}", initialized);
                    if initialized.eq("false") {
                        if let Ok(init_script) = std::fs::read_to_string(init_script) {
                            conn.execute_batch(&init_script)?;
                        }
                        conn.execute("insert into _meta", ["initialized", "true"])?;
                    }
                }
                Err(e) => {
                    warn!("{}", e);
                    match std::fs::read_to_string(init_script) {
                        Ok(init_script) => {
                            if let Err(e) = conn.execute_batch(&init_script) {
                                error!("Error execurting init script: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Error loading init.sql: {}", e);
                        }
                    }

                    if let Err(e) = conn.execute(
                        "insert into _meta (key, value) values (?1, ?2)",
                        ["initialized", "true"],
                    ) {
                        error!("Error inserting into _meta {}", e);
                    }
                }
            }
            info!("Database is initialized");
        }
        Ok(Self { conn })
    }

    pub fn write_user(&self, user: User) -> Result<()> {
        let conn = &self.conn;
        conn.execute(
            "insert into users (display_name, node_id, status) values (?1, ?2, ?3)",
            [&user.display_name, &user.node_id, &user.status.to_string()],
        )?;
        Ok(())
    }

    pub fn write_message(&self, message: Message) -> Result<()> {
        let conn = &self.conn;
        conn.execute(
            "insert into messages (content, sender_node_id, received_ts,sent_ts, read_ts ) values (?1, ?2, ?3, ?4, ?5)",
            params![
                &message.content,
                &message.sender_node_id,
                &message.received_ts,
                &message.sent_ts,
                &message.read_ts,
            ],
        )?;
        Ok(())
    }

    pub fn update_status(&mut self, node_id: NodeId, user_status: UserStatus) -> Result<()> {
        let conn = &self.conn;
        let node_id = serde_json::to_string(&node_id)?;

        match conn.execute(
            "update users set status = ?1 where node_id = ?2",
            [&user_status.to_string(), &node_id],
        ) {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("Error updating user status {}", e)),
        }
    }

    pub fn get_node_ids(&self) -> Result<Vec<NodeId>> {
        let conn = &self.conn;
        let mut statement = conn.prepare("select node_id from users where user_id > 1")?;
        let rows: Vec<String> = statement
            .query_map([], |row| Ok(row.get(0)?))?
            .filter_map(Result::ok)
            .collect();
        let node_ids = rows
            .into_iter()
            .map(|row| serde_json::from_str(&row).expect("Error deserialzing"));
        Ok(node_ids.collect())
    }

    pub fn get_conn(&self) -> &Connection {
        &self.conn
    }

    pub fn hard_reset(&self) -> Result<()> {
        let conn = &self.conn;
        conn.execute_batch("drop table if exists users;")?;
        info!("Dropped table messages");
        conn.execute_batch("drop table if exists messages;")?;
        info!("Dropped table users");
        Ok(())
    }
}
