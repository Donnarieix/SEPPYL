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
    pub created_at:i64,
}
impl ActiveModelBehavior for ActiveModel {}
