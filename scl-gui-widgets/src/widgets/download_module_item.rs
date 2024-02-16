use druid::{
    kurbo::{BezPath, Shape},
    piet::{PaintBrush, TextStorage},
    widget::{Click, ControllerHost, Image, LabelText},
    Affine, Data, Env, Event, LifeCycle, Point, RenderContext, Widget, WidgetExt, WidgetPod,
};

use super::label;
use crate::theme::{
    color::{
        base, list,
        main::IS_DARK,
        typography::{BODY, CAPTION_ALT},
    },
    icons::IconKeyPair,
};

enum Icon<D> {
    BezPath(BezPath),
    Image(Box<WidgetPod<D, Image>>),
}

/// 一个左侧有图标和说明信息，右侧有副文本信息的可点击项组件
pub struct DownloadModuleItem<D> {
    icon_key: IconKeyPair,
    icon: Icon<D>,
    text: WidgetPod<D, Box<dyn Widget<D>>>,
    desc: WidgetPod<D, Box<dyn Widget<D>>>,
}

impl<D: Data> DownloadModuleItem<D> {
    /// 根据所给的图标对（通过 `scl-macros` 来生成图标）创建组件
    pub fn new(
        icon_key: IconKeyPair,
        text: impl Into<LabelText<D>>,
        desc: impl Into<LabelText<D>>,
    ) -> Self {
        Self {
            icon_key,
            icon: Icon::BezPath(BezPath::new()),
            text: WidgetPod::new(Box::new(
                label::new(text)
                    .with_text_size(14.)
                    .with_text_color(base::MEDIUM)
                    .with_font(BODY)
                    .align_vertical(druid::UnitPoint::LEFT),
            )),
            desc: WidgetPod::new(Box::new(
                label::new(desc)
                    .with_text_color(base::MEDIUM)
                    .with_font(CAPTION_ALT)
                    .align_vertical(druid::UnitPoint::RIGHT),
            )),
        }
    }

    /// 根据所给的图像组件创建组件
    pub fn new_image(
        img: Image,
        text: impl Into<LabelText<D>>,
        desc: impl Into<LabelText<D>>,
    ) -> Self {
        Self {
            icon_key: crate::theme::icons::EMPTY,
            icon: Icon::Image(Box::new(WidgetPod::new(img))),
            text: WidgetPod::new(Box::new(
                label::new(text)
                    .with_text_size(14.)
                    .with_text_color(base::MEDIUM)
                    .with_font(BODY)
                    .align_vertical(druid::UnitPoint::LEFT),
            )),
            desc: WidgetPod::new(Box::new(
                label::new(desc)
                    .with_text_color(base::MEDIUM)
                    .with_font(CAPTION_ALT)
                    .align_vertical(druid::UnitPoint::RIGHT),
            )),
        }
    }

    /// 根据所给的图标对（通过 `scl-macros` 来生成图标）创建组件，但可以是动态文字
    pub fn dynamic(
        icon_key: IconKeyPair,
        text: impl Fn(&D, &Env) -> String + 'static,
        desc: impl Fn(&D, &Env) -> String + 'static,
    ) -> Self {
        Self {
            icon_key,
            icon: Icon::BezPath(BezPath::new()),
            text: WidgetPod::new(Box::new(
                label::dynamic(text)
                    .with_text_size(14.)
                    .with_text_color(base::MEDIUM)
                    .with_font(BODY)
                    .align_vertical(druid::UnitPoint::LEFT),
            )),
            desc: WidgetPod::new(Box::new(
                label::dynamic(desc)
                    .with_text_color(base::MEDIUM)
                    .with_font(CAPTION_ALT)
                    .align_vertical(druid::UnitPoint::RIGHT),
            )),
        }
    }

    /// 根据所给的图像组件创建组件，但可以是动态文字
    pub fn dynamic_image(
        img: Image,
        text: impl Fn(&D, &Env) -> String + 'static,
        desc: impl Fn(&D, &Env) -> String + 'static,
    ) -> Self {
        Self {
            icon_key: crate::theme::icons::EMPTY,
            icon: Icon::Image(Box::new(WidgetPod::new(img))),
            text: WidgetPod::new(Box::new(
                label::new(text)
                    .with_text_size(14.)
                    .with_text_color(base::MEDIUM)
                    .with_font(BODY)
                    .align_vertical(druid::UnitPoint::LEFT),
            )),
            desc: WidgetPod::new(Box::new(
                label::new(desc)
                    .with_text_color(base::MEDIUM)
                    .with_font(CAPTION_ALT)
                    .align_vertical(druid::UnitPoint::RIGHT),
            )),
        }
    }

