//! 去中心化分支:不再有 referral 概念,所有 referral-gated 主题对本地用户全部开放。
//!
//! 旧版逻辑:从云端 `ReferralsClient` 拉用户的 referral 信息(发出/收到的推荐数),
//! 据此控制两个 referral-gated 主题的可见性,并把状态缓存到 user preferences。
//! 现在云端 API 已下线,模块退化为永远返回 true 的 stub,仅保留外部接口(`new` /
//! `*_referral_theme_active` / `query_referral_status`)以兼容 `lib.rs` 注册和
//! `theme_chooser` 的查询点。
//!
//! `ReferralThemeEvent` 仍保留以满足 `Entity::Event` 关联类型(workspace/view 仍订阅
//! 该事件,只是再不会被 emit) — 等下个 Decentralize Batch 把订阅链一起清理。

use std::sync::Arc;

use crate::server::server_api::referral::ReferralsClient;
use warpui::{Entity, ModelContext};

pub enum ReferralThemeEvent {
    SentReferralThemeActivated,
    ReceivedReferralThemeActivated,
}

/// 去中心化:本地永远开放全部 referral-gated 主题。
pub struct ReferralThemeStatus;

impl Entity for ReferralThemeStatus {
    type Event = ReferralThemeEvent;
}

impl ReferralThemeStatus {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self
    }

    /// 去中心化:全部主题对本地用户开放。
    pub fn sent_referral_theme_active(&self) -> bool {
        true
    }

    /// 去中心化:同上。
    pub fn received_referral_theme_active(&self) -> bool {
        true
    }

    /// 去中心化:不再向服务端查询 referral 状态。
    pub fn query_referral_status(
        &self,
        _referrals_client: Arc<dyn ReferralsClient>,
        _ctx: &mut ModelContext<Self>,
    ) {
    }
}
