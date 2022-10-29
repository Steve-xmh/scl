//! 一个密码类，String 的壳子，用来在调试输出时挡住真实密码，防止泄露

use std::{
    fmt::{Debug, Display},
    ops::Deref,
};

use serde::{Deserialize, Serialize};

/// 一个密码类，String 的壳子，用来在调试输出时挡住真实密码，防止泄露
#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Password(String);

impl Password {
    /// 从密码类中拿出原始字符串，请注意保护密码安全
    pub fn take_string(self) -> String {
        self.0
    }

    /// 从密码类中复制出原始字符串，请注意保护密码安全
    pub fn to_owned_string(&self) -> String {
        self.0.to_owned()
    }

    /// 从密码类中借出原始字符串，请注意保护密码安全
    pub fn as_string(&self) -> &String {
        &self.0
    }
}

impl Debug for Password {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("***Password***")
    }
}

impl Display for Password {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("***Password***")
    }
}

impl Deref for Password {
    type Target = String;
    fn deref(&self) -> &String {
        &self.0
    }
}

impl From<Password> for String {
    fn from(a: Password) -> Self {
        a.0
    }
}

impl From<String> for Password {
    fn from(a: String) -> Self {
        Self(a)
    }
}
