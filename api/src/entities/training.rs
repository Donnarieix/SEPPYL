use sea_orm::entity::prelude::*;
use sea_orm::DeriveEntityModel;
use serde::Serialize;

#[sea_orm::model]
#[derive(Clone, Debug, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "trainings")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub user_id: Uuid,
    // Nullable for all sessions created before the team membership migration.
    pub team_id: Option<Uuid>,
    pub name: String,
    pub date: DateTime,
    pub notes: String,
    /// Scheduled duration in minutes. Old sessions remain unspecified.
    pub duration_minutes: Option<i32>,
}
impl ActiveModelBehavior for ActiveModel {}
