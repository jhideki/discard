mod utils;
use discard::database::db::Database;
use discard::database::models::{FromRow, Message, User};
use discard::utils::enums::UserStatus;
use discard::utils::logger;
use utils::Cleanup;

#[tokio::test]
async fn test_db_basic() {
    //Setup
    logger::init_tracing();
    let test_paths = vec!["./test_db_basic1", "./test_db_basic2"];

    let node = iroh::node::Node::memory().spawn().await.unwrap();
    let serialized_id = serde_json::to_string(&node.node_id()).unwrap();

    let cleanup = Cleanup {
        test_paths: &test_paths,
    };

    cleanup.remove_test_paths();

    let db = Database::new(test_paths[0], "./src/database/init.sql");
    assert!(db.is_ok(), "Database initialization failed");

    let mut db = db.expect("Database initialization failed");

    let message = Message {
        message_id: 1,
        content: "test".to_string(),
        sender_node_id: serialized_id.clone(),
        read_ts: None,
        sent_ts: None,
        received_ts: None,
    };

    let user = User {
        user_id: 1,
        display_name: "test".to_string(),
        node_id: serialized_id.clone(),
        status: UserStatus::Online,
    };

    let result = db.write_user(user.clone());
    assert!(
        result
            .map_err(|e| println!("Failed to write user {}", e))
            .is_ok(),
        ""
    );

    let result = db.write_message(message.clone());
    assert!(
        result
            .map_err(|e| println!("Failed to write message: {}", e))
            .is_ok(),
        ""
    );

    {
        let conn = db.get_conn();

        let result = conn.query_row(
            "select * from users where node_id = ?1",
            [serialized_id.clone()],
            User::from_row,
        );

        assert!(result.is_ok(), "Failed to retreive user");

        match result {
            Ok(result_user) => {
                assert!(
                    result_user == user,
                    "Resultant user is not equal to oriringal user"
                );

                println!("------User in db: {}", result_user.user_id);
            }
            Err(e) => {
                println!("Error reading user record: {}", e);
            }
        }
    }
    {
        let conn = db.get_conn();
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
                println!("------Message in db: {}", result_message.content);
            }
            Err(e) => {
                println!("Error reading user record: {}", e);
            }
        }
    }

    let result = db.update_status(node.node_id(), UserStatus::Offline);

    assert!(
        result
            .map_err(|e| println!("Error updating status {},", e))
            .is_ok(),
        ""
    );
    {
        let conn = db.get_conn();
        let result = conn.query_row(
            "select * from users where node_id = ?1",
            [serialized_id.clone()],
            User::from_row,
        );

        assert!(result
            .map(|u| assert!(u.status == UserStatus::Offline))
            .map_err(|e| println!("Error quering users {}", e))
            .is_ok());
    }

    let result = db.update_status(node.node_id(), UserStatus::Online);

    assert!(
        result
            .map_err(|e| println!("Error updating status {},", e))
            .is_ok(),
        ""
    );
    {
        let conn = db.get_conn();
        let result = conn.query_row(
            "select * from users where node_id = ?1",
            [serialized_id.clone()],
            User::from_row,
        );
        assert!(result
            .map(|u| assert!(u.status == UserStatus::Online))
            .map_err(|e| println!("Error quering user {}", e))
            .is_ok());
    }

    //Drop tables
    assert!(
        db.hard_reset()
            .map_err(|e| println!("Error hard resetting {}", e))
            .is_ok(),
        ""
    );
}
