mod handlers;
mod models;
mod types;
mod utils;

use handlers::handle_connection;
use models::ChatRoomMap;
use std::{env, io::Error as IoError};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), IoError> {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    let channel_state = ChatRoomMap::new();

    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    println!("Listening on: {}", addr);

    while let Ok((stream, addr)) = listener.accept().await {
        let channel_peer_map = channel_state.state.clone();
        tokio::spawn(handle_connection(channel_peer_map, stream, addr));
    }

    Ok(())
}
