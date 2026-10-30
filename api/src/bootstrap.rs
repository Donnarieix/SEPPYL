//! Optional, one-time production owner initialization.
//! The password hash is injected into the API build by a GitHub Actions secret.
//! No default login, plaintext password or reusable setup HTTP endpoint is shipped.
use std::{fs::OpenOptions, io::Write, path::Path};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait, QueryFilter};
use crate::entities::user;

/// Intentionally does nothing when the build contains no bootstrap secret.
/// The deployment workflow rejects a missing secret before transferring artifacts.
pub async fn initialize_if_configured(db: &DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    let Some(hash) = option_env!("SEPPYL_BOOTSTRAP_MAINTAINER_PASSWORD_HASH") else {
        return Ok(());
    };
    if !hash.starts_with("$argon2id$") {
        return Err("Invalid bootstrap hash format: Argon2id required".into());
    }
    argon2::PasswordHash::new(hash)
        .map_err(|_| "Malformed bootstrap password hash")?;

    provision_once(db, hash, Path::new("data/.maintainer-bootstrapped-v1")).await
}

/// Provision exactly once. The marker is persistent across API releases, and an
/// existing maintainer also prevents new privileged identities after a crash.
async fn provision_once(
    db: &DatabaseConnection,
    hash: &str,
    marker: &Path,
) -> Result<(), Box<dyn std::error::Error>> {

    if marker.exists() { return Ok(()); }

    let current=user::Entity::find()
        .filter(user::Column::Role.eq("maintainer"))
        .one(db).await?;
    if current.is_some() {
        // A preexisting maintainer wins. Never create a second privileged account.
        persist_marker(marker)?;
        return Ok(());
    }
    if user::Entity::find()
        .filter(user::Column::Username.eq("admin"))
        .one(db).await?.is_some() {
        return Err("Bootstrap refused: the admin username is already in use".into());
    }

    user::ActiveModel {
        id: Set(uuid::Uuid::new_v4()),
        username: Set("admin".to_owned()),
        password: Set(hash.to_owned()),
        role: Set(Some("maintainer".to_owned())),
        ..Default::default()
    }.insert(db).await?;
    // On crash between insert and marker, the existing-maintainer check on
    // the next startup prevents a duplicate account.
    persist_marker(marker)?;
    tracing::warn!("Initial maintainer provisioned once; rotate its credentials and remove bootstrap code");
    Ok(())
}

fn persist_marker(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut file=OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(b"Maintainer initialized; do not delete.\n")?;
    file.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;
    // Fixed PHC hash used only by tests, never accepted as production credentials.
    const TEST_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$ko1RJ3LS7gXHMPZ8wI31QA$bEB7xXacYbfJzHuchGcSLK62pxQ7QOvgy7ikRaLgvO4";

    async fn database() -> DatabaseConnection {
        let db=Database::connect("sqlite::memory:").await.unwrap();
        db.get_schema_registry("api::entities::*").sync(&db).await.unwrap();
        db
    }

    #[tokio::test]
    async fn bootstrap_runs_once_even_after_account_rename() {
        let db=database().await;
        let dir=std::env::temp_dir().join(format!("seppyl-bootstrap-test-{}",uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let marker=dir.join("done");
        provision_once(&db,TEST_HASH,&marker).await.unwrap();
        let first=user::Entity::find().one(&db).await.unwrap().unwrap();
        assert_eq!(first.username,"admin");
        assert_eq!(first.role.as_deref(),Some("maintainer"));
        let mut renamed:user::ActiveModel=first.into();
        renamed.username=Set("my-real-account".into());
        renamed.update(&db).await.unwrap();
        provision_once(&db,TEST_HASH,&marker).await.unwrap();
        assert_eq!(user::Entity::find().all(&db).await.unwrap().len(),1);
        assert!(marker.exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn regular_account_cannot_be_silently_promoted() {
        let db=database().await;
        user::ActiveModel{
            id:Set(uuid::Uuid::new_v4()),
            username:Set("admin".into()),
            password:Set("unrelated-existing-hash".into()),
            role:Set(Some("user".into())),
            ..Default::default()
        }.insert(&db).await.unwrap();
        let marker=std::env::temp_dir().join(format!("seppyl-not-created-{}",uuid::Uuid::new_v4()));
        assert!(provision_once(&db,TEST_HASH,&marker).await.is_err());
        assert!(!marker.exists());
        let existing=user::Entity::find().one(&db).await.unwrap().unwrap();
        assert_eq!(existing.role.as_deref(),Some("user"));
    }
}
