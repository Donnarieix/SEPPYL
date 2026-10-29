use sea_orm::entity::prelude::*;
use sea_orm::DeriveEntityModel;
use serde::Serialize;

#[sea_orm::model]
#[derive(Clone, Debug, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "training_activities")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub training_id: Uuid,
    // Nullable, so pre-existing training activities remain valid.
    pub catalog_id: Option<Uuid>,
    pub name: String,
    pub description: String,
    pub image_url: Option<String>,
    pub sort_order: i32,
}
impl ActiveModelBehavior for ActiveModel {}
