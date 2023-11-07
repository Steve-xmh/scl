//! 一些启动/安装游戏时会用到的实用模块

use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use inner_future::io::{AsyncRead, AsyncReadExt};
use sha1_smol::*;

use crate::prelude::*;

/// 根据当前构建目标判定的当前操作系统类型
///
/// 用于版本元数据的参数和依赖库的条件判断
#[cfg(target_os = "windows")]
pub const TARGET_OS: &str = "windows";
/// 根据当前构建目标判定的当前操作系统类型
///
/// 用于版本元数据的参数和依赖库的条件判断
#[cfg(target_os = "macos")]
pub const TARGET_OS: &str = "osx";
/// 根据当前构建目标判定的当前操作系统类型
///
/// 用于版本元数据的参数和依赖库的条件判断
#[cfg(target_os = "linux")]
pub const TARGET_OS: &str = "linux";

/// 根据当前构建目标判定的当前操作系统架构类型
///
/// 用于版本元数据的参数和依赖库的条件判断
#[cfg(target_arch = "x86")]
pub const TARGET_ARCH: &str = "x86";
/// 根据当前构建目标判定的当前操作系统架构类型
///
/// 用于版本元数据的参数和依赖库的条件判断
#[cfg(target_arch = "x86_64")]
pub const TARGET_ARCH: &str = "x86_64";
/// 根据当前构建目标判定的当前操作系统架构类型
///
/// 用于版本元数据的参数和依赖库的条件判断
#[cfg(target_arch = "arm")]
pub const NATIVE_ARCH: &str = "arm";
/// 根据当前构建目标判定的当前操作系统架构类型
///
/// 用于版本元数据的参数和依赖库的条件判断
#[cfg(target_arch = "aarch64")]
pub const TARGET_ARCH: &str = "aarch64";

/// 根据当前构建目标判定的当前操作系统的操作位数
///
/// 用于版本元数据的参数和依赖库的条件判断
#[cfg(target_arch = "x86")]
pub const NATIVE_ARCH: &str = "32";
/// 根据当前构建目标判定的当前操作系统的操作位数
///
/// 用于版本元数据的参数和依赖库的条件判断
#[cfg(target_arch = "x86_64")]
pub const NATIVE_ARCH: &str = "64";
/// 根据当前构建目标判定的当前操作系统的操作位数
///
/// 用于版本元数据的参数和依赖库的条件判断
#[cfg(target_arch = "arm")]
pub const NATIVE_ARCH: &str = "64";
/// 根据当前构建目标判定的当前操作系统的操作位数
///
/// 用于版本元数据的参数和依赖库的条件判断
#[cfg(target_arch = "aarch64")]
pub const NATIVE_ARCH: &str = "64";

/// 根据当前构建目标判定的类路径的分隔符
///
/// 用于启动参数的 `classpath` 部分的拼接
#[cfg(target_os = "windows")]
pub const CLASSPATH_SEPARATOR: &str = ";";
/// 根据当前构建目标判定的类路径的分隔符
///
/// 用于启动参数的 `classpath` 部分的拼接
#[cfg(not(target_os = "windows"))]
pub const CLASSPATH_SEPARATOR: &str = ":";

/// 一个内存页面位移值，仅 MacOS 用，用于自动内存计算
#[cfg(target_os = "macos")]
pub static PAGESHIFT: once_cell::sync::Lazy<libc::c_int> = once_cell::sync::Lazy::new(|| {
    let mut pagesize = unsafe { getpagesize() };
    let mut pageshift = 0;
    while pagesize > 1 {
        pageshift += 1;
        pagesize >>= 1;
    }
    pageshift - 10 // LOG1024
});

#[cfg(target_os = "macos")]
#[link(name = "c")]
extern "C" {
    fn getpagesize() -> libc::c_int;
}

/// 异步计算一个数据的 SHA1 摘要值
///
/// 返回一个十六进制的小写摘要字符串
pub async fn get_data_sha1(data: &mut (impl AsyncRead + Unpin)) -> DynResult<String> {
    let mut buf = [0u8; 16];
    let mut sha = Sha1::default();
    loop {
        let size = data.read(&mut buf).await?;
        if size > 0 {
            sha.update(&buf[..size]);
        } else {
            break;
        }
    }
    Ok(sha.hexdigest())
}

