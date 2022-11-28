//! 客户端结构，用于启动游戏
use std::{
    collections::HashMap,
    fmt::{Display, Formatter, Result},
    path::Path,
};

use inner_future::process::{Child, Command};

use super::{
    auth::structs::AuthMethod,
    version::structs::{Argument, VersionInfo},
};
use crate::{
    prelude::*,
    utils::{get_full_path, CLASSPATH_SEPARATOR, NATIVE_ARCH_LAZY, TARGET_OS},
    version::structs::{Allowed, VersionMeta},
};

/// 用于修复 CVE-2021-44228 远程代码执行漏洞
///
/// 似乎只需要加 `-Dlog4j2.formatMsgNoLookups=true` 参数到 `classpath` 之前就可以解决问题了
///
/// 一般用不到这个
pub const LOG4J_PATCH: &[u8] = include_bytes!("../assets/log4j-patch-agent-1.0.jar");

/// 一个客户端配置结构，开发者需要填充内部的一部分数据后传递给 [`Client::new`] 方可正确启动游戏
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// 使用的玩家账户
    pub auth: AuthMethod,
    /// 启动的版本元数据信息
    pub version_info: VersionInfo,
    /// 启动的版本类型
    pub version_type: String,
    /// 自定义 JVM 参数，这将会附加在 Class Path 之前的位置
    pub custom_java_args: Vec<String>,
    /// 自定义游戏参数，这将会附加在参数的最后部分
    pub custom_args: Vec<String>,
    /// 需要使用的 Java 运行时文件路径
    pub java_path: String,
    /// 最高内存，以 MB 为单位
    pub max_mem: u32,
    /// 是否进行预先资源及依赖检查
    pub recheck: bool,
}

/// 一个客户端结构，通过 [`ClientConfig`] 提供的信息组合启动参数，运行游戏
pub struct Client {
    /// 客户端的实际指令对象
    pub cmd: Command,
    /// 是否使用批处理登录，仅 Windows 可用
    pub terminal_launch: bool,
    /// 当前游戏目录路径
    pub game_dir: String,
    /// 当前使用的 Java 运行时路径
    pub java_path: String,
    /// 当前启动参数的副本，包含 Java 自身
    pub args: Vec<String>,
    /// 正在运行的进程对象
    pub process: Option<Child>,
}

fn get_game_directory(cfg: &ClientConfig) -> String {
    let version_base = std::path::Path::new(&cfg.version_info.version_base);
    let version_dir = version_base.join(&cfg.version_info.version);
    let version_dir = get_full_path(version_dir);
    let game_dir = version_base.parent().unwrap();
    let game_dir = get_full_path(game_dir);
    if let Some(_meta) = &cfg.version_info.meta {
        if let Some(scl) = &cfg.version_info.scl_launch_config {
            if scl.game_independent {
                version_dir
            } else {
                game_dir
            }
        } else {
            game_dir
        }
    } else {
        game_dir
    }
}

async fn parse_inheritsed_meta(cfg: &ClientConfig) -> VersionMeta {
    let meta = cfg.version_info.meta.as_ref().unwrap();
    let inherits_from = if !meta.inherits_from.is_empty() {
        meta.inherits_from.as_str()
    } else if !meta.client_version.is_empty() && cfg.version_info.version != meta.client_version {
        meta.client_version.as_str()
    } else {
        ""
    };
    if inherits_from.is_empty() {
        meta.to_owned()
    } else {
        let meta = meta.to_owned();
        let mut base_info = VersionInfo {
            version: inherits_from.to_owned(),
            version_base: cfg.version_info.version_base.to_owned(),
            ..Default::default()
        };
        if base_info.load().await.is_ok() {
            if let Some(base) = &mut base_info.meta {
                let mut base = base.to_owned();
                base += meta;
                base
            } else {
                meta.to_owned()
            }
        } else {
            meta
        }
    }
}

