use crate::debug::TEST_ROOT;
use rusqlite::{Connection, Result};

struct Database {
    conn: Connection,
}

impl Database {
    pub fn new() -> Self {
        let conn = Connection::open(TEST_ROOT).expect("Error creating db");
        Self { conn }
    }

    pub fn write() {}
    pub fn query() {}
}
