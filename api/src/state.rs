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

        migrate_existing_accounts(&db).await
            .expect("Unable to assign legacy local team memberships");

        import_previous_training_activities(&db).await
            .expect("Unable to migrate previously created training activities");
        backfill_team_resources(&db).await
            .expect("Unable to backfill team scopes on existing data");

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


// Existing installations own private players/activities. Give every existing
// account an isolated default club and team, with coach permissions, before
// enforcing the new role gate. New signups get read-only user rights.
async fn migrate_existing_accounts(db:&DatabaseConnection)->Result<(),sea_orm::DbErr>{
    use crate::entities::{user,club,team,club_member,team_member};
    let accounts=user::Entity::find().all(db).await?;
    for account in accounts{
        if account.global_role.is_some(){continue} // already migrated or registered
        let mut record:user::ActiveModel=account.clone().into();
        // This is the local legacy bootstrap account. Rotate its known
        // default password immediately; production must disable bootstrap.
        if account.username=="admin"{
            record.global_role=Set(Some("administrator".into()));
            record.update(db).await?;
            continue;
        }
        let transaction=db.begin().await?;
        let private_club=club::ActiveModel{
            id:Set(uuid::Uuid::new_v4()),
            name:Set(format!("Espace de {}",account.username)),..Default::default()
        }.insert(&transaction).await?;
        let private_team=team::ActiveModel{
            id:Set(uuid::Uuid::new_v4()),club_id:Set(private_club.id),
            name:Set("Équipe principale".into()),..Default::default()
        }.insert(&transaction).await?;
        club_member::ActiveModel{
            club_id:Set(private_club.id),user_id:Set(account.id),
            role:Set("reader".into()),..Default::default()
        }.insert(&transaction).await?;
        team_member::ActiveModel{
            team_id:Set(private_team.id),user_id:Set(account.id),
            role:Set("coach".into()),..Default::default()
        }.insert(&transaction).await?;
        record.global_role=Set(Some("user".into()));
        record.update(&transaction).await?;
        transaction.commit().await?;
    }
    Ok(())
}


/// Idempotent scope backfill. No player/score is deleted, and existing UUIDs
/// remain stable so that old notes and training handouts keep working.
async fn backfill_team_resources(db:&DatabaseConnection)->Result<(),sea_orm::DbErr>{
    use crate::entities::{user,team_member,team,training,catalog_activity,player_user,team_player};
    for account in user::Entity::find().all(db).await? {
        let team_id=team_member::Entity::find()
            .filter(team_member::COLUMN.user_id.eq(account.id))
            .one(db).await?.map(|m|m.team_id);
        let team_id=if team_id.is_some(){team_id}else if account.global_role.as_deref()==Some("administrator"){
            team::Entity::find().one(db).await?.map(|t|t.id)
        }else{None};
        let Some(team_id)=team_id else{continue};
        for row in training::Entity::find()
            .filter(training::COLUMN.user_id.eq(account.id))
            .filter(training::COLUMN.team_id.is_null()).all(db).await?{
            let mut active:training::ActiveModel=row.into();
            active.team_id=Set(Some(team_id));
            active.update(db).await?;
        }
        for row in catalog_activity::Entity::find()
            .filter(catalog_activity::COLUMN.user_id.eq(account.id))
            .filter(catalog_activity::COLUMN.team_id.is_null()).all(db).await?{
            let mut active:catalog_activity::ActiveModel=row.into();
            active.team_id=Set(Some(team_id));
            active.update(db).await?;
        }
        for pu in player_user::Entity::find()
            .filter(player_user::COLUMN.user_id.eq(account.id)).all(db).await?{
            if team_player::Entity::find_by_id((team_id,pu.player_id)).one(db).await?.is_none(){
                team_player::ActiveModel{
                    team_id:Set(team_id),player_id:Set(pu.player_id),..Default::default()
                }.insert(db).await?;
            }
        }
    }
    Ok(())
}
