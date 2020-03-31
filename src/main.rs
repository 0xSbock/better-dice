use futures::{Stream, StreamExt};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use tokio::sync::mpsc;
use warp::{sse::ServerSentEvent, Filter};
use rand::{thread_rng, Rng};
use percent_encoding::percent_decode;
use ammonia;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "better-dice", about = "a simple synchronized dice rolling site")]
struct Opt {
    /// Port
    #[structopt(short, long, default_value = "3030")]
    port: u16,

    /// Public hosting
    #[structopt(long)]
    public: bool
}

static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);
type Users = Arc<Mutex<HashMap<usize, mpsc::UnboundedSender<Message>>>>;

#[tokio::main]
async fn main() {
    let opt = Opt::from_args();

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

    // add static file delivery for js and css files
    let static_files = warp::path("static").and(warp::fs::dir("./static"));

    // serve the static html for the index route
    let index = warp::get()
        .and(warp::path::end())
        .and(warp::fs::file("./static/index.html"));

    let routes = index.or(static_files).or(rolls_recv).or(roll_send);
    if opt.public {
        warp::serve(routes).run(([0, 0, 0, 0], opt.port)).await;
    } else {
        warp::serve(routes).run(([127, 0, 0, 1], opt.port)).await;
    }
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
    // names could be url encoded (e.g. "name%20surname")
    let url_decoded_name = percent_decode(&char_name.as_bytes()).decode_utf8().unwrap();
    // sanitize the username input to prevent XSS
    let sanitized_name = ammonia::clean(&url_decoded_name);
    // construct a JSON response
    let response = format!("{{ \"name\": \"{}\", \"roll_result\": {} }}", sanitized_name, roll_result);

    // We use `retain` instead of a for loop so that we can reap any user that
    // appears to have disconnected.
    users.lock().unwrap().retain(|_uid, tx| {
        tx.send(Message::Reply(response.clone())).is_ok()
    });
}

