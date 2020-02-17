use futures::{future, FutureExt, stream, Stream, StreamExt};
use futures::channel::mpsc::{channel, Sender, Receiver};
use tokio::task;
use warp::Filter;
use warp::filters::ws::{WebSocket, Message};
use grafeia_core::*;
use std::sync::Mutex;
use std::sync::Arc;
use std::collections::HashMap;
use std::borrow::Cow;
use bincode;

#[macro_use] extern crate log;

struct Server {
    document: GlobalDocument,
    clients: HashMap<SiteId, Client>,
    last_id: SiteId
}
impl Server {
    fn new(document: GlobalDocument) -> Self {
        Server {
            document,
            clients: HashMap::new(),
            last_id: SiteId(1)
        }
    }
    fn next_id(&mut self) -> SiteId {
        let n = SiteId(self.last_id.0 + 1);
        self.last_id = n;
        n
    }
    fn respond(&mut self, id: SiteId, msg: Message) {
        if let Some(client) = self.clients.get_mut(&id) {
            match client.sender.try_send(Ok(msg)) {
                Ok(_) => {}
                Err(e) => {
                    info!("try_send to {:?} failed: {:?}. removing.", id, e);
                    self.clients.remove(&id);
                }
            }
        }
    }
}
type ClientTx = Sender<Result<Message, warp::Error>>;
struct Client {
    sender: ClientTx
}

fn user_message(data: Vec<u8>, id: SiteId, server: &Arc<Mutex<Server>>) {
    let command = ClientCommand::decode(&data).unwrap();

    match command {
        ClientCommand::Join => {
            info!("Join @ {:?} -> Welcome", id);
            let msg = Message::binary(ServerCommand::Welcome(id).encode());
            server.lock().unwrap().respond(id, msg);
        }
        ClientCommand::GetAll => {
            info!("GetAll @ {:?} -> Document", id);
            let mut s = server.lock().unwrap();
            let msg = Message::binary(ServerCommand::Document(Cow::Borrowed(&s.document)).encode());
            s.respond(id, msg);
        }
        ClientCommand::Op(op) => {
            info!("Op({:?}) @ {:?}", op, id);
            let msg = Message::binary(ServerCommand::Op(Cow::Borrowed(&op)).encode());
            let mut s = server.lock().unwrap();
            s.clients.retain(|&cid, client| {
                if cid != id {
                    info!("  -> Op @ {:?}", cid);
                    if let Err(e) = client.sender.try_send(Ok(msg.clone())) {
                        info!("try_send to {:?} failed: {:?}. removing.", cid, e);
                        return false;
                    }
                }
                true
            });
            s.document.apply(op);
        }
    }
}

async fn client_connected(ws: WebSocket, server: Arc<Mutex<Server>>) {
    let (ws_tx, ws_rx) = ws.split();
    let (mut tx, rx) = channel(64);

    task::spawn(rx.forward(ws_tx));
    
    let id = {
        let mut s = server.lock().unwrap();
        let id = s.next_id();
        s.clients.insert(id, Client { sender: tx });
        id
    };
    ws_rx.for_each(move |item| {
        if let Ok(message) = item {
            if message.is_binary() {
                let data = message.into_bytes();
                user_message(data, id, &server);
            } else if message.is_ping() {
                server.lock().unwrap().respond(id, message);
            } else if message.is_text() {
                println!("{}", message.to_str().unwrap());
            }
        } else {
            server.lock().unwrap().clients.remove(&id);
        }
        future::ready(())
    }).await;
}

type Version = (u16, u16);
type Data = (Target, Design, LocalDocument);

fn load() -> GlobalDocument {
    let path = std::env::args().nth(1).expect("no file given");
    let data = std::fs::read(path).expect("can't open file");
    let (version, data): (Version, Data) = bincode::deserialize(&data).expect("failed to decode");
    let (target, design, document) = data;
    let document = Document::from_local(document, SiteId(1));
    let global = document.to_global(&target, &design);
    global
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let document = load();

    let server = Arc::new(Mutex::new(Server::new(document)));
    let log = warp::path("log")
    // The `ws()` filter will prepare the Websocket handshake.
    .and(warp::ws())
    .map(move |ws: warp::ws::Ws| {
        let server = server.clone();
        // And then our closure will be called when it completes...
        ws.on_upgrade(move |websocket| client_connected(websocket, server))
    });

    let test = warp::path("test").and(warp::path::end())
        .and(warp::fs::file("/home/sebk/Rust/grafeia/web/diag.html"));

    let routes = log.or(test).or(warp::fs::dir("/home/sebk/Rust/grafeia/web"));

    warp::serve(routes)
        .run(([0, 0, 0, 0], 8000))
        .await;
}
