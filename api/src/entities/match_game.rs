use sea_orm::entity::prelude::*;
use sea_orm::DeriveEntityModel;
use serde::Serialize;
#[sea_orm::model]
#[derive(Clone,Debug,DeriveEntityModel,Serialize)]
#[sea_orm(table_name="match_games")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id:Uuid,
    pub team_id:Uuid,
    pub created_by:Uuid,
    pub opponent:String,
    pub date:DateTime,
    pub half_minutes:i32,
    pub notes:String,
}
impl ActiveModelBehavior for ActiveModel {}
