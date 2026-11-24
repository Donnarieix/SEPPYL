use sea_orm::entity::prelude::*;
use sea_orm::DeriveEntityModel;
use serde::Serialize;
#[sea_orm::model]
#[derive(Clone,Debug,DeriveEntityModel,Serialize)]
#[sea_orm(table_name="match_events")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id:Uuid,
    pub match_id:Uuid,
    pub event_type_id:Option<Uuid>,
    pub kind:String,
    pub minute:i32,
    pub player_id:Option<Uuid>,
    pub player_in_id:Option<Uuid>,
    pub notes:String,
    pub sort_order:i32,
}
impl ActiveModelBehavior for ActiveModel {}
