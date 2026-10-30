use sea_orm::DeriveEntityModel;
use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub username: String,
    pub password: String,
    /// None preserves existing accounts as trainers during migration.
    pub role: Option<String>,

    #[sea_orm(has_many)]
    pub players: HasMany<super::player::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
