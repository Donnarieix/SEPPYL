use sea_orm::entity::prelude::*;
use sea_orm::DeriveEntityModel;
use serde::Serialize;
#[sea_orm::model]
#[derive(Clone,Debug,DeriveEntityModel,Serialize)]
#[sea_orm(table_name="match_participants")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub match_id:Uuid,
    #[sea_orm(primary_key)]
    pub player_id:Uuid,
    pub position:String,
    pub starter:bool,
}
impl ActiveModelBehavior for ActiveModel {}
