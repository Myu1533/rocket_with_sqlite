use futures::{future::TryFutureExt, stream::TryStreamExt};
use rocket::fairing::{self, AdHoc};
use rocket::response::status::Created;
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::{futures, Build, Rocket};
use rocket_db_pools::{sqlx, Connection, Database};

#[derive(Database)]
#[database("sqlx")]
struct BodyControl(sqlx::SqlitePool);

type Result<T, E = rocket::response::Debug<sqlx::Error>> = std::result::Result<T, E>;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
struct Post {
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    id: Option<i64>,
    value: f64,
    member_id: i64,
}

struct List {
    // #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    id: i64,
    value: f64,
    member_id: i64,
    create_time: String,
    update_time: String,
}

#[post("/", data = "<post>")]
async fn create(
    mut db: Connection<BodyControl>,
    mut post: Json<Post>,
) -> Result<Created<Json<Post>>> {
    let results = sqlx::query!(
        "INSERT INTO weight (id, value, member_id, create_time, update_time) VALUES (?, ?, ?, datetime(), datetime()) RETURNING id",
        post.id,
        post.value,
        post.member_id
    )
    .fetch(&mut **db)
    .try_collect::<Vec<_>>()
    .await?;

    post.id = Some(results.first().expect("returning results").id);
    Ok(Created::new("/").body(post))
}

#[get("/?<member_id>")]
async fn list(mut db: Connection<BodyControl>, member_id: i64) -> Result<Json<Vec<List>>> {
    let results = sqlx::query_as!(
        List,
        "SELECT id, value, member_id, create_time, update_time FROM weight"
    )
    .fetch(&mut **db)
    .map_ok(|record| record)
    .try_collect::<Vec<_>>()
    .await?;

    Ok(Json(results))
}

async fn run_migrations(rocket: Rocket<Build>) -> fairing::Result {
    match BodyControl::fetch(&rocket) {
        Some(db) => match sqlx::migrate!("db/sqlx/migrations").run(&**db).await {
            Ok(_) => Ok(rocket),
            Err(e) => {
                error!("Failed to run migrations: {:?}", e);
                Err(rocket)
            }
        },
        None => Err(rocket),
    }
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("SQLx stage", |rocket| async {
        rocket
            .attach(BodyControl::init())
            .attach(AdHoc::try_on_ignite("SQLx migrate", run_migrations))
            .mount("/weight", routes![create])
    })
}
