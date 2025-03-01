// Websocket server
use std::{env, io::Error, sync::Arc};

use async_std::net::{TcpListener, TcpStream};
use async_std::sync::Mutex; // Changed to async_std's Mutex
use async_std::task;
use async_tungstenite::tungstenite::Message;
use futures::prelude::*;
use futures::stream::SplitSink;
use std::collections::HashMap;
use uuid::Uuid;

// Active connections are stored as a map from UUID to websocket sender
type Tx = SplitSink<async_tungstenite::WebSocketStream<TcpStream>, Message>;
type PeerMap = Arc<Mutex<HashMap<String, Tx>>>; // Using async_std's Mutex

async fn run() -> Result<(), Error> {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:3030".to_string());

    // Store active connections
    let peers: PeerMap = Arc::new(Mutex::new(HashMap::new()));

    // Create the event loop and TCP listener we'll accept connections on
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    println!("Listening on: {}", addr);

    // Accept and handle each incoming connection
    while let Ok((stream, _)) = listener.accept().await {
        let peer_map = peers.clone();
        task::spawn(accept_connection(stream, peer_map));
    }

    Ok(())
}

async fn accept_connection(stream: TcpStream, peer_map: PeerMap) {
    let addr = stream
        .peer_addr()
        .expect("connected streams should have a peer address");
    println!("Peer address: {}", addr);

    let ws_stream = async_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    println!("New WebSocket connection: {}", addr);

    // Generate a unique ID for this connection
    let id = Uuid::new_v4().to_string();
    println!("Connection ID: {}", id);

    let (tx, mut rx) = ws_stream.split();

    // Add the sender to our peer map for broadcasting
    peer_map.lock().await.insert(id.clone(), tx); // Using .await with async mutex

    // Handle incoming messages
    while let Some(msg) = rx.next().await {
        if let Ok(msg) = msg {
            if msg.is_text() || msg.is_binary() {
                println!("Received a message from {}: {:?}", addr, msg);

                // Create a list of all peers to broadcast to
                let peers_to_send = peer_map
                    .lock()
                    .await // Using .await with async mutex
                    .iter()
                    .map(|(id, _)| id.clone())
                    .collect::<Vec<String>>();

                // Broadcast the message to all connected clients
                for peer_id in peers_to_send {
                    if let Some(peer_tx) = peer_map.lock().await.get_mut(&peer_id) {
                        // Using .await with async mutex
                        if let Err(e) = peer_tx.send(msg.clone()).await {
                            println!("Error sending to {}: {:?}", peer_id, e);
                        }
                    }
                }
            }
        } else {
            break;
        }
    }

    // Connection closed, remove from the peer map
    peer_map.lock().await.remove(&id); // Using .await with async mutex
    println!("Connection {} closed", addr);
}

fn main() -> Result<(), Error> {
    task::block_on(run())
}
