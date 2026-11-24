//! Per-account acknowledgment of the latest Maintainer escalation.
//! Ordinary message read receipts stay intact when a ticket is escalated.
use sea_orm::entity::prelude::*;
use sea_orm::DeriveEntityModel;

#[sea_orm::model]
#[derive(Clone, Debug, DeriveEntityModel)]
#[sea_orm(table_name="support_ticket_maintainer_reads")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment=false)]
    pub user_id: Uuid,
    #[sea_orm(primary_key, auto_increment=false)]
    pub ticket_id: Uuid,
    pub generation: i64,
}
impl ActiveModelBehavior for ActiveModel {}
