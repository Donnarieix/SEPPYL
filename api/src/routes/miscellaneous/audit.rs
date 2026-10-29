use axum::{Extension,Json,extract::{State,Query},http::StatusCode};
use sea_orm::{EntityTrait,QueryOrder,PaginatorTrait,QuerySelect};
use serde::{Deserialize,Serialize};
use crate::{entities::audit_event,middleware::{CurrentRole,Role},state::AppState};

#[derive(Debug,Deserialize)]
pub struct AuditQuery{
    #[serde(default="default_page")]
    page:u64,
    #[serde(default="default_size")]
    page_size:u64,
}
fn default_page()->u64{1}
fn default_size()->u64{50}
fn accepted_size(value:u64)->bool{matches!(value,10|50|100|250|500|1000)}
#[derive(Serialize)]
pub struct AuditPage {
    pub items:Vec<audit_event::Model>,
    pub total:u64,
    pub page:u64,
    pub page_size:u64,
    pub total_pages:u64,
}
pub async fn list(
    Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,
    Query(query):Query<AuditQuery>,
)->Result<Json<AuditPage>,StatusCode>{
    if role.0!=Role::Administrator{return Err(StatusCode::FORBIDDEN);}
    if query.page==0 || !accepted_size(query.page_size){return Err(StatusCode::BAD_REQUEST);}
    let total=audit_event::Entity::find().count(&state.db).await
        .map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    let total_pages=if total==0{0}else{1+(total-1)/query.page_size};
    // Never multiply untrusted page numbers before checking the bounds.
    let items=if query.page>total_pages{
        Vec::new()
    }else{
        audit_event::Entity::find()
            .order_by_desc(audit_event::COLUMN.timestamp)
            .order_by_desc(audit_event::COLUMN.id)
            .offset((query.page-1)*query.page_size)
            .limit(query.page_size)
            .all(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?
    };
    Ok(Json(AuditPage{items,total,page:query.page,page_size:query.page_size,total_pages}))
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn only_supported_page_sizes(){
        for n in [10,50,100,250,500,1000]{assert!(accepted_size(n));}
        for n in [0,1,25,1001,u64::MAX]{assert!(!accepted_size(n));}
    }
}
