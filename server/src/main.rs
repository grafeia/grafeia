use futures::{future, FutureExt, StreamExt};
use warp::Filter;

#[tokio::main]
async fn main() {
    env_logger::init();

    let log = warp::path("log")
        // The `ws()` filter will prepare the Websocket handshake.
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| {
            // And then our closure will be called when it completes...
            ws.on_upgrade(|websocket| {
                // Just echo all messages back...
                let (tx, rx) = websocket.split();
                rx.for_each(|item| {
                    if let Some(s) = item.as_ref().ok().and_then(|msg| msg.to_str().ok()) {
                        println!("{}", s);
                    }
                    future::ready(())
                })
                /*
                rx.forward(tx).map(|result| {
                    if let Err(e) = result {
                        eprintln!("websocket error: {:?}", e);
                    }
                })
                */
            })
        });
    
    let test = warp::path("test").and(warp::path::end())
        .and(warp::fs::file("/home/sebk/Rust/grafeia/web/diag.html"));
    
    let routes = log.or(test).or(warp::fs::dir("/home/sebk/Rust/grafeia/web"));
    
    warp::serve(routes)
        .run(([0, 0, 0, 0], 8000))
        .await;
}
