use sea_orm::entity::prelude::*;
use sea_orm::DeriveEntityModel;
use serde::Serialize;
#[sea_orm::model]
#[derive(Clone, Debug, DeriveEntityModel, Serialize)]
#[sea_orm(table_name="teams")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub club_id: Uuid,
    pub name: String,
}
impl ActiveModelBehavior for ActiveModel {}
