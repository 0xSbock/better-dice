use futures::{Stream, StreamExt};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use tokio::sync::mpsc;
use warp::{sse::ServerSentEvent, Filter};
use rand::{thread_rng, Rng};
use ammonia;

static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);
type Users = Arc<Mutex<HashMap<usize, mpsc::UnboundedSender<Message>>>>;

#[tokio::main]
async fn main() {
    let users = Arc::new(Mutex::new(HashMap::new()));
    let users = warp::any().map(move || users.clone());

    let roll_send = warp::path!("roll" / String / usize)
        .and(warp::post())
        .and(warp::body::content_length_limit(1024))
        .and(users.clone())
        .map(|name, dice_sides, users| {
            user_dice_roll(name, dice_sides, &users);
            warp::reply()
        });

    let rolls_recv = warp::path("roll").and(warp::get()).and(users).map(|users| {
        // reply using server-sent events
        let stream = user_connected(users);
        warp::sse::reply(warp::sse::keep_alive().stream(stream))
    });

    let static_files = warp::path("static").and(warp::fs::dir("./static"));

    let index = warp::get()
        .and(warp::path::end())
        .and(warp::fs::file("./static/index.html"));

    let routes = index.or(static_files).or(rolls_recv).or(roll_send);
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

/// Message variants.
#[derive(Debug)]
enum Message {
    UserId(usize),
    Reply(String),
}


fn user_connected(
    users: Users,
    ) -> impl Stream<Item = Result<impl ServerSentEvent + Send + 'static, warp::Error>> + Send + 'static
{
    // Use a counter to assign a new unique ID for this user.
    let my_id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);

    eprintln!("new chat user: {}", my_id);

    // Use an unbounded channel to handle buffering and flushing of messages
    // to the event source...
    let (tx, rx) = mpsc::unbounded_channel();

    tx.send(Message::UserId(my_id))
        // rx is right above, so this cannot fail
        .unwrap();

    // Save the sender in our list of connected users.
    users.lock().unwrap().insert(my_id, tx);

    // Convert messages into Server-Sent Events and return resulting stream.
    rx.map(|msg| match msg {
        Message::UserId(my_id) => Ok((warp::sse::event("roll"), warp::sse::data(my_id)).into_a()),
        Message::Reply(reply) => Ok(warp::sse::data(reply).into_b()),
    })
}


fn user_dice_roll(char_name: String, dice_sides: usize, users: &Users) {
    let mut rng = thread_rng();
    let roll_result = rng.gen_range(1, dice_sides);
    let sanitized_name = ammonia::clean(char_name.as_str());
    let response = format!("{{ \"name\": \"{}\", \"roll_result\": {} }}", sanitized_name, roll_result);
    println!("{}", &response);

    // We use `retain` instead of a for loop so that we can reap any user that
    // appears to have disconnected.
    users.lock().unwrap().retain(|_uid, tx| {
        tx.send(Message::Reply(response.clone())).is_ok()
    });
}

