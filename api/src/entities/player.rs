use sea_orm::DeriveEntityModel;
use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "players")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub firstname: String,
    pub lastname: String,
    pub number: u32,

    #[sea_orm(has_many)]
    pub users: HasMany<super::user::Entity>,

    #[sea_orm(has_many)]
    pub activities: HasMany<super::activity::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
