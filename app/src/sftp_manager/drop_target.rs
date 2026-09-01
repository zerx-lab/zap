//! SFTP 浏览器拖拽目标 Element，拦截 OS 级文件拖拽事件。
//!
//! 仿照 `terminal_size_element.rs` 实现，在 `dispatch_event` 中
//! 捕获 `DragFiles` / `DragFileExit` / `DragAndDropFiles` 事件，
//! 转发为 `SftpBrowserAction`。
//! author: logic
//! date: 2026-05-27

use std::any::Any;
use std::path::PathBuf;

use warpui::{
    elements::Point, event::DispatchedEvent, geometry::vector::Vector2F, AfterLayoutContext,
    AppContext, Element, Event, EventContext, LayoutContext, PaintContext, SizeConstraint,
};

use super::browser::SftpBrowserAction;

/// SFTP 拖拽目标 Element
pub struct SftpDropTargetElement {
    child: Box<dyn Element>,
}

impl SftpDropTargetElement {
    /// 创建拖拽目标 Element
    pub fn new(child: Box<dyn Element>) -> Self {
        Self { child }
    }
}

impl Element for SftpDropTargetElement {
    fn layout(
        &mut self,
        constraint: SizeConstraint,
        ctx: &mut LayoutContext,
        app: &AppContext,
    ) -> Vector2F {
        self.child.layout(constraint, ctx, app)
    }

    fn after_layout(&mut self, ctx: &mut AfterLayoutContext, app: &AppContext) {
        self.child.after_layout(ctx, app);
    }

    fn paint(&mut self, origin: Vector2F, ctx: &mut PaintContext, app: &AppContext) {
        self.child.paint(origin, ctx, app)
    }

    fn size(&self) -> Option<Vector2F> {
        self.child.size()
    }

    fn origin(&self) -> Option<Point> {
        self.child.origin()
    }

    fn bounds(&self) -> Option<warpui::geometry::rect::RectF> {
        self.child.bounds()
    }

    fn parent_data(&self) -> Option<&dyn Any> {
        self.child.parent_data()
    }

    fn dispatch_event(
        &mut self,
        event: &DispatchedEvent,
        ctx: &mut EventContext,
        app: &AppContext,
    ) -> bool {
        let handled_by_child = self.child.dispatch_event(event, ctx, app);

        if !handled_by_child {
            let Some(z_index) = self.z_index() else {
                return false;
            };
            if let Some(event_at_z_index) = event.at_z_index(z_index, ctx) {
                match event_at_z_index {
                    Event::DragFiles { location } => {
                        if self.mouse_position_is_in_bounds(*location) {
                            ctx.dispatch_typed_action(SftpBrowserAction::DragFilesEnter);
                        } else {
                            ctx.dispatch_typed_action(SftpBrowserAction::DragFilesLeave);
                        }
                        return true;
                    }
                    Event::DragFileExit => {
                        ctx.dispatch_typed_action(SftpBrowserAction::DragFilesLeave);
                        return true;
                    }
                    Event::DragAndDropFiles { paths, location } => {
                        if self.mouse_position_is_in_bounds(*location) && !paths.is_empty() {
                            let paths: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
                            ctx.dispatch_typed_action(SftpBrowserAction::DragAndDropFiles(paths));
                        }
                        return true;
                    }
                    _ => {}
                };
            }
        }
        handled_by_child
    }
}

impl SftpDropTargetElement {
    /// 判断鼠标位置是否在 Element 边界内
    fn mouse_position_is_in_bounds(&self, position: Vector2F) -> bool {
        let Some(bounds) = self.bounds() else {
            return false;
        };
        bounds.contains_point(position)
    }
}
