use sea_orm::entity::prelude::*;
use sea_orm::DeriveEntityModel;
use serde::Serialize;

#[sea_orm::model]
#[derive(Clone, Debug, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "catalog_results")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub user_id: Uuid,
    #[sea_orm(primary_key)]
    pub activity_id: Uuid,
    #[sea_orm(primary_key)]
    pub player_id: Uuid,
    pub result: Option<String>,
}
impl ActiveModelBehavior for ActiveModel {}
