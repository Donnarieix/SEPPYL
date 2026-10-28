use sea_orm::entity::prelude::*;
use sea_orm::DeriveEntityModel;
use serde::Serialize;

#[sea_orm::model]
#[derive(Clone, Debug, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "training_scores")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub activity_id: Uuid,
    #[sea_orm(primary_key)]
    pub player_id: Uuid,
    pub score: i32,
}
impl ActiveModelBehavior for ActiveModel {}
