<img src="./assets/logo.svg" alt="scl-core logo" width="144" align="right">
<div align="left">
    <h1>Sharp Craft Launcher Open Source Project</h1>
    <span>
        An awsome lightweight, fast, clean launcher dependency framework, including launcher core library, launcher component library, Webview, etc! 
    </span>
</div>

![preview](https://user-images.githubusercontent.com/39523898/208238006-900bd5fe-f9f7-42a9-b726-da829162fbed.png)

![MSRV 1.75.0](https://img.shields.io/badge/MSRV-1.75.0-orange)

Written in Rust, fairly small memory footprint, excellent performence, and is optimized for binary size compression.

Note:the launcher only chinese version.

Cross-platform support for Windows, Linux, MacOS.

- Official website: [https://steve-xmh.github.io/scl](https://steve-xmh.github.io/scl)
- Develpment docs: [https://steve-xmh.github.io/scl/scl-docs](https://steve-xmh.github.io/scl/scl-docs)
- Design drawings: [https://www.figma.com/file/i2Sl8uD5nKS4dIki0yK29n/Sharp-Craft-Launcher-%E8%AE%BE%E8%AE%A1%E5%9B%BE](https://www.figma.com/file/i2Sl8uD5nKS4dIki0yK29n/Sharp-Craft-Launcher-%E8%AE%BE%E8%AE%A1%E5%9B%BE)
- Intro/Post(MineBBS): [https://www.minebbs.com/resources/sharp-craft-launcher-_-_.7177/](https://www.minebbs.com/resources/sharp-craft-launcher-_-_.7177/)
- Intro/Post(MCBBS): [https://www.mcbbs.net/thread-1223867-1-1.html](https://www.mcbbs.net/thread-1223867-1-1.html)
- Sourse code branch: [https://github.com/Steve-xmh/scl/tree/site](https://github.com/Steve-xmh/scl/tree/site)

## Source Code Architecture

- `scl-core`: [![](https://img.shields.io/badge/docs-passing-green)](https://steve-xmh.github.io/scl/scl-doc/scl_core/index.html)

    Launcher core library, including game start, game download, authentic login, mod download and more other function.
- `scl-webview`: [![](https://img.shields.io/badge/docs-passing-green)](https://steve-xmh.github.io/scl/scl-doc/scl_webview/index.html)

    Launcher Webview. A web browser library offers webview for Microsoft genuine login.
- `scl-macro`: [![](https://img.shields.io/badge/docs-passing-green)](https://steve-xmh.github.io/scl/scl-doc/scl_macro/index.html)

    Launcher procedural macro library, contains part of the procedural macro code for code generation, currently contains simple procedural macros for icon code generation
- `scl-gui-animation`: [![](https://img.shields.io/badge/docs-passing-green)](https://steve-xmh.github.io/scl/scl-doc/scl_gui_animation/index.html)

    The launcher graphic page animation function library, contains some convenient functions and tool classes for creating non-linear animations
- `scl-gui-widgets`: [![](https://img.shields.io/badge/docs-passing-green)](https://steve-xmh.github.io/scl/scl-doc/scl_gui_widgets/index.html)

    Launcher graphic page component library, base [Druid](https://github.com/linebender/druid) framework, provides a large number of graphic page components based on WinUI3 design specifications

## About Open Source LICENSE and Code Collaboration LICENSE

Combining Rust's single executable file feature, this SCL project uses the [LGPL 3.0 Open Source LICENSE](./LICENSE)  and eliminates static linking restrictions, please find more details in the [COPYING](./COPYING).

To put it simply, you can statically link the project's libraries without the need for open source.

Considering the development situation of SCL, if you need to contribute code to this repository, you will agree unconditionally by default [SteveXMH](https://github.com/Steve-xmh) Use the code you contributed to make profits and it is irrevocable. The ways of making profits include but are not limited to: afdian generation support, WeChat Alipay, etc.

Therefore, if you agree to this contribution agreement, please specify the following text in the remarks when submitting the PR for the first time (copy and paste, please replace the 'Github account ID' with your own Github account ID):

```
[Github account ID] unconditionally agrees [SteveXMH]（ https://github.com/Steve-xmh ）The code contributed by using [Github account ID] will be profitable in any form and will not be revoked.
```

## Version schedule

### 1.0 Schedule

- [x] 1.6+ vanilla version support
- [x] 1.6+ third party version support
- [x] 1.6+ download
- [x] Advanced settings and other options for game versions
    - [x] Mod management
    - [x] Custom startup parameters
- [x] 1.6+ Curseforge mod download
- [x] Modrinth mod download
- [x] Offline login
- [x] Authentic login（Mojang）
- [x] Authentic login（Microsoft）
- [x] third party login（Unified Pass）
- [x] third party login（Authlib-Injector）
- [x] download source change（BMCLAPI MCBBS MC）
- [x] more .minecraft folder
- [x] MacOS support
- [ ] Code structure optimization (currently, the code is still very messy and the naming is not very standardized)
- [ ] Visualization theme settings (postponed indefinitely due to Druid's memory leak bug)

### 2.0 Schedule

- [ ] Migrate UI to FLTK-RS
- [ ] Linux compiles use MUSL 
- [ ] Linux Leave only one Webkit2GTK dependency or make it a dynamic import
- [ ] Visualization theme settings
- [ ] Background tasks can be interrupted
- [ ] Application Volume Optimization

## Projects related to SCL

Here's a list of projects developed/secondarily developed by the authors, all of which are planned to be used in the development of the SCL launcher. Some of the projects use more relaxed or even CC0 open source license, so feel free to use them!

- [optifine-installer](https://github.com/Steve-xmh/optifine-installer): A command line installer module that can install almost all 1.7.2+Optifine, supports specifying the version name of the installation, and can be used for optimizing the installation automation of initiators. Open source using the CC0 open source protocol.
- [forge-install-bootstrapper](https://github.com/Steve-xmh/forge-install-bootstrapper): A revised version base  [bangbang93/forge-install-bootstrapper](https://github.com/bangbang93/forge-install-bootstrapper) , aim to support automated installation of all versions of the Forge (any version since 1.5.2 that provides )
- [alhc](https://github.com/Steve-xmh/alhc): A lightweight system under development asynchronous HTTP client framework, by calling the system's own framework to realize HTTP requests and asynchronous

## Support Me 

The author has been working on this project since January 2021, please give it a Star if you like it!

If you can, [Come website 'afdian' and support me](https://afdian.net/a/SteveXMH)！
