# GPUI Async Websocket Chat

This is an example application to showcase how I was able to get async sockets working with GPUI.

The main rub comes down to leveraging channels.

Our Web socket is split into a `read` Stream and `write` Sink via the futures extension(s).

```rs
let (ws_stream, _) = connect_async("ws://127.0.0.1:3030/chat")
    .await
    .expect("Failed to connect");

let (write, read) = ws_stream.split();
```

We can then create two background threads, one takes a channel and the `write` Sink,
the other a channel and the `read` Stream.

```rs
// Reading messages from a websocket on background thread
cx.background_executor()
    .spawn(handle_incoming(read, incoming_ch))
    .detach();

// Writing messages to a websocket on background thread
cx.background_executor()
    .spawn(handle_outgoing(write, outgoing_ch))
    .detach();
```

Roughly:

Control View --> Puts message on Channel --> loop reads from channel and puts on socket write Sink.

List View <-- Reads message from Channel <-- loop reads from socket read Stream and puts on channel.

We hand our outgoing_ch to a Controls view.
This view has a submit method that takes the text input's content and places it in the channel.

### Running the demo

Please run the websocket server first (separate terminal):

```bash
cargo run --bin server
```

You can then run the app. I recommend running multiple apps as the server is just a broadcaster, so:

```bash
cargo build
./target/debug/gpui-async
```

In again another terminal:

```bash
./target/debug/gpui-async
```
