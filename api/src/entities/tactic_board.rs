use sea_orm::entity::prelude::*;
use sea_orm::DeriveEntityModel;
use serde::Serialize;
#[sea_orm::model]
#[derive(Clone,Debug,DeriveEntityModel,Serialize)]
#[sea_orm(table_name="tactic_boards")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id:Uuid,
    pub owner_id:Uuid,
    pub folder_id:Option<Uuid>,
    pub name:String,
    pub content:String,
    pub is_public:bool,
}
impl ActiveModelBehavior for ActiveModel {}
