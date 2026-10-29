use sea_orm::entity::prelude::*;
use sea_orm::DeriveEntityModel;
use serde::Serialize;

#[sea_orm::model]
#[derive(Clone, Debug, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "catalog_materials")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub activity_id: Uuid,
    pub name: String,
    pub quantity: i32,
}
impl ActiveModelBehavior for ActiveModel {}
