use crate::{
    models::ChatUser,
    types::{ChatRoom, Tx},
    utils::author_message_to_json,
};
use futures_channel::mpsc::unbounded;
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};
use serde_json::Value;
use std::{collections::HashMap, net::SocketAddr};
use tokio::net::TcpStream;

pub fn add_peer_to_room(channel_peer_map: ChatRoom, data: Value, addr: SocketAddr, tx: Tx) {
    let mut channels = channel_peer_map.lock().unwrap();

    channels
        .entry(data["join_room"].to_string())
        .or_insert_with(HashMap::new)
        .insert(addr, tx);

    println!(
        "connection: {:?}, joined room: {:?}",
        addr,
        data["join_room"].clone()
    );
}

pub fn send_message(channel_peer_map: ChatRoom, chat_user: &ChatUser, addr: SocketAddr) {
    let current_active_room = &chat_user.active_room.to_string();

    let mut channels = channel_peer_map.lock().unwrap();

    let active_channels = channels.get(current_active_room).unwrap();

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
            recp.close_channel();
        }
    }

    remove_err_connections(recipients_to_remove, &mut channels, &current_active_room)
}

use std::collections::HashSet;

fn remove_err_connections(
    recipients_to_remove: Vec<SocketAddr>,
    channels_map: &mut HashMap<
        String,
        HashMap<
            SocketAddr,
            futures_channel::mpsc::UnboundedSender<tokio_tungstenite::tungstenite::Message>,
        >,
    >,
    current_active_room_name: &str,
) {
    if !current_active_room_name.is_empty() {
        if let Some(channel) = channels_map.get_mut(current_active_room_name) {
            let bad_peers: HashSet<_> = recipients_to_remove.iter().cloned().collect();
            channel.retain(|key, _| !bad_peers.contains(key));
        }
    }
}

pub async fn handle_connection(
    channel_peer_map: ChatRoom,
    raw_stream: TcpStream,
    addr: SocketAddr,
) {
    println!("Incoming TCP connection from: {}", addr);

    let ws_stream = match tokio_tungstenite::accept_async(raw_stream).await {
        Ok(ws_stream) => ws_stream,
        Err(err) => {
            eprintln!("Error during the WebSocket handshake: {}", err);
            return;
        }
    };

    println!("WebSocket connection established: {}", addr);

    let (tx, rx) = unbounded();

    let (outgoing, incoming) = ws_stream.split();

    let mut active_room: Option<String> = None; // Define a variable to hold the active_room value

    let broadcast_incoming = incoming.try_for_each(|msg| {
        println!("{:?}", msg);

        if let Ok(json) = msg.to_text().map_err(|_| ()) {
            if let Ok(data) = serde_json::from_str::<Value>(&json) {
                if data.get("join_room").is_none() {
                    active_room = Some(data["room_name"].clone().to_string());
                    let chat_user = ChatUser::new(data);
                    send_message(channel_peer_map.clone(), &chat_user, addr);
                } else if let Some(_) = data.get("join_room") {
                    add_peer_to_room(channel_peer_map.clone(), data, addr, tx.clone());
                }
            }
        }
        future::ok(())
    });

    let receive_from_others = rx.map(Ok).forward(outgoing);

    pin_mut!(broadcast_incoming, receive_from_others);
    future::select(broadcast_incoming, receive_from_others).await;
    let mut channels = channel_peer_map.lock().unwrap().clone();

    let active_room_name = &active_room.unwrap_or("".to_string());
    remove_err_connections(vec![addr], &mut channels, active_room_name);
}
