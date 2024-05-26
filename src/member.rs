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
    name: Option<String>,
    nickname: Option<String>,
    sex: Option<i64>,
    relationship: Option<i64>,
}

#[post("/", data = "<post>")]
async fn create(
    mut db: Connection<BodyControl>,
    mut post: Json<Post>,
) -> Result<Created<Json<Post>>> {
    let uid = Uuid::new_v4();
    let id = uid.to_string();
    print!("{}", id);
    let results = sqlx::query!(
        "INSERT INTO member (id, name, nickname, sex, relationship) VALUES (?, ?, ?, ?, ?) RETURNING id",
        id,
        post.name,
        post.nickname,
        post.sex,
        post.relationship
    )
    .fetch(&mut **db)
    .try_collect::<Vec<_>>()
    .await?;

    post.id = results.first().expect("returning results").id.clone();
    Ok(Created::new("/").body(post))
}

#[get("/")]
async fn list(mut db: Connection<BodyControl>) -> Result<Json<Vec<Post>>> {
    let results = sqlx::query!("SELECT * FROM member")
        .fetch_all(&mut **db)
        .await?;

    Ok(Json(
        results
            .into_iter()
            .map(|record| Post {
                id: record.id,
                name: record.name,
                nickname: record.nickname,
                sex: record.sex,
                relationship: record.relationship,
            })
            .collect(),
    ))
}

#[delete("/<id>")]
async fn delete(mut db: Connection<BodyControl>, id: String) -> Result<String> {
    let result = sqlx::query!("DELETE FROM member WHERE id = ?", id)
        .execute(&mut **db)
        .await;

    print!("{:?}", result);
    Ok("success".to_string())
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
            .mount("/member", routes![create, list, delete])
    })
}
