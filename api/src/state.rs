use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, Database, DatabaseConnection, EntityTrait, QueryFilter, TransactionTrait,
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

        import_previous_training_activities(&db).await
            .expect("Unable to migrate previously created training activities");

        Self { db }
    }
}

/// Idempotent import: the existing training activity keeps its ID and scores,
/// while its shared catalog definition and material are created exactly once.
async fn import_previous_training_activities(
    db: &DatabaseConnection,
) -> Result<(), sea_orm::DbErr> {
    use crate::entities::{catalog_activity, catalog_material, training, training_activity, training_material};
    let old = training_activity::Entity::find()
        .filter(training_activity::COLUMN.catalog_id.is_null())
        .all(db).await?;
    for item in old {
        let Some(session) = training::Entity::find_by_id(item.training_id).one(db).await? else {
            continue;
        };
        let txn = db.begin().await?;
        let def = catalog_activity::ActiveModel {
            id: Set(uuid::Uuid::new_v4()),
            user_id: Set(session.user_id),
            name: Set(item.name.clone()),
            description: Set(item.description.clone()),
            image_url: Set(item.image_url.clone()),
            ..Default::default()
        }.insert(&txn).await?;
        let equipment = training_material::Entity::find()
            .filter(training_material::COLUMN.activity_id.eq(item.id))
            .all(&txn).await?;
        for material in equipment {
            catalog_material::ActiveModel {
                id: Set(uuid::Uuid::new_v4()),
                activity_id: Set(def.id),
                name: Set(material.name.clone()),
                quantity: Set(material.quantity),
                ..Default::default()
            }.insert(&txn).await?;
            let old: training_material::ActiveModel = material.into();
            old.delete(&txn).await?;
        }
        let mut updated: training_activity::ActiveModel = item.into();
        updated.catalog_id = Set(Some(def.id));
        updated.update(&txn).await?;
        txn.commit().await?;
    }
    Ok(())
}