/// 返回一个相对路径的绝对路径格式
///
/// 因为标准库的 [`std::fs::canonicalize`] 不支持不存在的路径的解析，所以做了这个
///
/// 用于启动参数的路径绝对化
pub fn get_full_path(p: impl AsRef<std::path::Path>) -> String {
    use path_absolutize::*;
    let p = p.as_ref();
    match p.absolutize() {
        Ok(p) => {
            #[cfg(windows)]
            if let Some(p) = p.to_string_lossy().strip_prefix("\\\\?\\") {
                p.to_string()
            } else {
                p.to_string_lossy().to_string()
            }
            #[cfg(not(windows))]
            p.to_string_lossy().to_string()
        }
        Err(e) => {
            tracing::trace!(
                "Warning: Can't convert path {} to full path: {}",
                p.to_string_lossy(),
                e
            );
            p.to_string_lossy().to_string()
        }
    }
}

/// 根据传入的参数，返回一个可执行文件的绝对路径，如有必要会加上 `.exe` 后缀
///
/// 首先会确认路径是否存在，存在就直接返回，否则获取 `PATH` 环境变量并根据其中的路径逐个查询。
pub fn locate_path(exe_name: impl AsRef<Path>) -> PathBuf {
    if exe_name.as_ref().is_file() {
        return exe_name.as_ref().to_path_buf();
    } else {
        #[cfg(target_os = "windows")]
        {
            let exe_path = exe_name.as_ref().with_extension("exe");
            if exe_path.is_file() {
                return exe_path;
            }
        }
    }
    std::env::var_os("PATH")
        .and_then(|paths| {
            std::env::split_paths(&paths).find_map(|dir| {
                let full_path = dir.join(&exe_name);
                if full_path.is_file() {
                    Some(full_path)
                } else {
                    #[cfg(target_os = "windows")]
                    {
                        let exe_path = full_path.with_extension("exe");
                        if exe_path.is_file() {
                            Some(exe_path)
                        } else {
                            None
                        }
                    }
                    #[cfg(not(target_os = "windows"))]
                    {
                        None
                    }
                }
            })
        })
        .unwrap_or_else(|| exe_name.as_ref().to_path_buf())
}

/// 系统架构枚举
///
/// 目前根据 SCL 自身会支持的平台增加此处的枚举值
///
/// 用于启动参数的条件判断组合
#[derive(Debug, Clone, Copy)]
pub enum Arch {
    /// 一个 `x86` 平台
    X86,
    /// 一个 `x86_64`/`amd64` 平台
    X64,
    /// 一个 `arm64`/`aarch64` 平台
    ARM64,
}

impl Default for Arch {
    fn default() -> Self {
        #[cfg(target_arch = "x86")]
        return Arch::X86;
        #[cfg(target_arch = "x86_64")]
        return Arch::X64;
        #[cfg(target_arch = "aarch64")]
        return Arch::ARM64;
    }
}

impl Display for Arch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl AsRef<str> for Arch {
    fn as_ref(&self) -> &str {
        match self {
            Arch::X86 => "x86",
            Arch::X64 => "x86_64",
            Arch::ARM64 => "aarch64",
        }
    }
}

/// 一个延迟获取的当前系统的架构
///
/// 这个会获取到系统自身的架构，而非软件自身的编译目标架构
pub static NATIVE_ARCH_LAZY: once_cell::sync::Lazy<Arch> =
    once_cell::sync::Lazy::new(get_system_arch);

fn get_system_arch() -> Arch {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::System::SystemInformation::*;
        let mut info: SYSTEM_INFO = Default::default();
        GetNativeSystemInfo(&mut info);
        match info.Anonymous.Anonymous.wProcessorArchitecture.0 {
            0 => Arch::X86,
            12 => Arch::ARM64,
            9 => Arch::X64,
            _ => unreachable!(),
        }
    }
    #[cfg(all(target_os = "linux", target_arch = "x86"))]
    return Arch::X86;
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return Arch::X64;
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    return Arch::ARM64;
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return Arch::X64;
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return Arch::ARM64;
}

