use futures_channel::mpsc::UnboundedSender;
use std::{ collections::HashMap, net::SocketAddr, sync::{ Arc, Mutex } };
use tokio_tungstenite::tungstenite::protocol::Message;

pub type Tx = UnboundedSender<Message>;
pub type ChatRoom = Arc<Mutex<HashMap<String, HashMap<SocketAddr, Tx>>>>;
pub type UnlockedMap = HashMap<String, HashMap<SocketAddr, UnboundedSender<Message>>>;
