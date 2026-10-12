use sea_orm::DeriveEntityModel;
use sea_orm::entity::prelude::*;
use serde::Serialize;
use utoipa::ToSchema;

#[sea_orm::model]
#[derive(Clone, Debug, DeriveEntityModel, Serialize, ToSchema)]
#[sea_orm(table_name = "activities")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub name: String,
    pub date: DateTime,

    #[sea_orm(has_many, via="activity_player")]
    #[serde(skip)]
    pub players: HasMany<super::player::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
