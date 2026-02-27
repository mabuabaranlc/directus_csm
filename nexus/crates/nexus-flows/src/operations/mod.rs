pub mod condition;
pub mod exec;
pub mod item_create;
pub mod item_read;
pub mod item_update;
pub mod item_delete;
pub mod json_web_token;
pub mod log_op;
pub mod mail;
pub mod notification;
pub mod request;
pub mod sleep;
pub mod transform;
pub mod trigger;

use crate::manager::FlowManager;
use std::sync::Arc;

/// Register all built-in operations with the flow manager
pub async fn register_all(manager: &FlowManager) {
    manager.register_operation(Arc::new(condition::ConditionOperation)).await;
    manager.register_operation(Arc::new(exec::ExecOperation)).await;
    manager.register_operation(Arc::new(item_create::ItemCreateOperation)).await;
    manager.register_operation(Arc::new(item_read::ItemReadOperation)).await;
    manager.register_operation(Arc::new(item_update::ItemUpdateOperation)).await;
    manager.register_operation(Arc::new(item_delete::ItemDeleteOperation)).await;
    manager.register_operation(Arc::new(json_web_token::JwtOperation)).await;
    manager.register_operation(Arc::new(log_op::LogOperation)).await;
    manager.register_operation(Arc::new(mail::MailOperation)).await;
    manager.register_operation(Arc::new(notification::NotificationOperation)).await;
    manager.register_operation(Arc::new(request::RequestOperation)).await;
    manager.register_operation(Arc::new(sleep::SleepOperation)).await;
    manager.register_operation(Arc::new(transform::TransformOperation)).await;
    manager.register_operation(Arc::new(trigger::TriggerOperation)).await;
}
