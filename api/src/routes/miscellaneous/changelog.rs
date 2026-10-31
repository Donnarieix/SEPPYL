use axum::{Extension,Json,extract::{Path,State},http::StatusCode};
use sea_orm::{ActiveModelTrait,ActiveValue::Set,EntityTrait,QueryOrder,QueryFilter,ColumnTrait};
use serde::{Deserialize,Serialize};
use uuid::Uuid;
use crate::{entities::{changelog_entry,changelog_read},
    middleware::{CurrentRole,Role},
    routes::authentication::login::Claims,state::AppState};

/// The deployed application's release. Adding/editing historical entries is
/// independent of deploying the client and does not mark them as unread again.
pub const CURRENT_VERSION:&str=concat!("v",env!("CARGO_PKG_VERSION"));
fn now()->i64{
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .map(|v|v.as_secs() as i64).unwrap_or(0)
}
fn database_error(error:sea_orm::DbErr)->StatusCode{
    tracing::error!(%error,"Changelog database operation failed");
    StatusCode::INTERNAL_SERVER_ERROR
}
#[derive(Clone,Debug,Serialize)]
pub struct Changelog{
    pub version:String,pub title:String,pub body:String,
    pub body_html:Option<String>,pub is_hidden:bool,
    pub published_at:i64,pub updated_at:i64,
}
impl From<changelog_entry::Model> for Changelog{
    fn from(row:changelog_entry::Model)->Self{
        Self{version:row.version,title:row.title,body:row.body,
            body_html:row.body_html,is_hidden:row.is_hidden.unwrap_or(false),
            published_at:row.published_at,updated_at:row.updated_at}
    }
}
#[derive(Serialize)]
pub struct UnreadStatus{pub version:&'static str,pub unread:bool}

pub async fn list(
    Extension(role):Extension<CurrentRole>,State(state):State<AppState>
)->Result<Json<Vec<Changelog>>,StatusCode>{
    let rows=changelog_entry::Entity::find()
        .order_by_desc(changelog_entry::COLUMN.published_at)
        .all(&state.db).await.map_err(database_error)?;
    Ok(Json(rows.into_iter()
        .filter(|row|role.0==Role::Maintainer || !row.is_hidden.unwrap_or(false))
        .map(Into::into).collect()))
}
pub async fn status(
    Extension(claims):Extension<Claims>,State(state):State<AppState>
)->Result<Json<UnreadStatus>,StatusCode>{
    // A draft is never announced, even to its author; maintainers can
    // review drafts manually in the history until they publish.
    let exists=changelog_entry::Entity::find_by_id(CURRENT_VERSION.to_string())
        .one(&state.db).await.map_err(database_error)?
        .is_some_and(|entry|!entry.is_hidden.unwrap_or(false));
    let read=changelog_read::Entity::find_by_id((claims.sub,CURRENT_VERSION.to_string()))
        .one(&state.db).await.map_err(database_error)?.is_some();
    Ok(Json(UnreadStatus{version:CURRENT_VERSION,unread:exists&&!read}))
}
/// Idempotent per-user acknowledgement; cannot mark another user's read state.
pub async fn acknowledge(
    Extension(claims):Extension<Claims>,Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path(version):Path<String>,
)->Result<StatusCode,StatusCode>{
    let entry=changelog_entry::Entity::find_by_id(version.clone())
        .one(&state.db).await.map_err(database_error)?
        .ok_or(StatusCode::NOT_FOUND)?;
    if entry.is_hidden.unwrap_or(false) && role.0!=Role::Maintainer {
        return Err(StatusCode::NOT_FOUND);
    }
    if changelog_read::Entity::find_by_id((claims.sub,version.clone()))
        .one(&state.db).await.map_err(database_error)?.is_none(){
        let result=changelog_read::ActiveModel{
            user_id:Set(claims.sub),version:Set(version),read_at:Set(now()),
            ..Default::default()
        }.insert(&state.db).await;
        if let Err(error)=result {
            // Concurrent acknowledgements are harmless; other failures are not.
            if !matches!(error.sql_err(),
                Some(sea_orm::SqlErr::UniqueConstraintViolation(_))){
                return Err(database_error(error));
            }
        }
    }
    Ok(StatusCode::NO_CONTENT)
}
#[derive(Deserialize)]
pub struct EditChangelog{
    pub title:String,pub body:String,
    pub body_html:Option<String>,pub is_hidden:bool,
}
pub async fn save(
    Extension(role):Extension<CurrentRole>,State(state):State<AppState>,
    Path(version):Path<String>,Json(payload):Json<EditChangelog>,
)->Result<Json<Changelog>,StatusCode>{
    if role.0!=Role::Maintainer{return Err(StatusCode::FORBIDDEN);}
    if version.len()>32 || version.is_empty() ||
        !version.chars().all(|c|c.is_ascii_alphanumeric()||matches!(c,'.'|'-'|'_')) ||
        payload.title.trim().is_empty()||payload.title.len()>150||
        payload.body.trim().is_empty()||payload.body.len()>30_000 ||
        payload.body_html.as_ref().is_some_and(|html|html.len()>40_000) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let timestamp=now();
    // Treat all rich HTML as untrusted, just as the activity editor does.
    let cleaned_html=payload.body_html.as_deref().map(ammonia::clean);
    let saved=match changelog_entry::Entity::find_by_id(version.clone())
        .one(&state.db).await.map_err(database_error)?{
        Some(existing)=>{
            let mut active:changelog_entry::ActiveModel=existing.into();
            active.title=Set(payload.title.trim().to_string());
            active.body=Set(payload.body.trim().to_string());
            active.body_html=Set(cleaned_html.clone());
            active.is_hidden=Set(Some(payload.is_hidden));
            active.updated_at=Set(timestamp);
            active.update(&state.db).await.map_err(database_error)?
        }
        None=>{
            changelog_entry::ActiveModel{
                version:Set(version),title:Set(payload.title.trim().to_string()),
                body:Set(payload.body.trim().to_string()),
                body_html:Set(cleaned_html),is_hidden:Set(Some(payload.is_hidden)),
                published_at:Set(timestamp),updated_at:Set(timestamp),
                ..Default::default()
            }.insert(&state.db).await.map_err(database_error)?
        }
    };
    Ok(Json(saved.into()))
}
#[cfg(test)]
mod tests{
    use super::*;
    use sea_orm::Database;
    #[tokio::test]
    async fn each_account_sees_release_only_once_but_history_remains_visible(){
        let db=Database::connect("sqlite::memory:").await.unwrap();
        db.get_schema_registry("api::entities::*").sync(&db).await.unwrap();
        let state=AppState{db};
        seed(&state.db).await.unwrap();
        let a=Claims{sub:Uuid::new_v4(),exp:u64::MAX};
        let b=Claims{sub:Uuid::new_v4(),exp:u64::MAX};
        assert_eq!(list(Extension(CurrentRole(Role::Trainer)),State(state.clone())).await.unwrap().0.len(),3);
        assert!(status(Extension(a.clone()),State(state.clone())).await.unwrap().0.unread);
        acknowledge(Extension(a.clone()),Extension(CurrentRole(Role::Trainer)),
            State(state.clone()),Path(CURRENT_VERSION.into())).await.unwrap();
        acknowledge(Extension(a.clone()),Extension(CurrentRole(Role::Trainer)),
            State(state.clone()),Path(CURRENT_VERSION.into())).await.unwrap();
        assert!(!status(Extension(a),State(state.clone())).await.unwrap().0.unread);
        assert!(status(Extension(b),State(state.clone())).await.unwrap().0.unread);
        assert_eq!(list(Extension(CurrentRole(Role::Trainer)),State(state)).await.unwrap().0.len(),3);
    }
    #[tokio::test]
    async fn hidden_releases_are_private_to_maintainers_and_not_announced(){
        let db=Database::connect("sqlite::memory:").await.unwrap();
        db.get_schema_registry("api::entities::*").sync(&db).await.unwrap();
        let state=AppState{db};
        seed(&state.db).await.unwrap();
        let draft=EditChangelog{
            title:"Brouillon".into(),body:"Prochaine mise à jour".into(),
            body_html:Some("<h3>Prochaine mise à jour</h3>".into()),is_hidden:true,
        };
        save(Extension(CurrentRole(Role::Maintainer)),State(state.clone()),
            Path(CURRENT_VERSION.into()),Json(draft)).await.unwrap();
        assert_eq!(list(Extension(CurrentRole(Role::Trainer)),
            State(state.clone())).await.unwrap().0.len(),2);
        let maintainer=list(Extension(CurrentRole(Role::Maintainer)),
            State(state.clone())).await.unwrap().0;
        assert_eq!(maintainer.len(),3);
        assert!(maintainer.iter().any(|entry|entry.version==CURRENT_VERSION && entry.is_hidden));
        let trainer=Claims{sub:Uuid::new_v4(),exp:u64::MAX};
        assert!(!status(Extension(trainer.clone()),State(state.clone())).await.unwrap().0.unread);
        assert_eq!(acknowledge(Extension(trainer),Extension(CurrentRole(Role::Trainer)),
            State(state.clone()),Path(CURRENT_VERSION.into())).await.err(),
            Some(StatusCode::NOT_FOUND));
        save(Extension(CurrentRole(Role::Maintainer)),State(state.clone()),
            Path(CURRENT_VERSION.into()),Json(EditChangelog{
                title:"Publiée".into(),body:"Disponible".into(),
                body_html:None,is_hidden:false,
            })).await.unwrap();
        assert_eq!(list(Extension(CurrentRole(Role::Trainer)),
            State(state.clone())).await.unwrap().0.len(),3);
        assert!(status(Extension(Claims{sub:Uuid::new_v4(),exp:u64::MAX}),
            State(state)).await.unwrap().0.unread);
    }

