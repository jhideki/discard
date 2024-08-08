use crate::database::models::ToSqlStatement;

use anyhow::Result;
use rusqlite::{params_from_iter, Connection};
use tracing::{error, info, warn};

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

    pub fn write<T: ToSqlStatement>(&mut self, model: &T) -> Result<()> {
        let conn = &self.conn;
        let fields = model.to_sql();
        let columns: Vec<&str> = fields.iter().map(|(col, _)| *col).collect();
        let values: Vec<String> = fields.iter().map(|(_, val)| val.clone()).collect();
        let statement = format!(
            "insert into {} ({}) values ({})",
            T::table_name(),
            columns.join(", "),
            (0..values.len())
                .map(|_| "?")
                .collect::<Vec<_>>()
                .join(", "),
        );
        match conn.execute(&statement, params_from_iter(values.iter())) {
            Ok(_) => info!("Succesfully wrote to db!"),
            Err(e) => error!("Error writing to db: {}", e),
        }
        Ok(())
    }

    pub fn get_conn(&self) -> &Connection {
        &self.conn
    }

    pub fn hard_reset(&self) {
        let conn = &self.conn;
        if let Err(e) = conn.execute("drop table if exists", ["messages"]) {
            error!("Error dropping table messages: {}", e);
        }
        info!("Dropped table messages");
        if let Err(e) = conn.execute("drop table if exists", ["users"]) {
            error!("Error dropping table users: {}", e);
        }
        info!("Dropped table users");
    }
}