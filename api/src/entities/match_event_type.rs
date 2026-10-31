use sea_orm::entity::prelude::*;
use sea_orm::DeriveEntityModel;
use serde::Serialize;
#[sea_orm::model]
#[derive(Clone,Debug,DeriveEntityModel,Serialize)]
#[sea_orm(table_name="match_event_types")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id:Uuid,
    pub club_id:Uuid,
    pub label:String,
    pub kind:String,
}
impl ActiveModelBehavior for ActiveModel {}