impl Client {
    /// 根据传入的启动客户端版本设定创建一个客户端
    ///
    /// 这将会检查元数据，并组合出启动参数，之后可以使用 [`Client::launch`] 启动游戏
    pub async fn new(mut cfg: ClientConfig) -> DynResult<Self> {
        if cfg.version_info.meta.is_none() {
            anyhow::bail!("version_info is empty");
        }
        // let build_args_timer = std::time::Instant::now();
        let mut args = Vec::<String>::with_capacity(64);

        // 检查 log4jc 是否被安装，否则将 LOG4J_PATCH 的数据复制到库文件夹里
        // if !cfg.version_info.version_base.is_empty() {
        //     let lib_path = std::path::Path::new(&cfg.version_info.version_base);
        //     let lib_path = lib_path
        //         .parent()
        //         .ok_or_else(|| anyhow::anyhow!("There's no parent from the library path"))?
        //         .join("libraries")
        //         .join("org")
        //         .join("glavo")
        //         .join("1.0")
        //         .join("log4j-patch");
        //     if !lib_path.is_dir() {
        //         inner_future::fs::create_dir_all(&lib_path).await?;
        //     }
        //     let log4j_path = lib_path.join("log4j-patch-agent-1.0.jar");
        //     if !log4j_path.is_file() {
        //         inner_future::fs::OpenOptions::new()
        //             .write(true)
        //             .create(true)
        //             .truncate(true)
        //             .open(&log4j_path)
        //             .await?
        //             .write_all(LOG4J_PATCH)
        //             .await?;
        //     }
        // }

        // 检查是否继承版本
        let meta = parse_inheritsed_meta(&cfg).await;

        cfg.version_info.meta = Some(meta.to_owned());

        // 变量集，用来给参数中 ${VAR} 做文本替换
        let mut variables: HashMap<&'static str, String> = HashMap::with_capacity(19);
        variables.insert("${library_directory}", {
            // crate::path::MINECRAFT_LIBRARIES_PATH.to_string()
            let lib_path = std::path::Path::new(&cfg.version_info.version_base);
            let lib_path = lib_path
                .parent()
                .ok_or_else(|| anyhow::anyhow!("There's no parent from the library path"))?
                .join("libraries");
            let lib_path = get_full_path(lib_path);
            lib_path.replace(|a| a == '/' || a == '\\', "/")
        });
        variables.insert("${classpath}", {
            // 类路径，所有的 jar 库
            // 先添加 libraries 的库，然后添加自身
            let lib_base_path = variables
                .get("${library_directory}")
                .unwrap()
                .replace('/', &std::path::MAIN_SEPARATOR.to_string());
            // 使用 HashMap 以将模组加载器中的 Jar 进行覆盖
            let mut lib_args: HashMap<String, String> =
                HashMap::with_capacity(meta.libraries.len());
            for lib in &meta.libraries {
                let class_name = lib.name.as_str()
                    [0..lib.name.rfind(':').expect("Can't parse class name")]
                    .to_string();
                if !lib.rules.is_allowed() {
                    continue;
                }
                let lib_path = {
                    // 处理 name
                    let lib: Vec<&str> = lib.name.splitn(3, ':').collect();
                    let (package, name, version) = (lib[0], lib[1], lib[2]);
                    let package_path: Vec<&str> = package.split('.').collect();
                    format!(
                        "{}{sep}{}{sep}{}{sep}{}{sep}{}-{}.jar",
                        lib_base_path,
                        package_path.join(&std::path::MAIN_SEPARATOR.to_string()),
                        name,
                        version,
                        name,
                        version,
                        sep = std::path::MAIN_SEPARATOR,
                    )
                };
                let mut lib_path = if let Some(ds) = &lib.downloads {
                    if let Some(d) = &ds.artifact {
                        // 使用 artifact.path
                        format!(
                            "{}{sep}{}",
                            lib_base_path,
                            d.path.replace(
                                |a| a == '/' || a == '\\',
                                &std::path::MAIN_SEPARATOR.to_string()
                            ),
                            sep = std::path::MAIN_SEPARATOR
                        )
                    } else {
                        lib_path
                    }
                } else {
                    lib_path
                };
                if let Some(n) = &lib.natives {
                    if false {
                        if let Some(native_key) = n.get(TARGET_OS) {
                            let native_key =
                                native_key.replace("${arch}", NATIVE_ARCH_LAZY.as_ref());
                            let classifier = lib
                                .downloads
                                .as_ref()
                                .ok_or_else(|| {
                                    anyhow::anyhow!("No downloads struct for {}", &native_key)
                                })?
                                .classifiers
                                .as_ref()
                                .ok_or_else(|| {
                                    anyhow::anyhow!("No classifiers struct for {}", &native_key)
                                })?
                                .get(&native_key)
                                .ok_or_else(|| {
                                    anyhow::anyhow!("No classifier struct for {}", &native_key)
                                })?;
                            lib_path += CLASSPATH_SEPARATOR;
                            lib_path += &lib_base_path;
                            lib_path.push(std::path::MAIN_SEPARATOR);
                            lib_path += &classifier.path.replace(
                                |a| a == '/' || a == '\\',
                                &std::path::MAIN_SEPARATOR.to_string(),
                            );
                            // TODO: 解压原生库
                        }
                    }
                    // 可能有原生库要添加
                }
                lib_args.insert(class_name, lib_path);
            }

            let lib_args: Vec<_> =
                if meta.main_class == "cpw.mods.bootstraplauncher.BootstrapLauncher" {
                    // 新版 Forge 不再需要引入版本文件夹下的 jar 文件了
                    lib_args.into_iter().map(|x| x.1).collect()
                } else {
                    lib_args
                        .into_iter()
                        .map(|x| x.1)
                        .chain(meta.main_jars.iter().map(|a| {
                            a.replace(
                                |a| a == '/' || a == '\\',
                                &std::path::MAIN_SEPARATOR.to_string(),
                            )
                        }))
                        .collect()
                };

            #[cfg(target_os = "windows")]
            {
                lib_args.join(";")
            }
            #[cfg(target_os = "linux")]
            {
                lib_args.join(":")
            }
            #[cfg(target_os = "macos")]
            {
                lib_args.join(":")
            }
        });
        variables.insert("${max_memory}", format!("-Xmx{}m", cfg.max_mem));
        variables.insert(
            "${auth_player_name}",
            match &cfg.auth {
                AuthMethod::Offline { player_name, .. } => player_name.to_owned(),
                AuthMethod::Mojang { player_name, .. } => player_name.to_owned(),
                AuthMethod::Microsoft { player_name, .. } => player_name.to_owned(),
                AuthMethod::AuthlibInjector { player_name, .. } => player_name.to_owned(),
            },
        );
        variables.insert(
            "${natives_directory}",
            get_full_path(Path::new(&format!(
                "{}{sep}{ver}{sep}natives",
                cfg.version_info.version_base,
                ver = cfg.version_info.version,
                sep = std::path::MAIN_SEPARATOR,
            ))),
        );
        variables.insert("${version_name}", cfg.version_info.version.to_owned());
        variables.insert("${classpath_separator}", CLASSPATH_SEPARATOR.to_owned());
        variables.insert("${game_directory}", get_game_directory(&cfg));
        variables.insert("${assets_root}", {
            let assets_path = std::path::Path::new(&cfg.version_info.version_base);
            let assets_path = assets_path.parent().unwrap().join("assets");
            if cfg
                .version_info
                .meta
                .as_ref()
                .map(|x| {
                    x.asset_index
                        .as_ref()
                        .map(|x| &x.id == "pre-1.6")
                        .unwrap_or_default()
                })
                .unwrap_or_default()
            {
                let assets_path = assets_path.join("virtual").join("pre-1.6");
                get_full_path(assets_path)
            } else {
                get_full_path(assets_path)
            }
        });
        variables.insert(
            "${game_assets}",
            variables.get("${assets_root}").unwrap().to_owned(),
        );
        variables.insert(
            "${assets_index_name}",
            if let Some(asset_index) = &meta.asset_index {
                asset_index.id.to_owned()
            } else {
                String::new()
            },
        );
        variables.insert("${auth_session}", "token:0".into());
        variables.insert("${clientid}", "00000000402b5328".into());
        variables.insert(
            "${auth_access_token}",
            match &cfg.auth {
                AuthMethod::Offline { uuid, .. } => uuid.to_owned(),
                AuthMethod::Mojang { access_token, .. } => access_token.to_owned_string(),

                AuthMethod::Microsoft { access_token, .. } => access_token.to_owned_string(),
                AuthMethod::AuthlibInjector { access_token, .. } => access_token.to_owned_string(),
            },
        );
        variables.insert(
            "${auth_uuid}",
            match &cfg.auth {
                AuthMethod::Offline { uuid, .. } => uuid.to_owned(),
                AuthMethod::Mojang { uuid, .. } => uuid.to_owned(),
                AuthMethod::Microsoft { uuid, .. } => uuid.to_owned(),
                AuthMethod::AuthlibInjector { uuid, .. } => uuid.to_owned(),
            },
        );
        variables.insert(
            "${user_type}",
            match &cfg.auth {
                AuthMethod::Offline { .. } => "Legacy".into(),
                AuthMethod::Mojang { .. } | AuthMethod::AuthlibInjector { .. } => "Mojang".into(),
                AuthMethod::Microsoft { uuid: _, .. } => "Mojang".into(),
            },
        );
        variables.insert("${version_type}", cfg.version_type.to_owned());
        variables.insert("${user_properties}", "{}".into());
        variables.insert("${launcher_name}", "SharpCraftLauncher".into());
        variables.insert("${launcher_version}", "221".into());

        fn replace_each(variables: &HashMap<&'static str, String>, arg: String) -> String {
            let mut arg = arg;
            for (k, v) in variables {
                if arg.contains(*k) {
                    arg = arg.replace(*k, v);
                }
            }
            arg
        }

        // --- 组装参数
        // -- JVM 参数
        // JVM 参数
        // Encoding
        args.push("-Dfile.encoding=UTF-8".into());

        // 禁用 JNDI
        args.push("-Dlog4j2.formatMsgNoLookups=true".into());

        // 注入 log4j-patch

        // args.push(format!("-javaagent:{}", {
        //     let lib_path = std::path::Path::new(&cfg.version_info.version_base);
        //     let lib_path = lib_path
        //         .parent()
        //         .ok_or_else(|| anyhow::anyhow!("There's no parent from the library path"))?
        //         .join("libraries")
        //         .join("org")
        //         .join("glavo")
        //         .join("1.0")
        //         .join("log4j-patch")
        //         .join("log4j-patch-agent-1.0.jar");
        //     lib_path.to_string_lossy().to_string()
        // }));

        // 用户自定义JVM参数
        if let Some(scl_config) = &cfg.version_info.scl_launch_config {
            if !scl_config.jvm_args.is_empty() {
                args.push(scl_config.jvm_args.to_owned());
            }
        }

        if let AuthMethod::AuthlibInjector {
            api_location,
            server_meta,
            ..
        } = &cfg.auth
        {
            // 注入 Authlib Injector
            let authlib_injector_path = get_full_path(format!(
                "{}{sep}..{sep}authlib-injector.jar",
                cfg.version_info.version_base,
                sep = std::path::MAIN_SEPARATOR
            ));
            args.push(format!(
                "-javaagent:{}={}",
                authlib_injector_path, api_location
            ));
            args.push(format!(
                "-Dauthlibinjector.yggdrasil.prefetched={}",
                server_meta
            ));
        }
        if let Some(max_mem) = variables.get("${max_memory}") {
            args.push(max_mem.to_owned());
        }

        if let Some(arguments) = &meta.arguments {
            for arg in &arguments.jvm {
                match arg {
                    Argument::Common(arg) => args.push(replace_each(&variables, arg.to_owned())),
                    Argument::Specify(arg) => {
                        if arg.rules.is_allowed() {
                            for value in arg.value.iter() {
                                args.push(value.to_owned())
                            }
                        }
                    }
                }
            }
        } else {
            // 以前的 MC 元数据不包含 JVM 参数，所以咱还得手动加
            // Native Library Path
            args.push(format!(
                "-Djava.library.path={}",
                variables.get("${natives_directory}").unwrap()
            ));
            // Launcher name & version
            args.push(format!(
                "-Dminecraft.launcher.brand={}",
                variables.get("${launcher_name}").unwrap()
            ));
            args.push(format!(
                "-Dminecraft.launcher.version={}",
                variables.get("${launcher_version}").unwrap()
            ));
            // Class Path
            args.push("-cp".into());
            args.push(variables.get("${classpath}").unwrap().to_owned());
        }

        // 游戏主类
        args.push(meta.main_class.to_owned());

        fn dedup_argument(args: &mut Vec<String>, arg: &String) -> bool {
            let exist_arg = args.iter().enumerate().find(|x| x.1 == arg).map(|x| x.0);
            if let Some(exist_arg) = exist_arg {
                args.remove(exist_arg);
                if arg.starts_with('-') {
                    // 将附带参数一并删除
                    args.remove(exist_arg);
                }
                true
            } else {
                false
            }
        }

        // 游戏参数 旧版本 使用 minecraftArgument
        let splited = meta.minecraft_arguments.trim().split(' ');
        let mut skip_next_dedup = false;
        for arg in splited {
            if !arg.is_empty() {
                let arg = replace_each(&variables, arg.to_owned());
                if skip_next_dedup {
                    skip_next_dedup = false
                } else {
                    skip_next_dedup = dedup_argument(&mut args, &arg);
                }
                args.push(arg);
            }
        }

        // 游戏参数
        if let Some(arguments) = &meta.arguments {
            let mut skip_next_dedup = false;
            for arg in &arguments.game {
                match arg {
                    Argument::Common(arg) => {
                        let arg = replace_each(&variables, arg.to_owned());
                        if skip_next_dedup {
                            skip_next_dedup = false
                        } else {
                            skip_next_dedup = dedup_argument(&mut args, &arg);
                        }
                        args.push(arg);
                    }
                    Argument::Specify(_) => {
                        // TODO: 是否为试玩版，自定义窗口大小等自定义参数
                    }
                }
            }
        }

        // 用户自定义游戏参数
        if let Some(scl_config) = &cfg.version_info.scl_launch_config {
            if !scl_config.game_args.is_empty() {
                args.push(scl_config.game_args.to_owned());
            }
        }

        let java_path = if let Some(scl_config) = &cfg.version_info.scl_launch_config {
            if scl_config.java_path.is_empty() {
                cfg.java_path.to_owned()
            } else {
                scl_config.java_path.to_owned()
            }
        } else {
            cfg.java_path.to_owned()
        };

        let wrapper_path = cfg
            .version_info
            .scl_launch_config
            .as_ref()
            .map(|x| x.wrapper_path.to_owned())
            .unwrap_or_default();

        let mut cmd = if wrapper_path.is_empty() {
            Command::new(&java_path)
        } else {
            let mut cmd = Command::new(&wrapper_path);

            let wrapper_args = cfg
                .version_info
                .scl_launch_config
                .as_ref()
                .map(|x| x.wrapper_args.to_owned())
                .unwrap_or_default();

            if !wrapper_args.is_empty() {
                cmd.arg(wrapper_args);
            }

            cmd.arg(&java_path);
            cmd
        };

        cmd.args(&args);
        cmd.current_dir(get_game_directory(&cfg));
        #[cfg(target_os = "windows")]
        {
            cmd.env("APPDATA", get_game_directory(&cfg));
        }
        cmd.env("FORMAT_MESSAGES_PATTERN_DISABLE_LOOKUPS", "true");

        args.insert(0, java_path.to_owned());

        let terminal_launch = if let Some(scl_config) = &cfg.version_info.scl_launch_config {
            scl_config.use_terminal_launch
        } else {
            false
        };

        println!("CMD: {:?}", cmd);

        Ok(Self {
            cmd,
            terminal_launch,
            game_dir: get_game_directory(&cfg),
            java_path,
            args,
            process: None,
        })
    }