/// 内存状态对象，单位为 MB
pub struct MemoryStatus {
    /// 机器的内存总量，单位为 MB
    pub max: u64,
    /// 机器的可用内存总量，单位为 MB
    pub free: u64,
}

/// 获取一个可执行程序所对应的运行架构，对应不同系统有不同的实现。
///
/// 用于 SCL 识别对应的 Java 运行时架构，以选择正确的原生库路径
///
/// - 对于 Windows 的可执行文件格式描述，请参阅 "Microsoft PE and COFF Specification"
/// - 对于 MacOS 的可执行文件格式描述，请参阅 "OS X ABI Mach-O File Format Reference"
/// - 对于 Linux 的可执行文件格式描述，请参阅 "ELF-64 Object File Format"
#[cfg(target_os = "windows")]
pub async fn get_exec_arch(file_path: impl AsRef<std::path::Path>) -> DynResult<Arch> {
    use inner_future::io::AsyncSeekExt;

    let mut file = inner_future::fs::OpenOptions::new()
        .read(true)
        .open(file_path.as_ref())
        .await?;

    let mut buf = [0u8; 2];

    file.read_exact(&mut buf).await?;

    // PE Magic Number

    if !(buf[0] == 0x4D && buf[1] == 0x5A) {
        anyhow::bail!("文件不是一个合法的 PE 可执行文件");
    }

    file.seek(inner_future::io::SeekFrom::Start(0x3C)).await?;

    let mut buf = [0u8; 4];

    file.read_exact(&mut buf).await?;

    let pe_header_offset = u32::from_le_bytes(buf);

    file.seek(inner_future::io::SeekFrom::Start(pe_header_offset as u64))
        .await?;

    file.read_exact(&mut buf).await?;

    if !(buf[0] == 0x50 && buf[1] == 0x45 && buf[2] == 0x00 && buf[3] == 0x00) {
        anyhow::bail!("文件不是一个合法的 PE 可执行文件");
    }

    let mut buf = [0u8; 2];

    file.read_exact(&mut buf).await?;

    match buf {
        [0x4C, 0x01] => Ok(Arch::X86),   // X86 I386
        [0x64, 0x86] => Ok(Arch::X64),   // X86_64
        [0x64, 0xAA] => Ok(Arch::ARM64), // ARM64
        _ => anyhow::bail!("不支持判定此架构"),
    }
}

/// 获取一个可执行程序所对应的运行架构，对应不同系统有不同的实现。
///
/// 用于 SCL 识别对应的 Java 运行时架构，以选择正确的原生库路径
///
/// - 对于 Windows 的可执行文件格式描述，请参阅 "Microsoft PE and COFF Specification"
/// - 对于 MacOS 的可执行文件格式描述，请参阅 "OS X ABI Mach-O File Format Reference"
/// - 对于 Linux 的可执行文件格式描述，请参阅 "ELF-64 Object File Format"
#[cfg(target_os = "macos")]
pub async fn get_exec_arch(file_path: impl AsRef<std::path::Path>) -> DynResult<Arch> {
    let mut file = inner_future::fs::OpenOptions::new()
        .read(true)
        .open(file_path.as_ref())
        .await?;

    let mut buf = [0u8; 8];

    file.read_exact(&mut buf).await?;

    // 多架构 Mach-O，默认取当前运行架构
    // 一般出现于 /usr/bin/java 中
    if buf[0] == 0xCA && buf[1] == 0xFE && buf[2] == 0xBA && buf[3] == 0xBE {
        return Ok(Arch::default());
    }
    // Mach-O Magic Number
    // CF FA ED FE
    if !(buf[0] == 0xCF && buf[1] == 0xFA && buf[2] == 0xED && buf[3] == 0xFE) {
        anyhow::bail!("文件不是一个合法的 Mach-O 可执行文件");
    }

    // CPU Arch Type
    match (buf[4], buf[7]) {
        (7, 0) => Ok(Arch::X86),    // X86 I386
        (7, 1) => Ok(Arch::X64),    // X86_64
        (12, 1) => Ok(Arch::ARM64), // ARM64
        (_, _) => anyhow::bail!("不支持判定此架构"),
    }
}

