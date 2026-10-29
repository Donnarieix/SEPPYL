use sea_orm::entity::prelude::*;
use sea_orm::DeriveEntityModel;
use serde::Serialize;
#[sea_orm::model]
#[derive(Clone,Debug,DeriveEntityModel,Serialize)]
#[sea_orm(table_name="tactic_folders")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id:Uuid,
    pub owner_id:Uuid,
    pub parent_id:Option<Uuid>,
    pub name:String,
}
impl ActiveModelBehavior for ActiveModel {}
