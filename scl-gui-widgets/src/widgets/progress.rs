use druid::{
    kurbo::BezPath,
    piet::{PaintBrush, StrokeStyle},
    Affine, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx,
    PaintCtx, RenderContext, Size, UpdateCtx, Widget,
};

use crate::theme::color as theme;

/// 一个仿 WinUI 3 的进度指示圈，仅支持无界进度动画
pub struct ProgressRing {
    indeterminate: bool,
    indeterminate_time: f64,
}

impl ProgressRing {
    /// 创建一个进度指示圈
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for ProgressRing {
    fn default() -> Self {
        Self {
            indeterminate: true,
            indeterminate_time: 0.,
        }
    }
}

impl<T: Data> Widget<T> for ProgressRing {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut T, _env: &Env) {
        if let Event::AnimFrame(t) = event {
            self.indeterminate_time -= *t as f64 / 1_000_000_000.;
            if self.indeterminate {
                ctx.request_paint();
                ctx.request_anim_frame();
            }
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, _data: &T, _env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            if self.indeterminate {
                ctx.request_anim_frame();
            }
        }
    }

    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: &T, _data: &T, _env: &Env) {}

    fn layout(&mut self, _ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &T, _env: &Env) -> Size {
        bc.debug_check("ProgressRing");
        bc.constrain((60., 60.))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &T, env: &Env) {
        let front = env.get(theme::main::SECONDARY);
        let back = env.get(theme::chrome::HIGH);

        let size = Size::new(52., 52.);
        let rect = size.to_rounded_rect(30.);

        let t = self.indeterminate_time;
        let l = ((t * -std::f64::consts::PI).sin() + 1.) / 2.; // ((t * -std::f64::consts::PI).sin() + 1.) / 2.;

        // 曲线的长度
        let thera = l * std::f64::consts::PI;

        // 计算切线长度
        let rad = thera.cos().acos();
        let tangent_scale = 4. / 3. * (1. - (rad / 2.).cos()) / (rad / 2.).sin();
        let tangent_len = tangent_scale * 26.;

        // 计算各个点的正弦余弦
        let (sx, sy) = (t * 8.).sin_cos();
        let (p1ox, p1oy) = (t * 8. + std::f64::consts::FRAC_PI_2).sin_cos();
        let (ex, ey) = (t * 8. + thera).sin_cos();
        let (p2ox, p2oy) = (t * 8. + thera - std::f64::consts::FRAC_PI_2).sin_cos();

        // 点的坐标
        let start_point = (26. + sx * 26., 26. + sy * 26.);
        let end_point = (26. + ex * 26., 26. + ey * 26.);
        let control_1_point = (
            26. + sx * 26. + p1ox * tangent_len,
            26. + sy * 26. + p1oy * tangent_len,
        );
        let control_2_point = (
            26. + ex * 26. + p2ox * tangent_len,
            26. + ey * 26. + p2oy * tangent_len,
        );

        let mut path = BezPath::new();
        path.move_to(start_point);
        path.curve_to(control_1_point, control_2_point, end_point);

        // 绘制
        ctx.transform(Affine::translate((4., 4.)));
        ctx.stroke(rect, &PaintBrush::Color(back), 8.);
        ctx.stroke_styled(
            path,
            &PaintBrush::Color(front),
            8.,
            &StrokeStyle {
                line_cap: druid::piet::LineCap::Round,
                ..Default::default()
            },
        );
        if false {
            fn re((x, y): (f64, f64)) -> druid::Rect {
                druid::Rect::new(x - 0.5, y - 0.5, x + 0.5, y + 0.5)
            }
            // 绘制调试点
            ctx.stroke(re(start_point), &PaintBrush::Color(druid::Color::RED), 8.);
            ctx.stroke(
                re(control_1_point),
                &PaintBrush::Color(druid::Color::AQUA),
                8.,
            );
            ctx.stroke(
                re(control_2_point),
                &PaintBrush::Color(druid::Color::OLIVE),
                8.,
            );
            ctx.stroke(re(end_point), &PaintBrush::Color(druid::Color::GREEN), 8.);
        }
    }

    fn id(&self) -> Option<druid::WidgetId> {
        None
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}
