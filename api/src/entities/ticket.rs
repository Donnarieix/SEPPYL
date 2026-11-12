use sea_orm::entity::prelude::*;
use sea_orm::DeriveEntityModel;
use serde::Serialize;
#[sea_orm::model]
#[derive(Clone,Debug,DeriveEntityModel,Serialize)]
#[sea_orm(table_name="support_tickets")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id:Uuid,
    pub author_id:Uuid,
    pub subject:String,
    pub status:String,
    pub maintainer_required:bool,
    pub created_at:i64,
    pub updated_at:i64,
}
impl ActiveModelBehavior for ActiveModel {}
