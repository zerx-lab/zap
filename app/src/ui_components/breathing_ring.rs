//! 呼吸环 Element:在 paint 中按 elapsed 更新边框透明度,并 `repaint_after` 驱动下一帧。
//!
//! `repaint_after` 只重跑已有 element 树的 layout/paint,不会调用 View::render。
//! 因此环的透明度必须在本 Element 的 paint 里计算,不能在 View 里写死到 Container。

use std::sync::{Arc, Mutex};
use std::time::Duration;

use instant::Instant;
use pathfinder_color::ColorU;
use pathfinder_geometry::{
    rect::RectF,
    vector::{vec2f, Vector2F},
};
use warpui::elements::{Border, CornerRadius, Element, Fill, Point, Radius};
use warpui::event::DispatchedEvent;
use warpui::{
    AfterLayoutContext, AppContext, EventContext, LayoutContext, PaintContext, SizeConstraint,
};

pub(crate) const BREATHING_PERIOD: Duration = Duration::from_millis(1600);
const REPAINT_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Clone)]
pub(crate) struct BreathingStateHandle(Arc<Mutex<Instant>>);

impl Default for BreathingStateHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl BreathingStateHandle {
    pub(crate) fn new() -> Self {
        Self(Arc::new(Mutex::new(Instant::now())))
    }

    pub(crate) fn elapsed(&self) -> Duration {
        self.0.lock().expect("breathing state poisoned").elapsed()
    }
}

pub(crate) fn breathing_opacity(elapsed: Duration, period: Duration) -> u8 {
    let period_secs = period.as_secs_f32().max(f32::EPSILON);
    let turns = elapsed.as_secs_f32() / period_secs;
    let wave = (1.0 - (turns * std::f32::consts::TAU).cos()) * 0.5;
    ((0.4 + 0.6 * wave) * 255.0).round() as u8
}

/// 在 child 外画圆形描边。`animate` 为 true 时每帧更新透明度并预约重绘。
pub(crate) struct BreathingRing {
    child: Box<dyn Element>,
    color: ColorU,
    border_width: f32,
    animate: bool,
    state: BreathingStateHandle,
    origin: Option<Point>,
    size: Option<Vector2F>,
}

impl BreathingRing {
    pub(crate) fn new(
        child: Box<dyn Element>,
        color: ColorU,
        border_width: f32,
        animate: bool,
        state: BreathingStateHandle,
    ) -> Self {
        Self {
            child,
            color,
            border_width,
            animate,
            state,
            origin: None,
            size: None,
        }
    }

    fn border_buffer(&self) -> Vector2F {
        vec2f(self.border_width * 2., self.border_width * 2.)
    }
}

impl Element for BreathingRing {
    fn layout(
        &mut self,
        constraint: SizeConstraint,
        ctx: &mut LayoutContext,
        app: &AppContext,
    ) -> Vector2F {
        let buffer = self.border_buffer();
        let child_constraint = SizeConstraint {
            min: (constraint.min - buffer).max(Vector2F::zero()),
            max: (constraint.max - buffer).max(Vector2F::zero()),
        };
        let child_size = self.child.layout(child_constraint, ctx, app);
        let size = child_size + buffer;
        self.size = Some(size);
        size
    }

    fn after_layout(&mut self, ctx: &mut AfterLayoutContext, app: &AppContext) {
        self.child.after_layout(ctx, app);
    }

    fn paint(&mut self, origin: Vector2F, ctx: &mut PaintContext, app: &AppContext) {
        self.origin = Some(Point::from_vec2f(origin, ctx.scene.z_index()));
        let Some(size) = self.size else {
            return;
        };
        let opacity = if self.animate {
            breathing_opacity(self.state.elapsed(), BREATHING_PERIOD)
        } else {
            255
        };
        ctx.scene
            .draw_rect_with_hit_recording(RectF::new(origin, size))
            .with_background(Fill::None)
            .with_border(
                Border::all(self.border_width).with_border_fill(ColorU::new(
                    self.color.r,
                    self.color.g,
                    self.color.b,
                    opacity,
                )),
            )
            .with_corner_radius(CornerRadius::with_all(Radius::Percentage(50.)));
        self.child.paint(
            origin + vec2f(self.border_width, self.border_width),
            ctx,
            app,
        );
        if self.animate {
            ctx.repaint_after(REPAINT_INTERVAL);
        }
    }

    fn size(&self) -> Option<Vector2F> {
        self.size
    }

    fn origin(&self) -> Option<Point> {
        self.origin
    }

    fn dispatch_event(
        &mut self,
        event: &DispatchedEvent,
        ctx: &mut EventContext,
        app: &AppContext,
    ) -> bool {
        self.child.dispatch_event(event, ctx, app)
    }
}

#[cfg(test)]
#[path = "breathing_ring_tests.rs"]
mod tests;
