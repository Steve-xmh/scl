//! 解析版本号
use std::{
    cmp::Ordering,
    fmt::{Display, Formatter},
    str::FromStr,
};

use nom::*;

/// 一个用于表达当前 Minecraft 版本号的枚举结构
///
/// 用来根据版本判断使用不同的安装方式，还有相关的资源获取等
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd)]
pub enum MinecraftVersion {
    /// 正式版本的版本号
    Release(u32, u32, u32),
    /// 快照版本的版本号
    Snapshot(u32, u32, char),
    /// 一些特殊版本的版本号，有可能是 Beta 或者 Alpha 等远古版本或被特殊命名的版本
    Custom(String),
}

impl Default for MinecraftVersion {
    fn default() -> Self {
        Self::Custom("".into())
    }
}

impl FromStr for MinecraftVersion {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_version(s).map(|a| a.1).or(Err(()))
    }
}

impl Display for MinecraftVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MinecraftVersion::Release(a, b, c) => {
                if c == &0 {
                    write!(f, "{a}.{b}")
                } else {
                    write!(f, "{a}.{b}.{c}")
                }
            }
            MinecraftVersion::Snapshot(a, b, c) => write!(f, "{a:02}w{b:02}{c}"),
            Self::Custom(c) => write!(f, "{c}"),
        }
    }
}

impl MinecraftVersion {
    /// 检查该版本需要的最低 Java 版本  
    /// 目前检测到 1.17+ 的正式版本都会返回 16，其余的返回 8
    pub fn required_java_version(&self) -> u8 {
        if let Self::Release(mayor, minor, _) = *self {
            if mayor >= 1 && minor >= 21 {
                21
            } else if mayor >= 1 && minor >= 17 {
                16
            } else {
                8
            }
        } else if let Self::Snapshot(year, week, num) = *self {
            // https://www.minecraft.net/zh-hans/article/minecraft-snapshot-24w14a
            if year >= 24 && week >= 14 && num >= 'a' {
                21
            } else if year >= 21 && week >= 8 {
                16
            } else {
                8
            }
        } else {
            8
        }
    }

    /// 确认如果该版本需要安装 Forge，是否使用覆盖 minecraft.jar 的方式进行安装
    ///
    /// 一般在版本为 1.5.1 或者更早时为 `true`
    ///
    /// 如果为 `false` 则使用 Forge 安装器安装
    pub fn should_forge_use_override_installiation(&self) -> bool {
        if let Self::Release(a, b, c) = self {
            match a.cmp(&1) {
                Ordering::Greater => false,
                Ordering::Equal => match b.cmp(&1) {
                    Ordering::Greater => false,
                    Ordering::Equal => *c < 2,
                    Ordering::Less => true,
                },
                Ordering::Less => true,
            }
        } else {
            true
        }
    }

    /// 确认如果该版本需要安装 Forge，下载的文件名尾缀是否是 `client` 还是 `universal`
    ///
    /// 一般在版本为 1.2.5 或者更早时为 `true`
    ///
    /// 如果为 `false` 则使用 `universal` 作为尾缀
    pub fn should_forge_use_client_or_universal(&self) -> bool {
        if let Self::Release(a, b, c) = self {
            match a.cmp(&1) {
                Ordering::Greater => false,
                Ordering::Equal => match b.cmp(&2) {
                    Ordering::Greater => false,
                    Ordering::Equal => *c <= 5,
                    Ordering::Less => true,
                },
                Ordering::Less => true,
            }
        } else {
            true
        }
    }
}

/// 尝试解析版本字符串，并转换成 [`MinecraftVersion`] 枚举类型
pub fn parse_version(input: &str) -> IResult<&str, MinecraftVersion> {
    let (input, first_number) = character::complete::digit1(input)?;
    let first_number = first_number.parse::<u32>().unwrap();
    let (input, s) = character::complete::one_of(".w")(input)?;
    match s {
        '.' => {
            // Release
            let (input, second_number) = character::complete::digit1(input)?;
            let second_number = second_number.parse::<u32>().unwrap();
            if input.is_empty() {
                return Ok((
                    input,
                    MinecraftVersion::Release(first_number, second_number, 0),
                ));
            }
            let (input, _) = character::complete::char('.')(input)?;
            let (input, third_number) = character::complete::digit1(input)?;
            let third_number = third_number.parse::<u32>().unwrap();
            Ok((
                input,
                MinecraftVersion::Release(first_number, second_number, third_number),
            ))
        }
        'w' => {
            // Snapshot
            let (input, second_number) = character::complete::digit1(input)?;
            let second_number = second_number.parse::<u32>().unwrap();
            let (input, tag_alpha) = character::complete::anychar(input)?;
            Ok((
                input,
                MinecraftVersion::Snapshot(first_number, second_number, tag_alpha),
            ))
        }
        _ => {
            panic!("Version dot is not correct!")
        }
    }
}

#[test]
fn parse_version_test() {
    assert_eq!(
        parse_version("1.16.5").unwrap().1,
        MinecraftVersion::Release(1, 16, 5)
    );
    assert_eq!(
        parse_version("1.8").unwrap().1,
        MinecraftVersion::Release(1, 8, 0)
    );
    assert_eq!(
        parse_version("21w08b").unwrap().1,
        MinecraftVersion::Snapshot(21, 8, 'b')
    );
    assert_eq!(
        "1.16.5".parse::<MinecraftVersion>().unwrap(),
        MinecraftVersion::Release(1, 16, 5)
    );
    assert_eq!(
        "1.8".parse::<MinecraftVersion>().unwrap(),
        MinecraftVersion::Release(1, 8, 0)
    );
    assert_eq!(
        "21w08b".parse::<MinecraftVersion>().unwrap(),
        MinecraftVersion::Snapshot(21, 8, 'b')
    );
    assert_eq!(&MinecraftVersion::Release(1, 16, 5).to_string(), "1.16.5");
    assert_eq!(&MinecraftVersion::Release(1, 8, 0).to_string(), "1.8");
    assert_eq!(
        &MinecraftVersion::Snapshot(21, 8, 'b').to_string(),
        "21w08b"
    );
    assert!(MinecraftVersion::Release(1, 16, 5).required_java_version() >= 8);
    assert!(MinecraftVersion::Release(1, 17, 1).required_java_version() >= 16);
    assert!(MinecraftVersion::Release(1, 17, 0).required_java_version() >= 16);
}
