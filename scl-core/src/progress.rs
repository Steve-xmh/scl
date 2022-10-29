//! 这里提供的特质用于报告异步进度

/// 一个异步报告特质，用来给宿主提供目前异步程序的处理状态
///
/// 实现该特质时需要注意，当对象被克隆时，其响应也应当和原始对象一致。
///
/// 当进行调用时，如不需要报告进度可以使用 `None as Option<()>`（或本模块中的 `NR` 常量）来跳过报告
pub trait Reporter: Clone + Send + Sync {
    /// 返回一个和原始对象分离的报告对象，应当是一个全新的报告对象
    ///
    /// 所响应的进度不会同时上报给原始对象
    #[must_use]
    fn fork(&self) -> Self {
        self.to_owned()
    }

    /// 返回一个报告会同时上报给原始报告的对象
    #[must_use]
    fn sub(&self) -> Self {
        self.fork()
    }

    /// 发送信息，并根据情况相继上报给原始对象
    fn send(&self, state: ReportState);

    /// 发送信息，但是可变引用状态，并根据情况相继上报给原始对象
    ///
    /// 如果你的上报对象自身对不可变有限制可以考虑实现这个方法
    fn send_mut(&mut self, state: ReportState) {
        self.send(state)
    }
}

pub(crate) trait Progress: Reporter {
    fn set_message(&self, msg: String) {
        self.send(ReportState::SetMessage(msg));
    }
    fn set_sub_message(&self, msg: String) {
        self.send(ReportState::SetSubMessage(msg));
    }
    fn set_max_progress(&self, value: f64) {
        self.send(ReportState::SetMaxProgress(value));
    }
    fn add_max_progress(&self, value: f64) {
        self.send(ReportState::AddMaxProgress(value));
    }
    fn set_progress(&self, value: f64) {
        self.send(ReportState::SetProgress(value));
    }
    fn add_progress(&self, value: f64) {
        self.send(ReportState::AddProgress(value));
    }
    fn set_indeterminate_progress(&self) {
        self.send(ReportState::SetIndeterminateProgress);
    }
    fn hide_progress(&self) {
        self.send(ReportState::SetIndeterminateProgress);
    }
    fn remove_progress(self) {
        self.send(ReportState::RemoveProgress);
    }
}

impl<R: Reporter> Reporter for Option<R> {
    fn fork(&self) -> Self {
        self.as_ref().map(|s| s.fork())
    }

    fn sub(&self) -> Self {
        self.as_ref().map(|s| s.sub())
    }

    fn send(&self, state: ReportState) {
        if let Some(s) = &self {
            s.send(state);
        }
    }

    fn send_mut(&mut self, state: ReportState) {
        if let Some(s) = self {
            s.send_mut(state);
        }
    }
}

impl<R: Reporter> Progress for R {}

/// 一个不会有任何响应的报告对象，如果宿主不需要获悉进度或状态可将这个传入参数
pub const NR: Option<()> = None;

impl Reporter for () {
    fn send(&self, _: ReportState) {}
}

/// 核心库报告的异步进度的所有枚举
#[derive(Debug, Clone)]
pub enum ReportState {
    /// 设置主要文字信息
    SetMessage(String),
    /// 设置次要文字信息
    SetSubMessage(String),
    /// 设置进度的最大值
    SetMaxProgress(f64),
    /// 增加/减少进度的最大值
    AddMaxProgress(f64),
    /// 设置当前进度
    SetProgress(f64),
    /// 增加/减少当前进度
    AddProgress(f64),
    /// 将进度设置为不定进度模式
    SetIndeterminateProgress,
    /// 隐藏此进度
    HideProgress,
    /// 删除此进度
    RemoveProgress,
}
