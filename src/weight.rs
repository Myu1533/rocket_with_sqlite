use futures::stream::TryStreamExt;
use rocket::fairing::{self, AdHoc};
use rocket::response::status::Created;
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::{futures, Build, Rocket};
use rocket_db_pools::{sqlx, Connection, Database};
use uuid::Uuid;

#[derive(Database)]
#[database("sqlx")]
struct BodyControl(sqlx::SqlitePool);

type Result<T, E = rocket::response::Debug<sqlx::Error>> = std::result::Result<T, E>;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
struct Post {
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    member_id: String,
    value: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
struct List {
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    value: Option<f64>,
    member_id: Option<String>,
    create_time: Option<i64>,
    update_time: Option<i64>,
}

#[post("/", data = "<post>")]
async fn create(
    mut db: Connection<BodyControl>,
    mut post: Json<Post>,
) -> Result<Created<Json<Post>>> {
    let uid = Uuid::new_v4();
    let id = uid.to_string();
    let results = sqlx::query!(
        "INSERT INTO weight (id, value, member_id, create_time, update_time) VALUES (?, ?, ?, unixepoch(), unixepoch()) RETURNING id",
        id,
        post.value,
        post.member_id
    )
    .fetch(&mut **db)
    .try_collect::<Vec<_>>()
    .await?;

    post.id = results.first().expect("returning results").id.clone();
    Ok(Created::new("/").body(post))
}

#[get("/?<member_id>")]
async fn list(mut db: Connection<BodyControl>, member_id: String) -> Result<Json<Vec<List>>> {
    let results = sqlx::query!("SELECT * FROM weight where member_id = ?", member_id)
        .fetch_all(&mut **db)
        .await?;

    Ok(Json(
        results
            .into_iter()
            .map(|record| List {
                id: record.id,
                value: record.value,
                member_id: record.member_id,
                create_time: record.create_time,
                update_time: record.update_time,
            })
            .collect(),
    ))
}

#[delete("/<id>")]
async fn delete(mut db: Connection<BodyControl>, id: String) -> Result<String> {
    let result = sqlx::query!("DELETE FROM weight WHERE id = ?", id)
        .execute(&mut **db)
        .await?;

    print!("{:?}", result);
    Ok("Deleted Success!".to_string())
}

async fn run_migrations(rocket: Rocket<Build>) -> fairing::Result {
    match BodyControl::fetch(&rocket) {
        Some(db) => match sqlx::migrate!("db/migrations").run(&**db).await {
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
            .mount("/weight", routes![create, list, delete])
    })
}
