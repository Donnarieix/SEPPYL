//! Per-account, per-message read receipts. Independent of timestamps or UUID ordering.
use sea_orm::entity::prelude::*;
use sea_orm::DeriveEntityModel;
#[sea_orm::model]
#[derive(Clone,Debug,DeriveEntityModel)]
#[sea_orm(table_name="support_ticket_reads")]
pub struct Model {
    #[sea_orm(primary_key,auto_increment=false)]
    pub user_id:Uuid,
    #[sea_orm(primary_key,auto_increment=false)]
    pub message_id:Uuid,
    pub ticket_id:Uuid,
}
impl ActiveModelBehavior for ActiveModel {}
