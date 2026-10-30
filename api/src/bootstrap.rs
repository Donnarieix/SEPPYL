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

    let marker=Path::new("data/.maintainer-bootstrapped-v1");
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
