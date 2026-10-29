use sea_orm::entity::prelude::*;
use sea_orm::DeriveEntityModel;
use serde::Serialize;
#[sea_orm::model]
#[derive(Clone, Debug, DeriveEntityModel, Serialize)]
#[sea_orm(table_name="audit_events")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub actor_id: Option<Uuid>,
    pub action: String,
    pub resource: String,
    pub status: i32,
    pub occurred_at: DateTime,
}
impl ActiveModelBehavior for ActiveModel {}
