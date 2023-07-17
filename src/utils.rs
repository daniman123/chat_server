use crate::models::ChatUser;
use tokio_tungstenite::tungstenite::protocol::Message;

pub fn author_message_to_json(chat_user: &ChatUser) -> Message {
    let auth_msg = serde_json::json!({
        "author": &chat_user.author,
        "message": &chat_user.message,
    });
    return Message::Text(auth_msg.to_string());
}
