use crate::core::client::Client;
//Create two clients and test sdp exchange via sending store bytes from iroh
#[tokio::test]
async fn test_connection() {
    let mut p1 = Client::new().await;
    let p2 = Client::new().await;
    let node2_id = p2.get_node_id();
    //Initiliaze the connection with p2 by sending Session + self.node_id
    if let Err(e) = p1.init_connection(node2_id).await {
        panic!("Error initializing the conn {e}");
    };
    //Receive conncetion from p1 by listening for new Sessions + p1.node_id
    if let Err(e) = p2.receive_connection().await {
        panic!("Error receiving the conn {e}");
    }
}
