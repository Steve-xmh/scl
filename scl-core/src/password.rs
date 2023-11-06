//! 一个密码类，String 的壳子，用来在调试输出时挡住真实密码，防止泄露

use std::{
    fmt::{Debug, Display},
    ops::Deref,
};

use serde::{de::Visitor, Deserialize, Deserializer, Serialize};

/// 一个密码类，String 的壳子，用来在调试输出时挡住真实密码，防止泄露
///
/// 任何格式化输出都会返回 `***Password***`，所以如果需要取用密码，请使用 [`Password::take_string`] [`Password::to_owned_string`] [`Password::as_string`]
#[derive(Clone, PartialEq, Eq, Default)]
pub struct Password(String);

impl Serialize for Password {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Password {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PasswordVisitor;
        impl<'de> Visitor<'de> for PasswordVisitor {
            type Value = Password;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a password as string")
            }

            fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Password(v.to_string()))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Password(v.to_string()))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Password(v.to_string()))
            }
        }
        deserializer.deserialize_str(PasswordVisitor)
    }
}

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

impl From<&str> for Password {
    fn from(a: &str) -> Self {
        Self(a.to_owned())
    }
}