    /// 以 Builder 模式设置启动程序的标准输入方式
    ///
    /// 详情请参考 [`std::process::Command::stdin`]
    pub fn stdin(mut self, cfg: impl Into<std::process::Stdio>) -> Self {
        self.set_stdin(cfg);
        self
    }

    /// 以 Builder 模式设置启动程序的标准输出方式
    ///
    /// 详情请参考 [`std::process::Command::stdout`]
    pub fn stdout(mut self, cfg: impl Into<std::process::Stdio>) -> Self {
        self.set_stdout(cfg);
        self
    }

    /// 以 Builder 模式设置启动程序的标准错误输出方式
    ///
    /// 详情请参考 [`std::process::Command::stderr`]
    pub fn stderr(mut self, cfg: impl Into<std::process::Stdio>) -> Self {
        self.set_stderr(cfg);
        self
    }

    /// 设置启动程序的标准输入方式
    ///
    /// 详情请参考 [`std::process::Command::stdin`]
    pub fn set_stdin(&mut self, cfg: impl Into<std::process::Stdio>) {
        self.cmd.stdin(cfg);
    }

    /// 设置启动程序的标准输出方式
    ///
    /// 详情请参考 [`std::process::Command::stdout`]
    pub fn set_stdout(&mut self, cfg: impl Into<std::process::Stdio>) {
        self.cmd.stdout(cfg);
    }

