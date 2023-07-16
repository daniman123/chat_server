use serde_json::Value;

#[derive(Debug)]
pub struct ChatUser {
    pub active_room: Value,
    pub author: Value,
    pub message: Value,
}

impl ChatUser {
    pub fn new(data: Value) -> ChatUser {
        return ChatUser {
            active_room: data["room_name"].clone(),
            author: data["author"].clone(),
            message: data["message"].clone(),
        };
    }
}
