mod input;
mod workspace;

use async_channel::{unbounded, Receiver, Sender};
use async_std::net::TcpStream;
use async_tungstenite::tungstenite::Message;
use async_tungstenite::{async_std::connect_async, WebSocketStream};
use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use gpui::*;

use input::*;
use workspace::*;

pub struct State {
    messages: Vec<String>,
}

pub async fn handle_incoming(
    mut read: SplitStream<WebSocketStream<TcpStream>>,
    incoming_tx: Sender<String>,
) {
    while let Some(Ok(Message::Text(text))) = read.next().await {
        incoming_tx.send(text.to_string()).await.unwrap();
    }
}

pub async fn handle_outgoing(
    mut write: SplitSink<WebSocketStream<TcpStream>, Message>,
    outgoing_rx: Receiver<String>,
) {
    while let Ok(content) = outgoing_rx.recv().await {
        let _ = write.send(Message::Text(content.into())).await;
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        // Key binds needed for the text input
        cx.bind_keys([
            KeyBinding::new("backspace", Backspace, None),
            KeyBinding::new("delete", Delete, None),
            KeyBinding::new("left", Left, None),
            KeyBinding::new("right", Right, None),
            KeyBinding::new("shift-left", SelectLeft, None),
            KeyBinding::new("shift-right", SelectRight, None),
            KeyBinding::new("cmd-a", SelectAll, None),
            KeyBinding::new("cmd-v", Paste, None),
            KeyBinding::new("cmd-c", Copy, None),
            KeyBinding::new("cmd-x", Cut, None),
            KeyBinding::new("home", Home, None),
            KeyBinding::new("end", End, None),
            KeyBinding::new("ctrl-cmd-space", ShowCharacterPalette, None),
        ]);

        // Communication to/from the socket handled with channels
        let (incoming_tx, incoming_rx) = unbounded::<String>();
        let (outgoing_tx, outgoing_rx) = unbounded::<String>();

        cx.spawn(|cx| async move {
            let (ws_stream, _) = connect_async("ws://127.0.0.1:3030/chat")
                .await
                .expect("Failed to connect");

            let (write, read) = ws_stream.split();

            // Reading messages from a websocket on background thread
            cx.background_executor()
                .spawn(handle_incoming(read, incoming_tx))
                .detach();

            // Writing messages to a websocket on background thread
            cx.background_executor()
                .spawn(handle_outgoing(write, outgoing_rx))
                .detach();
        })
        .detach();

        // Initialize state empty messages
        let state_entity = cx.new(|_cx| State {
            messages: Vec::new(),
        });

        // Spawn task to handle incoming changes from channel
        // These messages are added to the state entity
        // Our message list observes the state entity for changes
        // and updates itself accordingly
        let state_entity_clone = state_entity.clone();
        cx.spawn(|mut cx| async move {
            loop {
                let incoming_msg = incoming_rx.recv().await.unwrap();
                // Update the state with the incoming message
                let _ = state_entity_clone.update(&mut cx, |entity, cx| {
                    // Clear messages if they exceed a certain limit
                    if entity.messages.len() > 1_000 {
                        entity.messages.clear();
                    }
                    entity.messages.push(incoming_msg);
                    cx.notify();
                });
            }
        })
        .detach();

        let window = cx
            .open_window(WindowOptions::default(), |_window, app| {
                // Our workspace uses the outgoing_tx channel to submit messages
                // We pass the state entity to the Workspace, and then to the Controls,
                // but we could set it as a Global to make it accessible from anywhere in the app.
                Workspace::build(app, state_entity, outgoing_tx)
            })
            .unwrap();

        // Focus the window to the controls view
        window
            .update(cx, |workspace, window, cx| {
                window.focus(&workspace.controls_view.focus_handle(cx));
                cx.activate(true);
            })
            .unwrap();
    });
}
