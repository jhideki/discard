#[derive(Debug)]
pub struct User {
    id: i32,
    display_name: String,
}

#[derive(Debug)]
pub struct Message {
    id: i32,
    content: Vec<u8>,
    sender_id: i32,
}
