//! Java 的搜索，版本检测
use std::path::{Path, PathBuf};

use inner_future::stream::StreamExt;

use crate::{prelude::*, utils::Arch};

/// 一个 Java 运行时类型
#[derive(Debug, Clone)]
pub struct JavaRuntime {
    java_path: String,
    java_version: String,
    java_main_version: u8,
    java_64bit: bool,
    java_arch: Arch,
}

impl JavaRuntime {
    /// 通过一个指向 Java 可执行文件的路径来创建 [`JavaRuntime`]
    ///
    /// 在此会尝试运行这个文件并获取相关的版本信息，确认无误后返回
    pub async fn from_java_path(java_path: impl AsRef<std::ffi::OsStr>) -> DynResult<Self> {
        let output = query_java_version_output(&java_path).await?;
        let version = query_java_version(&output);
        let java_main_version = get_java_version(version);
        let java_64bit = query_java_is_64bit(&output);
        let java_arch =
            crate::utils::get_exec_arch(std::path::PathBuf::from(java_path.as_ref())).await?;
        Ok(Self {
            java_path: java_path.as_ref().to_string_lossy().to_string(),
            java_64bit,
            java_version: version.to_owned(),
            java_main_version,
            java_arch,
        })
    }

    /// 获取此 Java 运行时的可执行文件路径
    #[inline]
    pub fn path(&self) -> &str {
        &self.java_path
    }

    /// 获取此 Java 运行时的版本号
    #[inline]
    pub fn version(&self) -> &str {
        &self.java_version
    }

    /// 获取此 Java 运行时的运行架构是否是针对 64 位平台的
    #[inline]
    pub fn is_64bit(&self) -> bool {
        self.java_64bit
    }

    /// 获取此 Java 运行时的主 Java 版本号
    #[inline]
    pub fn main_version(&self) -> u8 {
        self.java_main_version
    }

    /// 获取此 Java 运行时的运行架构
    #[inline]
    pub fn arch(&self) -> Arch {
        self.java_arch
    }
}

/// 执行 Java 并使用 -version 获取其版本输出
async fn query_java_version_output(java_path: impl AsRef<std::ffi::OsStr>) -> DynResult<String> {
    let c = {
        #[cfg(windows)]
        {
            use inner_future::process::windows::CommandExt;
            inner_future::process::Command::new(java_path)
                .arg("-version")
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .creation_flags(0x00000200 | 0x08000000)
                .spawn()?
        }
        #[cfg(not(windows))]
        {
            inner_future::process::Command::new(java_path)
                .arg("-version")
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()?
        }
    };
    let output = c.output().await?;
    Ok(String::from_utf8(output.stderr)?)
}

/// 根据 Java 的版本输出确认是否为 64 位版本
fn query_java_is_64bit(java_output: &str) -> bool {
    java_output.contains("64-Bit")
}

/// 根据 Java 裁剪的版本号文本确认主版本号
/// - `1.8.x` 将返回 `8`
/// - `1.7.x` 将返回 `7`
/// - `10+` 将返回其主版本数字
/// - 其余返回 `0` 表示未知版本
fn get_java_version(java_version_string: &str) -> u8 {
    fn parser(input: &str) -> nom::IResult<&str, &str> {
        nom::character::complete::digit1(input)
    }
    if let Ok((_, r)) = parser(java_version_string) {
        let r = r.parse().unwrap(); // 应当不会出错的
        if r > 1 {
            r
        } else if java_version_string.contains("1.8") {
            8
        } else if java_version_string.contains("1.7") {
            7
        } else {
            0 // 未知版本
        }
    } else if java_version_string.contains("1.8") {
        8
    } else if java_version_string.contains("1.7") {
        7
    } else {
        0 // 未知版本
    }
}

