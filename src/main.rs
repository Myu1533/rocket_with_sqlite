use rocket::http::Status;
use rocket::tokio::time::{sleep, Duration};
use rocket::Request;

#[macro_use]
extern crate rocket;

mod member;
mod weight;

#[catch(default)]
fn default_catcher(status: Status, req: &Request) -> String {
    format!(
        "'{}' \n I couldn't find '{}'. Try something else?",
        status,
        req.uri()
    )
}

#[get("/delay/<seconds>")]
async fn delay(seconds: u64) -> String {
    sleep(Duration::from_secs(seconds)).await;
    format!("Waited for {} seconds", seconds)
}

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[get("/world")]
fn world() -> &'static str {
    "Hello, world!"
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index])
        .mount("/hello", routes![world])
        .mount("/delay", routes![delay])
        .attach(member::stage())
        .attach(weight::stage())
        .register("/", catchers![default_catcher])
}
