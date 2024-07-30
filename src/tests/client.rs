use crate::core::client::Client;
//Create two clients and test sdp exchange via sending store bytes from iroh
#[tokio::test]
async fn test_connection() {
    let p1 = Client::new().await;
    let p2 = Client::new().await;
    let node1 = p1.get_node_id();
    let node2 = p2.get_node_id();
    p1.init_connection(node2).await;
}
