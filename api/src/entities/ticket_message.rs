use sea_orm::entity::prelude::*;
use sea_orm::DeriveEntityModel;
use serde::Serialize;
#[sea_orm::model]
#[derive(Clone,Debug,DeriveEntityModel,Serialize)]
#[sea_orm(table_name="support_ticket_messages")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id:Uuid,
    pub ticket_id:Uuid,
    pub author_id:Uuid,
    pub message:String,
    // Nullable for older messages and new image-only messages.
    pub attachment_name:Option<String>,
    pub created_at:i64,
    pub edited_at:Option<i64>,
    pub deleted_at:Option<i64>,
}
impl ActiveModelBehavior for ActiveModel {}
