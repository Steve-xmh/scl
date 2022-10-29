pub(crate) use smol as inner_future;
pub(crate) type DynResult<T = ()> = anyhow::Result<T>;
pub(crate) use serde::*;

pub use crate::download::GameDownload;
pub(crate) use crate::progress::*;
