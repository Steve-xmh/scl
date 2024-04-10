[English](./README.EN.md)
<img src="./assets/logo.svg" alt="scl-core logo" width="144" align="right">
<div align="left">
    <h1>Sharp Craft Launcher Open Source Project</h1>
    <span>
        一个无比轻量，迅速，简洁的启动器的附属框架，包括启动器核心库，启动器组件库，启动器 WebView 框架还有更多！
    </span>
</div>

![预览图](https://user-images.githubusercontent.com/39523898/208238006-900bd5fe-f9f7-42a9-b726-da829162fbed.png)

![MSRV 1.75.0](https://img.shields.io/badge/MSRV-1.75.0-orange)


使用 Rust 编程语言编写，内存占用相当之小，性能相当之优秀，针对二进制大小做了力所能及的压缩优化。

原生跨平台，支持 Windows，Linux，MacOS 三大主流操作系统。

- 官网：[https://steve-xmh.github.io/scl](https://steve-xmh.github.io/scl)
- 开发文档：[https://steve-xmh.github.io/scl/scl-docs](https://steve-xmh.github.io/scl/scl-docs)
- 设计图：[https://www.figma.com/file/i2Sl8uD5nKS4dIki0yK29n/Sharp-Craft-Launcher-%E8%AE%BE%E8%AE%A1%E5%9B%BE](https://www.figma.com/file/i2Sl8uD5nKS4dIki0yK29n/Sharp-Craft-Launcher-%E8%AE%BE%E8%AE%A1%E5%9B%BE)
- 介绍/发布贴（MineBBS）：[https://www.minebbs.com/resources/sharp-craft-launcher-_-_.7177/](https://www.minebbs.com/resources/sharp-craft-launcher-_-_.7177/)
- 介绍/发布贴（MCBBS）：[https://www.mcbbs.net/thread-1223867-1-1.html](https://www.mcbbs.net/thread-1223867-1-1.html)
- 官网源代码分支：[https://github.com/Steve-xmh/scl/tree/site](https://github.com/Steve-xmh/scl/tree/site)

## 源代码架构

- `scl-core`: [![](https://img.shields.io/badge/docs-passing-green)](https://steve-xmh.github.io/scl/scl-doc/scl_core/index.html) 启动器核心库，包含了游戏启动，游戏下载，正版登录，模组下载等游戏操作功能
- `scl-webview`: [![](https://img.shields.io/badge/docs-passing-green)](https://steve-xmh.github.io/scl/scl-doc/scl_webview/index.html) 启动器 WebView 网页浏览器库，提供了用于微软正版登录的浏览器窗口
- `scl-macro`: [![](https://img.shields.io/badge/docs-passing-green)](https://steve-xmh.github.io/scl/scl-doc/scl_macro/index.html) 启动器过程宏库，包含了部分用于代码生成的过程宏代码，目前包含图标代码生成的简易过程宏
- `scl-gui-animation`: [![](https://img.shields.io/badge/docs-passing-green)](https://steve-xmh.github.io/scl/scl-doc/scl_gui_animation/index.html) 启动器图形页面动画函数库，包含了一些方便用来制作非线性动画的函数和工具类
- `scl-gui-widgets`: [![](https://img.shields.io/badge/docs-passing-green)](https://steve-xmh.github.io/scl/scl-doc/scl_gui_widgets/index.html) 启动器图形页面组件库，基于 [Druid](https://github.com/linebender/druid) 框架，提供了大量基于 WinUI3 设计规范制作的图形页面组件

## 关于开源协议和代码协作协议

结合 Rust 本身的单执行文件特性，本 SCL 项目使用 [LGPL 3.0 开源协议](./LICENSE) 并免除静态链接的限制，详情可以查阅 [COPYING](./COPYING) 或者 [参考译文](./COPYING-CN)

使用简单的说就是，你可以以静态链接本项目的库而不需要开放源代码。

考虑到 SCL 的自身开发情况，如果你需要贡献代码到本仓库，你将默认无条件同意 [SteveXMH](https://github.com/Steve-xmh) 使用你所贡献的代码盈利且不可撤销，盈利方式包括但不限于：爱发电支持，微信支付宝等。

故如果同意本贡献协议，请在第一次提交 PR 时在备注中写明以下文字（可复制粘贴，请将`[Github账户ID]`更换成自己的 Github 账户 ID）：

```
[Github账户ID]无条件同意[SteveXMH](https://github.com/Steve-xmh)使用[Github账户ID]所贡献的代码以任何形式盈利且不会撤销。
```

## 版本计划表

### 1.0 计划表

- [x] 1.6+ 的纯净版本支持
- [x] 1.6+ 的第三方版本支持
- [x] 1.6+ 游戏下载
- [x] 游戏版本高级设置及其它选项
    - [x] 模组管理
    - [x] 自定义启动参数
- [x] 1.6+ Curseforge 模组下载
- [x] Modrinth 模组下载
- [x] 离线登录
- [x] 正版登录（Mojang）
- [x] 正版登录（Microsoft）
- [x] 第三方登录（统一通行证）
- [x] 第三方登录（Authlib-Injector）
- [x] 更换下载源（BMCLAPI MCBBS MC）
- [x] 多 .minecraft 文件夹
- [x] MacOS 支持
- [ ] 代码结构优化（目前代码还是很乱，命名也不太规范）
- [ ] 可视化主题设置（因 Druid 的内存泄露 BUG 无期限推迟此功能）

### 2.0 计划表

- [ ] 移植 UI 到 FLTK-RS
- [ ] Linux 使用 MUSL 编译
- [ ] Linux 只留一个 Webkit2GTK 依赖或做成动态导入
- [ ] 可视化主题设置
- [ ] 后台任务可中断
- [ ] 应用程序体积优化

## 与 SCL 有关联的项目

这里列出了由作者自行开发/二次开发的一些项目，它们都将计划用在 SCL 启动器的开发中。一部分项目使用的是更加宽松甚至是 CC0 的开源共享协议，所以请随意使用吧！

- [optifine-installer](https://github.com/Steve-xmh/optifine-installer): 一个可安装几乎所有 1.7.2+ Optifine 的命令行安装器模块，支持指定安装的版本名称，可以用于启动器的 Optifine 安装自动化。使用 CC0 开源协议开源。
- [forge-install-bootstrapper](https://github.com/Steve-xmh/forge-install-bootstrapper): 一个基于 [bangbang93/forge-install-bootstrapper](https://github.com/bangbang93/forge-install-bootstrapper) 的改版，目的是支持 Forge 全部版本安装器的自动化安装（自 1.5.2 以来的任何提供安装器的版本）
- [alhc](https://github.com/Steve-xmh/alhc): 一个正在开发中的轻量级系统异步 HTTP 客户端框架，通过调用系统自带的框架实现 HTTP 请求并实现异步

## 支持

作者自 2021 年 1 月开始做到了现在的项目，喜欢的话请给一个 Star 吧！

如果有能力的话，[来爱发电为我发电支持吧](https://afdian.net/a/SteveXMH)！
