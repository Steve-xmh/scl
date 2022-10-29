use druid::{
    widget::prelude::*, Data, InternalLifeCycle, Point, Rect, Scalable, Screen, Vec2, WindowHandle,
};

/// This is a wrapper widget that attempts to ensure that the widget it wraps is fully contained in
/// one monitor.
///
/// It may be useful for things like tooltips and dropdowns.
pub struct OnMonitor<W> {
    pub(crate) inner: W,
    pub(crate) parent: WindowHandle,
}

/// Returns the bounds (in virtual screen coordinates) of a monitor containing the origin of `w`.
///
/// (We don't guarantee any particular behavior if there is more than one such monitor).
fn screen_bounds(w: &WindowHandle) -> Rect {
    let monitors = Screen::get_monitors();
    let scale = w.get_scale().unwrap_or_default();
    let window_origin = w.get_position();
    for m in monitors {
        if m.virtual_rect().to_dp(scale).contains(window_origin) {
            return m.virtual_work_rect().to_dp(scale);
        }
    }
    Rect::from_origin_size(Point::ZERO, Size::new(f64::INFINITY, f64::INFINITY))
}

fn calc_nudge(rect: Rect, bounds: Rect) -> Vec2 {
    // Returns an offset that tries to translate interval to within bounds.
    fn nudge(interval: (f64, f64), bounds: (f64, f64)) -> f64 {
        let nudge_up = (bounds.0 - interval.0).max(0.0);
        let nudge_down = (bounds.1 - interval.1).min(0.0);
        if nudge_up > 0.0 {
            nudge_up
        } else {
            nudge_down
        }
    }

    let x_nudge = nudge((rect.x0, rect.x1), (bounds.x0, bounds.x1));
    let y_nudge = nudge((rect.y0, rect.y1), (bounds.y0, bounds.y1));
    Vec2::new(x_nudge, y_nudge)
}

impl<T: Data, W: Widget<T>> Widget<T> for OnMonitor<W> {
    fn event(&mut self, ctx: &mut EventCtx, ev: &Event, data: &mut T, env: &Env) {
        self.inner.event(ctx, ev, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, ev: &LifeCycle, data: &T, env: &Env) {
        match ev {
            LifeCycle::Size(_) | LifeCycle::Internal(InternalLifeCycle::ParentWindowOrigin) => {
                let w = ctx.window();
                let rect = Rect::from_origin_size(ctx.window_origin(), ctx.size());
                let current_window_pos = w.get_position();
                let bounds = screen_bounds(&self.parent);
                let nudge = calc_nudge(rect + current_window_pos.to_vec2(), bounds);
                w.set_position(current_window_pos + nudge);
            }
            _ => {}
        }
        self.inner.lifecycle(ctx, ev, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.inner.update(ctx, old_data, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        self.inner.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.inner.paint(ctx, data, env);
    }
}
