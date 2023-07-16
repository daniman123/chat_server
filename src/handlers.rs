use std::{ net::SocketAddr, collections::HashMap };
use futures_channel::mpsc::UnboundedSender;
use tokio_tungstenite::tungstenite::protocol::Message;
type Tx = UnboundedSender<Message>;
use serde_json::Value;
use crate::{ChatRoom, models::ChatUser, utils::author_message_to_json};

pub fn add_peer_to_room(channel_peer_map: ChatRoom, data: Value, addr: SocketAddr, tx: Tx) {
    let mut channels = channel_peer_map.lock().unwrap();

    channels.entry(data["join_room"].to_string()).or_insert_with(HashMap::new).insert(addr, tx);

    println!("connection: {:?}, joined room: {:?}", addr, data["join_room"].clone());
}

pub fn handle_room_disconnect(
    active_room: Option<Value>,
    channel_peer_map: ChatRoom,
    addr: SocketAddr
) {
    if let Some(active_room) = active_room {
        if !active_room.to_string().is_empty() {
            if
                let Some(inner_map) = channel_peer_map
                    .lock()
                    .unwrap()
                    .get_mut(&active_room.to_string().clone())
            {
                inner_map.remove(&addr);
                println!("{} disconnected", &addr);
            }
        }
    }
}

pub fn send_message(channel_peer_map: ChatRoom, chat_user: &ChatUser, addr: SocketAddr) {
    let mut channels = channel_peer_map.lock().unwrap();

    let active_channels = channels.get(&chat_user.active_room.to_string()).unwrap();

    let broadcast_recipients = active_channels
        .iter()
        .filter(|(peer_addr, _)| peer_addr != &&addr)
        .map(|(peer_addr, ws_sink)| (peer_addr, ws_sink));

    let message = author_message_to_json(&chat_user);

    let mut recipients_to_remove = Vec::new();

    for (addr, recp) in broadcast_recipients {
        if let Err(_err) = recp.unbounded_send(message.clone()) {
            // eprintln!("Failed to send message to recipient: {} {:?}", err, recp);
            recipients_to_remove.push(*addr);
        }
    }

    for addr in recipients_to_remove {
        channels.get_mut(&chat_user.active_room.to_string()).unwrap().remove(&addr);
    }
}
