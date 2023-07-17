use crate::types::ChatRoom;
use serde_json::Value;
use std::{collections::HashMap, sync::Mutex};

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

pub struct ChatRoomMap {
    pub state: ChatRoom,
}

impl ChatRoomMap {
    pub fn new() -> Self {
        ChatRoomMap {
            state: ChatRoom::new(Mutex::new(HashMap::new())),
        }
    }
}
