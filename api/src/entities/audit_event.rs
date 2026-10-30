use sea_orm::entity::prelude::*;
use sea_orm::DeriveEntityModel;
use serde::Serialize;

#[sea_orm::model]
#[derive(Clone, Debug, DeriveEntityModel, Serialize)]
#[sea_orm(table_name="audit_events")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub actor_id: Uuid,
    pub action: String,
    pub path: String,
    pub status: i32,
    pub timestamp: i64,
    // Captured at mutation time. Older rows have no verified scope.
    pub scope: Option<String>,
    pub club_id: Option<Uuid>,
    pub team_id: Option<Uuid>,
    pub description: Option<String>,
}
impl ActiveModelBehavior for ActiveModel {}
