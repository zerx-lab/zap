use warp_graphql::billing::{AddonCreditsOption, PlanPricing, PricingInfo, StripeSubscriptionPlan};
use warpui::{Entity, ModelContext, SingletonEntity};

/// A global model for pricing information from the server.
///
/// In OpenWarp this is effectively a no-op stub: the OSS channel has no
/// cloud server pushing pricing data, so `pricing_info` is normally `None`
/// for the lifetime of the process and every getter returns `None`. The
/// model is preserved only because consumer call sites (request_usage,
/// billing-aware modals, teams settings page) still reference it;
/// downstream cloud-removal phases will eventually retire those call sites
/// and let us delete this entirely.
#[derive(Debug)]
pub struct PricingInfoModel {
    pricing_info: Option<PricingInfo>,
}

impl PricingInfoModel {
    pub fn new() -> Self {
        Self { pricing_info: None }
    }

    /// Updates the model with the latest pricing information from the server.
    pub fn update_pricing_info(&mut self, pricing_info: PricingInfo, ctx: &mut ModelContext<Self>) {
        self.pricing_info = Some(pricing_info);
        ctx.emit(PricingInfoModelEvent::PricingInfoUpdated);
    }

    /// Returns the pricing for a specific plan.
    pub fn plan_pricing(&self, plan: &StripeSubscriptionPlan) -> Option<&PlanPricing> {
        self.pricing_info
            .as_ref()?
            .plans
            .iter()
            .find(|p| &p.plan == plan)
    }

    pub fn addon_credits_options(&self) -> Option<&[AddonCreditsOption]> {
        self.pricing_info
            .as_ref()
            .map(|info| info.addon_credits_options.as_slice())
    }
}

impl Default for PricingInfoModel {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub enum PricingInfoModelEvent {
    PricingInfoUpdated,
}

impl Entity for PricingInfoModel {
    type Event = PricingInfoModelEvent;
}

impl SingletonEntity for PricingInfoModel {}
