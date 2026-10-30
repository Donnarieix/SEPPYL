use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectOptions, Database, DatabaseConnection, EntityTrait, QueryFilter, TransactionTrait,
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

        let url = "sqlite://data/seppyl.local.db?mode=rwc";
        // Run SQLite table rebuilds on a dedicated single connection so
        // connection-local foreign-key PRAGMAs cannot leak across pool workers.
        let mut migration_options=ConnectOptions::new(url);
        migration_options.max_connections(1);
        let migration_db=Database::connect(migration_options).await
            .expect("Unable to connect for database migration");
        crate::migrations::migrate_legacy_player_firstname(
            &migration_db,Some("data/seppyl.local.db-before-player-migration")
        )
            .await.expect("Unable to safely migrate the players table; restore the backup and inspect SQLite schema");
        crate::migrations::add_club_information(&migration_db).await
            .expect("Unable to migrate club information columns");
        crate::migrations::add_team_information(&migration_db).await
            .expect("Unable to migrate team information columns");
        crate::migrations::add_changelog_fields(&migration_db).await
            .expect("Unable to migrate changelog information columns");
        migration_db.close().await
            .expect("Unable to close migration connection");

        let db = Database::connect(url)
            .await
            .unwrap();

        db.get_schema_registry("api::entities::*")
            .sync(&db)
            .await
            .unwrap();
        // Enforce the complete player identity, never firstname alone.
        // A pre-existing conflict aborts startup rather than silently merging
        // distinct player IDs, team memberships, scores or attendance.
        crate::migrations::enforce_player_identity(&db).await
            .expect("Duplicate player identities in the existing database: resolve them on a backup before startup");


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

        crate::routes::miscellaneous::changelog::seed(&db).await
            .expect("Unable to seed changelog history");
        import_previous_training_activities(&db).await
            .expect("Unable to migrate previously created training activities");

        if let Ok(admin_username) = std::env::var("SEPPYL_BOOTSTRAP_ADMIN_USERNAME") {
            use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, QueryFilter};
            if let Some(existing) = crate::entities::user::Entity::find()
                .filter(crate::entities::user::COLUMN.username.eq(admin_username))
                .one(&db).await.expect("Unable to load bootstrap admin")
            {
                if existing.role.as_deref()!=Some("administrator")
                    && existing.role.as_deref()!=Some("maintainer") {
                    let mut active:crate::entities::user::ActiveModel = existing.into();
                    active.role = Set(Some("administrator".into()));
                    active.update(&db).await.expect("Unable to bootstrap administrator");
                    tracing::warn!("Elevated bootstrap account to administrator");
                }
            } else {
                tracing::warn!("Bootstrap username was not found; no administrator elevated");
            }
        }

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

