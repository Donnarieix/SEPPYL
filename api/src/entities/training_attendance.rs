use sea_orm::entity::prelude::*;
use sea_orm::DeriveEntityModel;
use serde::Serialize;

#[sea_orm::model]
#[derive(Clone, Debug, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "training_attendance")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub training_id: Uuid,
    #[sea_orm(primary_key)]
    pub player_id: Uuid,
    pub present: bool,
    pub score: Option<i32>,
    pub score_comment: Option<String>,
}
impl ActiveModelBehavior for ActiveModel {}
