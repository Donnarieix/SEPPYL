use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, Database, DatabaseConnection, EntityTrait, QueryFilter,
};

use crate::entities;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
}

impl AppState {
    pub async fn default() -> Self {
        // Use an ignored local copy rather than writing real player data into the
        // SQLite file already tracked by Git. Keep that original file untouched.
        const LOCAL_DB: &str = "data/seppyl.local.db";
        std::fs::create_dir_all("data").expect("Unable to create data directory");
        if !std::path::Path::new(LOCAL_DB).exists()
            && std::path::Path::new("data/sappyl.db").exists()
        {
            std::fs::copy("data/sappyl.db", LOCAL_DB)
                .expect("Unable to initialize local database from existing data/sappyl.db");
        }

        let db = Database::connect("sqlite://data/seppyl.local.db?mode=rwc")
            .await
            .unwrap();

        db.get_schema_registry("api::entities::*")
            .sync(&db)
            .await
            .unwrap();

        let user = entities::user::Entity::find()
            .filter(entities::user::COLUMN.username.eq("admin".to_string()))
            .one(&db)
            .await
            .unwrap();

        if user.is_none() {
            let user = entities::user::ActiveModel {
                id: Set(uuid::Uuid::new_v4()),
                username: Set("admin".into()),
                password: Set("$argon2id$v=19$m=19456,t=2,p=1$ko1RJ3LS7gXHMPZ8wI31QA$bEB7xXacYbfJzHuchGcSLK62pxQ7QOvgy7ikRaLgvO4".into()),
                ..Default::default()
            };
            user.insert(&db).await.unwrap();
        }

        Self { db }
    }
}
