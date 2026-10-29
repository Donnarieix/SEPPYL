//! Built-in tactical board, intentionally independent of the easy2coach app.
//! All drawings are private unless explicitly published by their owner.
use axum::{Extension,Json,extract::{Path,State},http::StatusCode};
use sea_orm::{ActiveModelTrait,ActiveValue::Set,EntityTrait,QueryFilter};
use serde::{Deserialize,Serialize};
use uuid::Uuid;
use crate::{entities::{tactic_board,tactic_folder},routes::authentication::login::Claims,state::AppState,access::{self,Role}};

#[derive(Serialize)]
pub struct Library {pub folders:Vec<tactic_folder::Model>,pub boards:Vec<tactic_board::Model>}
#[derive(Deserialize)]
pub struct FolderInput {pub name:String,pub parent_id:Option<Uuid>}
#[derive(Deserialize)]
pub struct BoardInput {pub name:String,pub folder_id:Option<Uuid>,pub content:String,pub is_public:bool}
#[derive(Serialize,Deserialize)]
pub struct Diagram {pub pitch:String,pub markers:Vec<Marker>,pub arrows:Vec<Arrow>}
#[derive(Serialize,Deserialize)]
pub struct Marker {pub kind:String,pub x:f64,pub y:f64}
#[derive(Serialize,Deserialize)]
pub struct Arrow {pub x1:f64,pub y1:f64,pub x2:f64,pub y2:f64}

fn valid_name(s:&str)->bool{!s.trim().is_empty()&&s.len()<=150}
fn valid_coord(n:f64)->bool{n.is_finite()&&(0.0..=1.0).contains(&n)}
fn validated_content(content:&str)->Result<String,StatusCode>{
    if content.len()>100_000{return Err(StatusCode::PAYLOAD_TOO_LARGE)}
    let diagram:Diagram=serde_json::from_str(content).map_err(|_|StatusCode::BAD_REQUEST)?;
    if !["football","half_pitch","blank"].contains(&diagram.pitch.as_str()) ||
        diagram.markers.len()>250 || diagram.arrows.len()>250 {return Err(StatusCode::BAD_REQUEST)}
    if diagram.markers.iter().any(|m|!["player","opponent","ball","cone"].contains(&m.kind.as_str()) ||
        !valid_coord(m.x)||!valid_coord(m.y)) ||
       diagram.arrows.iter().any(|a| !valid_coord(a.x1)||!valid_coord(a.y1)||!valid_coord(a.x2)||!valid_coord(a.y2)){
        return Err(StatusCode::BAD_REQUEST)
    }
    serde_json::to_string(&diagram).map_err(|_|StatusCode::BAD_REQUEST)
}
async fn own_board(state:&AppState,user:Uuid,id:Uuid)->Result<tactic_board::Model,StatusCode>{
    let board=tactic_board::Entity::find_by_id(id).one(&state.db).await
        .map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    if board.owner_id!=user && access::effective_role(state,user).await?!=Role::Administrator{
        return Err(StatusCode::NOT_FOUND)
    }
    Ok(board)
}
async fn own_folder(state:&AppState,user:Uuid,id:Uuid)->Result<(),StatusCode>{
    let folder=tactic_folder::Entity::find_by_id(id).one(&state.db).await
        .map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if folder.owner_id!=user && access::effective_role(state,user).await?!=Role::Administrator{
        return Err(StatusCode::NOT_FOUND)
    }
    Ok(())
}
pub async fn library(
    Extension(claims):Extension<Claims>,State(state):State<AppState>
)->Result<Json<Library>,StatusCode>{
    let folders=tactic_folder::Entity::find().filter(tactic_folder::COLUMN.owner_id.eq(claims.sub))
        .all(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    let boards=tactic_board::Entity::find().filter(tactic_board::COLUMN.owner_id.eq(claims.sub))
        .all(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(Library{folders,boards}))
}
pub async fn create_folder(
    Extension(claims):Extension<Claims>,State(state):State<AppState>,Json(body):Json<FolderInput>
)->Result<(StatusCode,Json<tactic_folder::Model>),StatusCode>{
    if !valid_name(&body.name){return Err(StatusCode::BAD_REQUEST)}
    if let Some(parent)=body.parent_id{own_folder(&state,claims.sub,parent).await?}
    let saved=tactic_folder::ActiveModel{
        id:Set(Uuid::new_v4()),owner_id:Set(claims.sub),parent_id:Set(body.parent_id),
        name:Set(body.name.trim().into()),..Default::default()
    }.insert(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED,Json(saved)))
}
pub async fn create_board(
    Extension(claims):Extension<Claims>,State(state):State<AppState>,Json(body):Json<BoardInput>
)->Result<(StatusCode,Json<tactic_board::Model>),StatusCode>{
    if !valid_name(&body.name){return Err(StatusCode::BAD_REQUEST)}
    if let Some(folder)=body.folder_id{own_folder(&state,claims.sub,folder).await?}
    let content=validated_content(&body.content)?;
    let saved=tactic_board::ActiveModel{
        id:Set(Uuid::new_v4()),owner_id:Set(claims.sub),folder_id:Set(body.folder_id),
        name:Set(body.name.trim().into()),content:Set(content),
        is_public:Set(body.is_public),..Default::default()
    }.insert(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED,Json(saved)))
}
pub async fn save_board(
    Extension(claims):Extension<Claims>,State(state):State<AppState>,
    Path(id):Path<Uuid>,Json(body):Json<BoardInput>
)->Result<StatusCode,StatusCode>{
    if !valid_name(&body.name){return Err(StatusCode::BAD_REQUEST)}
    if let Some(folder)=body.folder_id{own_folder(&state,claims.sub,folder).await?}
    let content=validated_content(&body.content)?;
    let existing=own_board(&state,claims.sub,id).await?;
    let mut active:tactic_board::ActiveModel=existing.into();
    active.name=Set(body.name.trim().into());
    active.folder_id=Set(body.folder_id);
    active.content=Set(content);
    active.is_public=Set(body.is_public);
    active.update(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}
pub async fn delete_board(
    Extension(claims):Extension<Claims>,State(state):State<AppState>,Path(id):Path<Uuid>
)->Result<StatusCode,StatusCode>{
    let board=own_board(&state,claims.sub,id).await?;
    let active:tactic_board::ActiveModel=board.into();
    active.delete(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}
pub async fn public_board(
    State(state):State<AppState>,Path(id):Path<Uuid>
)->Result<Json<tactic_board::Model>,StatusCode>{
    let board=tactic_board::Entity::find_by_id(id).one(&state.db).await
        .map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    if !board.is_public{return Err(StatusCode::NOT_FOUND)}
    Ok(Json(board))
}
#[cfg(test)]
mod tests{
 use super::*;
 #[test]fn reject_untrusted_or_invalid_diagrams(){
  assert!(validated_content(r#"{"pitch":"football","markers":[{"kind":"player","x":0.4,"y":0.5}],"arrows":[]}"#).is_ok());
  assert!(validated_content(r#"{"pitch":"football","markers":[{"kind":"script","x":0.4,"y":0.5}],"arrows":[]}"#).is_err());
  assert!(validated_content(r#"{"pitch":"football","markers":[{"kind":"ball","x":42,"y":0.5}],"arrows":[]}"#).is_err());
 }
}
