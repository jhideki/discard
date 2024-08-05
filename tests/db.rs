mod utils;
use discard::core::client::Client;
use discard::database::db::Database;
use discard::database::models::{FromRow, Message, User};
use discard::utils::logger;
use utils::Cleanup;

#[tokio::test]
async fn test_db_basic() {
    //Setup
    logger::init_tracing();
    let test_paths = vec!["./test_db.db3", "./test-path4"];

    let cleanup = Cleanup {
        test_paths: &test_paths,
    };

    cleanup.remove_test_paths();

    let db = Database::new(test_paths[0], "./src/database/init.sql");
    assert!(db.is_ok(), "Database initialization failed");

    let client = Client::new(test_paths[1]).await;

    let mut db = db.expect("Database initialization failed");

    let message = Message {
        message_id: 1,
        content: "test".to_string(),
        sender_id: 1,
    };

    let user = User {
        user_id: 1,
        display_name: "test".to_string(),
        node_id: client.get_node_id().to_string(),
    };

    let result = db.write(&user);
    assert!(result.is_ok(), "Failed to write User");

    let result = db.write(&message);
    assert!(result.is_ok(), "Failed to write message");

    let conn = db.get_conn();

    let result = conn.query_row(
        "select * from users where user_id = ?1",
        [&user.user_id],
        User::from_row,
    );

    assert!(result.is_ok(), "Failed to retreive user");

    match result {
        Ok(result_user) => {
            assert!(
                result_user == user,
                "Resultant user is not equal to oriringal user"
            );
        }
        Err(e) => {
            println!("Error reading user record: {}", e);
        }
    }

    let result = conn.query_row(
        "select * from messages where message_id = ?1",
        [&message.message_id],
        Message::from_row,
    );

    assert!(result.is_ok(), "Failed to retreive message");

    match result {
        Ok(result_message) => {
            assert!(
                result_message == message,
                "Resultant message is not equal to oriringal message"
            );
        }
        Err(e) => {
            println!("Error reading user record: {}", e);
        }
    }

    //Drop tables
    db.hard_reset();
}
