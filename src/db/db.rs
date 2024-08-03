use crate::db::models::ToSqlStatement;
use crate::debug::TEST_ROOT;
use anyhow::Result;
use rusqlite::{params_from_iter, Connection, ParamsFromIter, ToSql};

struct Database {
    conn: Connection,
}

impl Database {
    pub fn new() -> Result<Self> {
        let conn = Connection::open(TEST_ROOT).expect("Error creating db");
        {
            conn.execute(
                "create table if not exists _meta (key text primary key, value text)",
                [],
            )?;
            let mut check = conn.prepare("select value from _meta where key = 'initialized'")?;
            if let Ok(initialized) = check.query_row(["initialized"], |row| row.get::<_, String>(0))
            {
                if initialized.eq("false") {
                    if let Ok(init_script) = std::fs::read_to_string("./init.sql") {
                        conn.execute_batch(&init_script);
                    }
                }
            }
        }
        Ok(Self { conn })
    }

    pub fn write<T: ToSqlStatement>(&mut self, model: T) {
        let conn = &self.conn;
        let fields = model.to_sql();
        let columns: Vec<&str> = fields.iter().map(|(col, _)| *col).collect();
        let values: Vec<String> = fields.iter().map(|(_, val)| val).collect();
        let statement = format!(
            "insert into {} ({}) values ({})",
            T::table_name(),
            columns.join(", "),
            (0..values.len())
                .map(|_| "?")
                .collect::<Vec<_>>()
                .join(", "),
        );
        conn.execute(&statement, params_from_iter(values.iter()));
    }
    pub fn query() {}
}