    /// Provide a closure to be called when this button is clicked.
    pub fn on_click(
        self,
        f: impl Fn(&mut druid::EventCtx, &mut D, &druid::Env) + 'static,
    ) -> ControllerHost<Self, Click<D>> {
        ControllerHost::new(self, Click::new(f))
    }

    fn reload_icon(&mut self, env: &druid::Env) {
        if let Icon::BezPath(p) = &mut self.icon {
            *p = BezPath::from_svg(env.get(&self.icon_key.0).as_str()).unwrap_or_default();
        }
    }
}

impl<D: Data> Widget<D> for DownloadModuleItem<D> {
    fn event(&mut self, ctx: &mut druid::EventCtx, event: &Event, data: &mut D, env: &druid::Env) {
        match event {
            Event::MouseDown(_) => {
                ctx.set_active(true);
                ctx.request_paint();
            }
            Event::MouseUp(_) => {
                if ctx.is_active() {
                    ctx.set_active(false);
                    ctx.request_paint();
                }
            }
            _ => (),
        }
        self.text.event(ctx, event, data, env);
        self.desc.event(ctx, event, data, env);
        if let Icon::Image(img) = &mut self.icon {
            img.event(ctx, event, data, env);
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut druid::LifeCycleCtx,
        event: &druid::LifeCycle,
        data: &D,
        env: &druid::Env,
    ) {
        if let LifeCycle::WidgetAdded = event {
            self.reload_icon(env);
        } else if let LifeCycle::HotChanged(_) = event {
            ctx.request_paint();
        }
        self.text.lifecycle(ctx, event, data, env);
        self.desc.lifecycle(ctx, event, data, env);
        if let Icon::Image(img) = &mut self.icon {
            img.lifecycle(ctx, event, data, env);
        }
    }

    fn update(&mut self, ctx: &mut druid::UpdateCtx, old_data: &D, data: &D, env: &druid::Env) {
        if ctx.has_requested_update() || !old_data.same(data) {
            self.text.update(ctx, data, env);
            self.desc.update(ctx, data, env);
        }
        match &mut self.icon {
            Icon::Image(img) => {
                img.update(ctx, data, env);
            }
            Icon::BezPath(_) => {
                if ctx.env_key_changed(&self.icon_key.0) {
                    self.reload_icon(env);
                }
                if ctx.env_key_changed(&self.icon_key.1) {
                    ctx.request_paint();
                }
            }
        }
    }

    fn layout(
        &mut self,
        ctx: &mut druid::LayoutCtx,
        bc: &druid::BoxConstraints,
        data: &D,
        env: &druid::Env,
    ) -> druid::Size {
        bc.debug_check("DownloadModuleItem");
        let bc = bc.shrink_max_height_to(40.);
        let text_bc = bc.shrink((40., 0.));
        let desc_bc = bc.shrink((50., 0.));
        let text_size = self.text.layout(ctx, &text_bc, data, env);
        let desc_size = self.desc.layout(ctx, &desc_bc, data, env);
        self.text.set_origin(ctx, (40., 0.).into());
        self.desc.set_origin(ctx, (40., 0.).into());
        if let Icon::Image(img) = &mut self.icon {
            let img_size = img.layout(ctx, &bc, data, env);
            let top_left = Point::new(
                (40.0 - img_size.width) / 2.0,
                (40.0 - img_size.height) / 2.0,
            );
            img.set_origin(ctx, top_left);
        }
        bc.constrain((text_size.width.max(desc_size.width) + 40., 40.))
    }

    fn paint(&mut self, ctx: &mut druid::PaintCtx, data: &D, env: &druid::Env) {
        let size = ctx.size();
        let is_hot = ctx.is_hot();
        let is_active = ctx.is_active();

        if is_hot {
            ctx.fill(
                size.to_rect(),
                &PaintBrush::Color(if is_active {
                    env.get(base::LOW)
                } else {
                    env.get(list::LIST_LOW)
                }),
            )
        }
        let icon_size = druid::Size::new(size.height, size.height);
        ctx.with_save(|ctx| match &mut self.icon {
            Icon::BezPath(p) => {
                let icon_brush = PaintBrush::Color(env.get(if env.get(IS_DARK) {
                    &self.icon_key.2
                } else {
                    &self.icon_key.1
                }));
                ctx.transform(Affine::translate(
                    ((icon_size - p.bounding_box().size()) / 2.).to_vec2(),
                ));
                ctx.fill_even_odd(p.to_owned(), &icon_brush);
            }
            Icon::Image(img) => {
                img.paint(ctx, data, env);
            }
        });
        self.text.paint(ctx, data, env);
        self.desc.paint(ctx, data, env);
    }
}
