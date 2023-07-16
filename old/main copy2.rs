use std::{ collections::HashMap, env, io::Error as IoError, net::SocketAddr, sync::{ Arc, Mutex } };

use futures_channel::mpsc::{ unbounded, UnboundedSender };
use futures_util::{ future, pin_mut, stream::TryStreamExt, StreamExt };
use serde_json::Value;
use tokio::net::{ TcpListener, TcpStream };
use tokio_tungstenite::tungstenite::protocol::Message;

type Tx = UnboundedSender<Message>;
type ChatRoom = Arc<Mutex<HashMap<String, HashMap<SocketAddr, Tx>>>>;

#[derive(Debug)]
struct ChatUser {
    active_room: Value,
    author: Value,
    message: Value,
}

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
                    let chat_user = ChatUser {
                        active_room: data["room_name"].clone(),
                        author: data["author"].clone(),
                        message: data["message"].clone(),
                    };

                    let mut channels = channel_peer_map.lock().unwrap();

                    let active_channels = channels.get(&chat_user.active_room.to_string()).unwrap();

                    let broadcast_recipients = active_channels
                        .iter()
                        .filter(|(peer_addr, _)| peer_addr != &&addr)
                        .map(|(peer_addr, ws_sink)| (peer_addr, ws_sink));

                    let auth_msg =
                        serde_json::json!({
                    "author": &chat_user.author,
                    "message": &chat_user.message,
                });

                    let message = Message::Text(auth_msg.to_string());

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
                } else if let Some(_) = data.get("join_room") {
                    let mut channels = channel_peer_map.lock().unwrap();

                    channels
                        .entry(data["join_room"].to_string())
                        .or_insert_with(HashMap::new)
                        .insert(addr, tx.clone());

                    println!(
                        "connection: {:?}, joined room: {:?}",
                        addr,
                        data["join_room"].clone()
                    );
                }
            }
        }
        future::ok(())
    });
    let receive_from_others = rx.map(Ok).forward(outgoing);

    pin_mut!(broadcast_incoming, receive_from_others);
    future::select(broadcast_incoming, receive_from_others).await;

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

fn handle_room_disconnect() {
    
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
