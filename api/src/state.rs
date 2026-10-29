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
        // Keep production data stable across deployments.
        std::fs::create_dir_all("data").expect("Unable to create persistent data directory");
        let db = Database::connect("sqlite://data/sappyl.db?mode=rwc")
            .await
            .expect("Cannot open persistent SEPPYL database");
        db.get_schema_registry("api::entities::*")
            .sync(&db)
            .await
            .expect("Cannot synchronize database schema");

        // No default administrator or embedded password hash.
        import_previous_training_activities(&db)
            .await
            .expect("Cannot migrate existing training activities");
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

