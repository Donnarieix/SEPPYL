use sea_orm::entity::prelude::*;
use sea_orm::DeriveEntityModel;
use serde::Serialize;
#[sea_orm::model]
#[derive(Clone,Debug,DeriveEntityModel,Serialize)]
#[sea_orm(table_name="announcements")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id:Uuid,
    pub title:String,
    pub body:String,
    pub urgent:bool,
    pub published:bool,
    pub starts_at:i64,
    pub ends_at:Option<i64>,
    pub created_at:i64,
}
impl ActiveModelBehavior for ActiveModel {}
