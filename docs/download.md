---
hide:
  - toc
  - navigation
---

# 正式版本下载

以下是 1.1.0 正式版本的下载清单列表，由于跨平台实现难度大，所以版本有可能不能做到同步，或者是功能上会有所差异。

更新有可能并不及时，如需及时获取最新版本，欢迎加入 SCL 公开 QQ 群：[877328353](https://jq.qq.com/?_wv=1027&k=wY61QuOf)

[蓝奏云总下载链接](https://wwu.lanzouy.com/b07o64kxa) 密码：a0sh

| 目标系统                  | 文件名称                                                  | 备注                                                                                                                    |
| ------------------------- | --------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Windows 32 位             | SharpCraftLauncher-20230101-1.1.0-i686.exe                | 如果不清楚自己是什么架构，选择这个即可                                                                                  |
| Windows 64 位             | SharpCraftLauncher-20230101-1.1.0-x86_64.exe              |
| Windows ARM64             | SharpCraftLauncher-20230101-1.1.0-aarch64.exe             | 需要安装 [ARM64 版本的 Microsoft Visual C++ Redistributable](https://aka.ms/vs/17/release/vc_redist.arm64.exe) 作为依赖 |
| MacOS Intel/Apple Selicon | SharpCraftLauncher-20230101-1.1.0-universal-darwin.tar.gz | 该版本原生通用于两个平台，请勿使用转译执行，否则会影响到软件的相关功能                                                  |
| Linux x86_64              | SharpCraftLauncher-20230101-1.1.0-linux-x86_64.tar.gz     | 请参阅[特殊安装要求](#linux)确认依赖是否安装                                                                            |
| Linux aarch64             | SharpCraftLauncher-20220910-1.0.0-linux-aarch64.zip       | 请参阅[特殊安装要求](#linux)确认依赖是否安装                                                                            |
| Linux aarch64 AppImage    | SharpCraftLauncher-20220910-1.0.0-linux-aarch64.appimage  | 理论上本单文件执行程序不需要依赖，但如果遇到问题请参阅[特殊安装要求](#linux)再次确认                                    |
| 其它系统                  | /                                                         | 请自行根据命名方式选择需要的目标可执行文件                                                                              |

# 历史版本下载

以下是已经过时的 Beta 以及 DEMO 公开版本，供归档查阅使用。

[Beta 0.4.1 蓝奏云下载链接](https://wwu.lanzouy.com/iLEYv09l1b8b)

[Beta 版本蓝奏云总下载链接](https://wwu.lanzouy.com/b07neavof) 密码：474u

[DEMO 版本蓝奏云总下载链接](https://wwu.lanzouy.com/b07mrdope) 密码：a3xg

# 特殊安装要求

尽管 SCL 已经尽可能静态链接了所有可能的依赖，但是以下平台尚未能够完全实现 0 依赖，如果 SCL 没能在你的电脑上成功运行，请尝试检查以下步骤以尝试修复。

## Windows on ARM

1. WinARM 因构建环境问题，需要安装 [ARM64 版本的 Microsoft Visual C++ Redistributable](https://aka.ms/vs/17/release/vc_redist.arm64.exe) 作为依赖

## Linux

1. 需要安装 OpenSSL GTK+3.0 等依赖，在一般的 Linux 桌面发行版这些应该都是默认预装好了的，如果没有可以尝试使用你的系统软件包管理器安装这些依赖。
