mod models;
mod utils;
use models::ChatUser;

mod handlers;
use handlers::{ add_peer_to_room, handle_room_disconnect };

use std::{ collections::HashMap, env, io::Error as IoError, net::SocketAddr, sync::{ Arc, Mutex } };

use futures_channel::mpsc::{ unbounded, UnboundedSender };
use futures_util::{ future, pin_mut, stream::TryStreamExt, StreamExt };
use serde_json::Value;
use tokio::net::{ TcpListener, TcpStream };
use tokio_tungstenite::tungstenite::protocol::Message;

use crate::handlers::send_message;

type Tx = UnboundedSender<Message>;

type ChatRoom = Arc<Mutex<HashMap<String, HashMap<SocketAddr, Tx>>>>;

async fn handle_connection(channel_peer_map: ChatRoom, raw_stream: TcpStream, addr: SocketAddr) {
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

    let mut active_room: Option<Value> = None; // Define a variable to hold the active_room value

    let broadcast_incoming = incoming.try_for_each(|msg| {
        if let Ok(json) = msg.to_text().map_err(|_| ()) {
            if let Ok(data) = serde_json::from_str::<Value>(&json) {
                if data.get("join_room").is_none() {
                    active_room = Some(data["room_name"].clone());
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

    handle_room_disconnect(active_room, channel_peer_map, addr);
}

#[tokio::main]
async fn main() -> Result<(), IoError> {
    let addr = env
        ::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    let channel_state = ChatRoom::new(Mutex::new(HashMap::new()));

    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    println!("Listening on: {}", addr);

    while let Ok((stream, addr)) = listener.accept().await {
        let channel_peer_map = channel_state.clone();
        tokio::spawn(handle_connection(channel_peer_map, stream, addr));
    }

    Ok(())
}