/// 搜索可能存在 Java 的地方找到可用的 Java 运行时
/// 返回的结果列表项均为 java.exe/javaw.exe 或 java 执行文件的目录
pub async fn search_for_java() -> Vec<String> {
    // 从安装目录中搜索 Java
    async fn check_bin_java_directory(path: impl AsRef<Path>, result: &mut Vec<String>) {
        if let Ok(mut d) = inner_future::fs::read_dir(path).await {
            while let Ok(Some(d)) = d.try_next().await {
                let mut path = d.path();
                #[cfg(target_os = "macos")]
                path.push("Contents/Home");
                path.push("bin");
                #[cfg(target_os = "windows")]
                {
                    path.push("java.exe");
                    if path.is_file() {
                        tracing::debug!("Added {}", path.display());
                        result.push(d.path().to_string_lossy().to_string());
                    }
                    {
                        path.pop();
                        path.push("javaw.exe");
                        if path.is_file() {
                            tracing::debug!("Added {}", path.display());
                            result.push(path.to_string_lossy().to_string());
                        }
                    }
                }
                #[cfg(not(windows))]
                {
                    path.push("java");
                    if path.is_file() {
                        result.push(path.to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    fn check_java(path: impl Into<PathBuf>, result: &mut Vec<String>) {
        let mut path: PathBuf = path.into();
        #[cfg(windows)]
        {
            path.push("java.exe");
            if path.is_file() {
                result.push(path.to_string_lossy().to_string());
            }
            path.pop();
            path.push("javaw.exe");
            if path.is_file() {
                result.push(path.to_string_lossy().to_string());
            }
        }
        #[cfg(not(windows))]
        {
            path.push("java");
            if path.is_file() {
                result.push(path.to_string_lossy().to_string());
            }
        }
    }
    #[cfg(windows)]
    {
        let mut result = Vec::with_capacity(16);
        // 从注册表中搜索 JavaHome
        use winreg::{enums::*, *};
        fn search_local_machine_reg_value(key: &str, result: &mut Vec<String>) {
            let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
            if let Ok(subkey) = hklm
                .open_subkey("SOFTWARE\\JavaSoft")
                .and_then(|subkey| subkey.open_subkey(key))
            {
                for s in subkey.enum_keys().flatten() {
                    let subkey = subkey.open_subkey(s);
                    if let Ok(subkey) = subkey {
                        if let Ok(v) = subkey.get_value("JavaHome") {
                            result.push(v);
                        }
                    }
                }
            }
        }
        search_local_machine_reg_value("Java Runtime Environment", &mut result);
        search_local_machine_reg_value("Java Development Kit", &mut result);
        search_local_machine_reg_value("JRE", &mut result);
        search_local_machine_reg_value("JDK", &mut result);
        async fn list_java_directory(path: &str, result: &mut Vec<String>) {
            let mut path = std::path::PathBuf::from(path);
            path.push("Java");
            check_bin_java_directory(&path, result).await;
            path.pop();
            path.push("BellSoft");
            check_bin_java_directory(&path, result).await;
            path.pop();
            path.push("AdoptOpenJDK");
            check_bin_java_directory(&path, result).await;
            path.pop();
            path.push("Zulu");
            check_bin_java_directory(&path, result).await;
            path.pop();
            path.push("Microsoft");
            check_bin_java_directory(&path, result).await;
            path.pop();
            path.push("Eclipse Foundation");
            check_bin_java_directory(&path, result).await;
            path.pop();
            path.push("Semeru");
            check_bin_java_directory(&path, result).await;
            path.pop();
        }
        if let Ok(program_files) = std::env::var("ProgramFiles")
            .or_else::<std::env::VarError, _>(|_| Ok("C:\\Program Files".into()))
        {
            list_java_directory(&program_files, &mut result).await;
        }
        if let Ok(program_files) = std::env::var("ProgramFiles(x86)")
            .or_else::<std::env::VarError, _>(|_| Ok("C:\\Program Files (x86)".into()))
        {
            list_java_directory(&program_files, &mut result).await;
            let mut minecraft_launcher_dir = PathBuf::from(program_files);
            minecraft_launcher_dir
                .push("Minecraft Launcher\\runtime\\jre-legacy\\windows-x64\\jre-legacy\\bin");
            check_java(&minecraft_launcher_dir, &mut result);
            minecraft_launcher_dir.pop();
            minecraft_launcher_dir.pop();
            minecraft_launcher_dir.pop();
            minecraft_launcher_dir.push("windows-x86\\jre-legacy\\bin");
            check_java(&minecraft_launcher_dir, &mut result);
            minecraft_launcher_dir.pop();
            minecraft_launcher_dir.pop();
            minecraft_launcher_dir.pop();
            minecraft_launcher_dir.pop();
            minecraft_launcher_dir.push("java-runtime-alpha\\windows-x64\\java-runtime-alpha\\bin");
            check_java(&minecraft_launcher_dir, &mut result);
            minecraft_launcher_dir.pop();
            minecraft_launcher_dir.pop();
            minecraft_launcher_dir.pop();
            minecraft_launcher_dir.push("windows-x86\\java-runtime-alpha\\bin");
            check_java(&minecraft_launcher_dir, &mut result);
        }
        if let Ok(program_files) = std::env::var("ProgramFiles(ARM)")
            .or_else::<std::env::VarError, _>(|_| Ok("C:\\Program Files (ARM)".into()))
        {
            list_java_directory(&program_files, &mut result).await;
        }
        // 从环境变量中查询 java.exe
        if let Ok(paths) = std::env::var("PATH") {
            for path in paths.split(';') {
                check_java(path, &mut result);
            }
        }
        // 从 JABBA 中查询 java.exe
        if let Ok(path) = std::env::var("JABBA_HOME") {
            let path = PathBuf::from(path).join("jdk");
            check_bin_java_directory(path.as_path(), &mut result).await;
        }
        let mut result: Vec<_> = result
            .into_iter()
            .map(|mut a| {
                #[cfg(windows)]
                {
                    if !a.ends_with("java.exe")
                        && !a.ends_with("javaw.exe")
                        && !a.ends_with("java")
                        && !a.ends_with("javaw")
                    {
                        if !a.ends_with('\\') && !a.ends_with('/') {
                            a.push('\\');
                        }
                        a.push_str("bin\\java.exe");
                    }
                }
                #[cfg(not(windows))]
                {
                    if !a.ends_with("java") {
                        if !a.ends_with('/') {
                            a.push('/');
                        }
                        a.push_str("bin/java");
                    }
                }
                a
            })
            .collect();
        result.sort();
        result.dedup();
        result
    }
    #[cfg(target_os = "linux")]
    {
        let mut result = Vec::with_capacity(16);

        // 一些常见目录
        check_bin_java_directory("/usr/java/bin", &mut result).await;
        check_bin_java_directory("/usr/lib/jvm/bin", &mut result).await;
        check_bin_java_directory("/usr/lib32/jvm/bin", &mut result).await;

        // 从环境变量中查询 java.exe
        if let Ok(paths) = std::env::var("PATH") {
            for path in paths.split(';') {
                check_java(path, &mut result);
            }
        }
        // 从 JABBA 中查询 java.exe
        if let Ok(path) = std::env::var("JABBA_HOME") {
            let path = PathBuf::from(path).join("jdk");
            check_bin_java_directory(path.as_path(), &mut result).await;
        }
        // 主目录里的 .minecraft/runtime 文件夹
        if let Some(mut home_path) = dirs::home_dir() {
            home_path.push(".minecraft/runtime/jre-legacy/linux/jre-legacy/bin");
            check_java(&home_path, &mut result);
            home_path.push("../../../../java-runtime-alpha/linux/java-runtime-alpha/bin");
            check_java(&home_path, &mut result);
        }

        result
    }
    #[cfg(target_os = "macos")]
    {
        let mut result = Vec::with_capacity(16);

        check_bin_java_directory("/Library/Java/JavaVirtualMachines", &mut result).await;
        check_bin_java_directory("/System/Library/Java/JavaVirtualMachines", &mut result).await;
        check_java(
            "/Library/Internet Plug-Ins/JavaAppletPlugin.plugin/Contents/Home/bin/java",
            &mut result,
        );
        check_java("/Applications/Xcode.app/Contents/Applications/Application Loader.app/Contents/MacOS/itms/java/bin/java", &mut result);

        result
    }
}

/// 根据 Java 的输出裁剪出版本号文本
fn query_java_version(java_output: &str) -> &str {
    fn parser(input: &str) -> nom::IResult<&str, &str> {
        nom::bytes::complete::take_until("\"")(input)
    }
    let pat = "version \"";
    if let Some(p) = java_output.find(pat) {
        if let Ok((_, r)) = parser(&java_output[p + pat.len()..]) {
            r
        } else {
            ""
        }
    } else {
        ""
    }
}