    /// 设置启动程序的标准错误输出方式
    ///
    /// 详情请参考 [`std::process::Command::stderr`]
    pub fn set_stderr(&mut self, cfg: impl Into<std::process::Stdio>) {
        self.cmd.stderr(cfg);
    }

    /// 拿出参数，参数数组的第一个成员为提供的 Java 执行文件
    pub fn take_args(self) -> Vec<String> {
        self.args
    }

    /// 拿出 Command
    pub fn take_cmd(self) -> Command {
        self.cmd
    }

    /// 启动游戏，并返回进程 ID
    pub async fn launch(&mut self) -> DynResult<u32> {
        let c = {
            #[cfg(target_os = "windows")]
            {
                use inner_future::process::windows::CommandExt;
                // 写入到 bat 再启动
                let args = self
                    .args
                    .iter()
                    .map(|a| {
                        if a.contains(' ') {
                            format!("\"{}\"", a)
                        } else {
                            a.to_owned()
                        }
                    })
                    .collect::<Vec<_>>();
                let batdata = format!(
                    "\
                    @chcp 65001\r\n\
                    @echo off\r\n\
                    :: 这是 Sharp Craft Launcher 为启动游戏创建的启动脚本\r\n\
                    :: 如果你的游戏启动出现了问题，你可以尝试手动运行这个脚本以确认原因\r\n\
                    set APPDATA=\"{gamedir}\"\r\n\
                    set CURRENT=\"%~dp0\"\r\n\
                    cd /d \"{gamedir}\"\r\n\
                    {args}\r\n\
                    cd /d %CURRENT%\r\n\
                    pause\r\n\
                ",
                    gamedir = self.game_dir,
                    args = args.join(" ")
                );
                use futures::AsyncWriteExt;
                use inner_future::fs::windows::OpenOptionsExt;
                let mut file = inner_future::fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .attributes(0x2)
                    .open(".scl.launch.bat")
                    .await?;
                file.write_all(batdata.as_bytes()).await?;
                if self.terminal_launch {
                    let _ = file.sync_all().await;
                    let _ = file.sync_data().await;
                    drop(file);
                    inner_future::process::Command::new(".scl.launch.bat")
                        // .stdout(Stdio::piped())
                        // .stderr(Stdio::piped())
                        .creation_flags(0x08000000 | 0x00000400)
                        .spawn()?
                } else {
                    match self
                        .cmd
                        // .stdout(Stdio::piped())
                        // .stderr(Stdio::piped())
                        .creation_flags(0x08000000)
                        .spawn()
                    {
                        Ok(c) => c,
                        Err(e) => {
                            if e.kind() == std::io::ErrorKind::NotFound {
                                anyhow::bail!("使用 Java {} 启动游戏时发生错误：找不到 Java 执行文件，请确认你的 Java 文件是否存在 {:?}", self.java_path, e)
                            } else {
                                anyhow::bail!(
                                    "使用 Java {} 启动游戏时发生错误 {:?}",
                                    self.java_path,
                                    e
                                )
                            }
                        }
                    }
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                match self.cmd.spawn() {
                    Ok(c) => c,
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::NotFound {
                            anyhow::bail!("启动游戏时发生错误：找不到 Java 执行文件，请确认你的 Java 文件是否存在 {:?}", e)
                        } else {
                            anyhow::bail!("启动游戏时发生错误 {:?}", e)
                        }
                    }
                }
            }
        };
        let pid = c.id();
        self.process = Some(c);
        Ok(pid)
    }

    /// 如果游戏进程还在运行，则尝试停止游戏进程
    pub fn stop(&mut self) -> DynResult {
        if let Some(mut p) = self.process.take() {
            p.kill()?;
        }
        Ok(())
    }
}

impl Display for Client {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let running = if self.process.is_some() {
            "running"
        } else {
            "idle"
        };
        write!(f, "[MCClient {} args={:?}]", running, self.args)
    }
}