    #[tokio::test]
    async fn only_maintainer_can_edit_and_edits_do_not_reprompt(){
        let db=Database::connect("sqlite::memory:").await.unwrap();
        db.get_schema_registry("api::entities::*").sync(&db).await.unwrap();
        let state=AppState{db};
        seed(&state.db).await.unwrap();
        let claims=Claims{sub:Uuid::new_v4(),exp:u64::MAX};
        acknowledge(Extension(claims.clone()),Extension(CurrentRole(Role::Trainer)),
            State(state.clone()),Path(CURRENT_VERSION.into())).await.unwrap();
        let edit=||Json(EditChangelog{
            title:"Corrigé".into(),body:"Changelog corrigé".into(),
            body_html:Some("<b>Corrigé</b><script>alert(1)</script>".into()),is_hidden:false,
        });
        assert_eq!(save(Extension(CurrentRole(Role::Administrator)),State(state.clone()),
            Path(CURRENT_VERSION.into()),edit()).await.err(),Some(StatusCode::FORBIDDEN));
        let updated=save(Extension(CurrentRole(Role::Maintainer)),State(state.clone()),
            Path(CURRENT_VERSION.into()),edit()).await.unwrap().0;
        assert_eq!(updated.title,"Corrigé");
        assert!(updated.body_html.as_deref().unwrap().contains("<b>"));
        assert!(!updated.body_html.as_deref().unwrap().contains("<script>"));
        assert!(!status(Extension(claims),State(state)).await.unwrap().0.unread);
    }
}

/// Preserve human edits across deployments. Only insert missing releases.
pub async fn seed(db:&sea_orm::DatabaseConnection)->Result<(),sea_orm::DbErr>{
    const ENTRIES:[(&str,&str,&str);3]=[
      ("v0","Version initiale","Première version de SEPPYL.\n\n• Gestion des joueuses et des entraînements.\n• Présences et évaluations de 0 à 5, globales ou par activité.\n• Création d'activités et suivi du matériel.\n• Tableur historique des activités et des joueuses.\n• Fiche d'entraînement imprimable au format PDF via le navigateur.\n• Compte personnel et préférences d'affichage."),
      ("v0.1","Nouvelle interface et gestion des clubs","Cette version prépare le déploiement de SEPPYL.\n\n• Clubs, équipes, effectifs partagés et droits d'accès.\n• Administration séparée de l'utilisation courante.\n• Tableau tactique interactif, flèches, couleurs et mode plein écran.\n• Activités réutilisables et éditeur de texte enrichi.\n• Journal d'activité lisible avec détails des modifications.\n• Logos personnalisés et recadrage des images pour clubs et équipes.\n• Thèmes clair, sombre, système et contrasté.\n• Navigation responsive, sélecteurs avec recherche et notifications discrètes.\n• Historique des versions consultable dans l'application."),
      ("v0.1.1","Sécurité et fiabilité","Cette mise à jour renforce la sécurité et la fiabilité de SEPPYL.\n\n• Amélioration de la sécurité des comptes et des accès.\n• Renforcement des vérifications avant chaque mise en production.\n• Corrections de stabilité et améliorations techniques.")
    ];
    for (i,(version,title,body)) in ENTRIES.into_iter().enumerate(){
        if changelog_entry::Entity::find_by_id(version.to_string()).one(db).await?.is_none(){
            changelog_entry::ActiveModel{
                version:Set(version.to_string()),title:Set(title.to_string()),
                body:Set(body.to_string()),is_hidden:Set(Some(false)),
                body_html:Set(None),published_at:Set(1_000+i as i64),
                updated_at:Set(now()),..Default::default()
            }.insert(db).await?;
        }
    }
    Ok(())
}
