mod event_store;

use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use parking_lot::Mutex;

use serde_json::Value;
use std::borrow::Cow;

use event_store::*;
pub use event_store::{Event, EventPayload};

lazy_static! {
    static ref TELEMETRY: Mutex<EventStore> = Mutex::new(EventStore::new());
}

#[macro_export]
macro_rules! record_telemetry_from_ctx {
    ($user_id: expr, $anonymous_id: expr, $name:expr, $payload: expr, $contains_ugc: expr, $ctx: expr) => {{
        let timestamp = $crate::time::get_current_time();
        $ctx.background_executor()
            .spawn(async move {
                $crate::telemetry::record_event(
                    $user_id,
                    $anonymous_id,
                    $name,
                    $payload,
                    $contains_ugc,
                    timestamp,
                )
            })
            .detach();
    }};
}

#[macro_export]
macro_rules! record_telemetry_on_executor {
    ($user_id: expr, $anonymous_id: expr, $name:expr, $payload: expr, $contains_ugc: expr, $executor: expr) => {{
        let timestamp = $crate::time::get_current_time();
        let _ = $executor
            .spawn(async move {
                $crate::telemetry::record_event(
                    $user_id,
                    $anonymous_id,
                    $name,
                    $payload,
                    $contains_ugc,
                    timestamp,
                )
            })
            .detach();
    }};
}

/// Creates a new `Event`, but does not record it. It is up to the caller to determine when, and
/// how, the event should be recorded.
pub fn create_event(
    user_id: Option<String>,
    anonymous_id: String,
    name: Cow<'static, str>,
    payload: Option<Value>,
    contains_ugc: bool,
    timestamp: DateTime<Utc>,
) -> Event {
    let mut telemetry = TELEMETRY.lock();
    telemetry.create_event(
        user_id,
        anonymous_id,
        name,
        payload,
        contains_ugc,
        timestamp,
    )
}

// openWarp 闭源遥测剥离 P1:
// 三个 record_* 入口全部 no-op,EventStore 永远不再被填充,后续 flush 拿到的是空 Vec,
// `send_batch_messages_to_rudder` 因 messages.is_empty() 立即返回 → 929 处宏调用原地不动,
// 全链路无外发。`TELEMETRY` lazy_static / `EventStore` 暂留死代码,P4 物理清理。
pub fn record_event(
    _user_id: Option<String>,
    _anonymous_id: String,
    _name: Cow<'static, str>,
    _payload: Option<Value>,
    _contains_ugc: bool,
    _timestamp: DateTime<Utc>,
) {
}

pub fn record_identify_user_event(
    _user_id: String,
    _anonymous_id: String,
    _timestamp: DateTime<Utc>,
) {
}

/// Adds a 'App Active' event to the global event queue.  This should only be called in an async
/// context.
pub fn record_app_active_event(
    _user_id: Option<String>,
    _anonymous_id: String,
    _timestamp: DateTime<Utc>,
) {
}

pub fn flush_events() -> Vec<Event> {
    TELEMETRY.lock().events.drain(..).collect()
}