/// 获取一个可执行程序所对应的运行架构，对应不同系统有不同的实现。
///
/// 用于 SCL 识别对应的 Java 运行时架构，以选择正确的原生库路径
///
/// - 对于 Windows 的可执行文件格式描述，请参阅 "Microsoft PE and COFF Specification"
/// - 对于 MacOS 的可执行文件格式描述，请参阅 "OS X ABI Mach-O File Format Reference"
/// - 对于 Linux 的可执行文件格式描述，请参阅 "ELF-64 Object File Format"
#[cfg(target_os = "linux")]
pub async fn get_exec_arch(file_path: impl AsRef<std::path::Path>) -> DynResult<Arch> {
    use inner_future::io::AsyncSeekExt;

    let mut file = inner_future::fs::OpenOptions::new()
        .read(true)
        .open(file_path.as_ref())
        .await?;

    let mut buf = [0u8; 4];
    file.read_exact(&mut buf).await?;

    // ELF Magic Number
    // 7F 45 4C 46
    if !(buf[0] == 0x7F && buf[1] == 0x45 && buf[2] == 0x4C && buf[3] == 0x46) {
        anyhow::bail!("文件不是一个合法的 ELF 可执行文件");
    }

    file.seek(inner_future::io::SeekFrom::Start(0x12)).await?;

    let mut buf = [0u8; 2];
    file.read_exact(&mut buf).await?;

    let elf_type = u16::from_le_bytes(buf);

    if elf_type != 2 {
        anyhow::bail!("文件不是一个合法的 ELF 可执行文件");
    }

    match u16::from_le_bytes(buf) {
        0x03 => Ok(Arch::X86),   // X86
        0x3E => Ok(Arch::X64),   // X86_64
        0xB7 => Ok(Arch::ARM64), // ARM64
        _ => anyhow::bail!("不支持判定此架构"),
    }
}

/// 获取当前内存使用状态，单位为 MB
pub fn get_mem_status() -> MemoryStatus {
    #[cfg(target_os = "windows")]
    unsafe {
        use windows::Win32::System::SystemInformation::MEMORYSTATUSEX;
        let mut ms = MEMORYSTATUSEX {
            dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as _,
            ..Default::default()
        };
        windows::Win32::System::SystemInformation::GlobalMemoryStatusEx(&mut ms).unwrap();
        MemoryStatus {
            max: ms.ullTotalPhys / 1024 / 1024,
            free: ms.ullAvailPhys / 1024 / 1024,
        }
    }
    #[cfg(target_os = "linux")]
    {
        let stat = std::fs::read_to_string("/proc/meminfo").unwrap();
        let mut max = None;
        let mut free = None;
        for line in stat.lines() {
            // 原单位是 kB
            if line.starts_with("MemTotal:") {
                max = line[10..line.len() - 3]
                    .trim()
                    .parse::<u64>()
                    .map(|x| Some(x / 1024))
                    .unwrap_or_default();
            } else if line.starts_with("MemFree:") {
                free = line[9..line.len() - 3]
                    .trim()
                    .parse::<u64>()
                    .map(|x| Some(x / 1024))
                    .unwrap_or_default();
            }
            if max.is_some() && free.is_some() {
                break;
            }
        }
        MemoryStatus {
            max: max.unwrap_or(2048),
            free: free.unwrap_or(2048),
        }
    }
    #[cfg(target_os = "macos")]
    {
        unsafe {
            let total = libc::sysconf(libc::_SC_PHYS_PAGES);
            if total == -1 {
                return MemoryStatus {
                    max: 2048,
                    free: 2048,
                };
            }

            let host_port = libc::mach_host_self();
            let mut stat = std::mem::MaybeUninit::<libc::vm_statistics64>::zeroed();
            let mut stat_count = libc::HOST_VM_INFO64_COUNT;

            if libc::host_statistics64(
                host_port,
                libc::HOST_VM_INFO64,
                stat.as_mut_ptr() as *mut i32,
                &mut stat_count,
            ) != libc::KERN_SUCCESS
            {
                return MemoryStatus {
                    max: 2048,
                    free: 2048,
                };
            }

            let stat = stat.assume_init();

            MemoryStatus {
                max: ((total as u64) << *PAGESHIFT) / 1024,
                free: (((stat.inactive_count + stat.free_count) as u64) << *PAGESHIFT) / 1024,
            }
        }
    }
}
