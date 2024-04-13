<img src="./assets/logo.svg" alt="scl-core logo" width="144" align="right">
<div align="left">
    <h1>Sharp Craft Launcher Open Source Project</h1>
    <span>
        An extremely lightweight, fast, and concise launcher framework, including the launcher core library, launcher component library, launcher WebView framework, and more!
    </span>
</div>

![preview](https://user-images.githubusercontent.com/39523898/208238006-900bd5fe-f9f7-42a9-b726-da829162fbed.png)

![MSRV 1.75.0](https://img.shields.io/badge/MSRV-1.75.0-orange)

Written in the Rust programming language, with very low memory usage and excellent performance, and has made efforts to optimize binary size.

Note: Currently, this launcher only contains Chinese version.

Cross-platform support for Windows, Linux, MacOS.

- Official website: [https://steve-xmh.github.io/scl](https://steve-xmh.github.io/scl)
- Develpment docs: [https://steve-xmh.github.io/scl/scl-docs](https://steve-xmh.github.io/scl/scl-docs)
- Design drawings: [https://www.figma.com/file/i2Sl8uD5nKS4dIki0yK29n/Sharp-Craft-Launcher-%E8%AE%BE%E8%AE%A1%E5%9B%BE](https://www.figma.com/file/i2Sl8uD5nKS4dIki0yK29n/Sharp-Craft-Launcher-%E8%AE%BE%E8%AE%A1%E5%9B%BE)
- Introduction/Release (MineBBS): [https://www.minebbs.com/resources/sharp-craft-launcher-_-_.7177/](https://www.minebbs.com/resources/sharp-craft-launcher-_-_.7177/)
- Introduction/Release (MCBBS): [https://www.mcbbs.net/thread-1223867-1-1.html](https://www.mcbbs.net/thread-1223867-1-1.html)
- Official website source code branch: [https://github.com/Steve-xmh/scl/tree/site](https://github.com/Steve-xmh/scl/tree/site)

## Source Code Architecture

- `scl-core`: [![](https://img.shields.io/badge/docs-passing-green)](https://steve-xmh.github.io/scl/scl-doc/scl_core/index.html)
    The launcher core library, including game launching, game downloading, authentic login, mod downloading, and other game-related functionalities.
- `scl-webview`: [![](https://img.shields.io/badge/docs-passing-green)](https://steve-xmh.github.io/scl/scl-doc/scl_webview/index.html)

    The launcher WebView web browser library, providing a browser window for Microsoft genuine login.
- `scl-macro`: [![](https://img.shields.io/badge/docs-passing-green)](https://steve-xmh.github.io/scl/scl-doc/scl_macro/index.html)

    The launcher procedural macro library, containing some procedural macro code for code generation, currently including a simple procedural macro for generating icons
- `scl-gui-animation`: [![](https://img.shields.io/badge/docs-passing-green)](https://steve-xmh.github.io/scl/scl-doc/scl_gui_animation/index.html)

    he launcher graphical page animation function library, containing some functions and utility classes for creating non-linear animations.
- `scl-gui-widgets`: [![](https://img.shields.io/badge/docs-passing-green)](https://steve-xmh.github.io/scl/scl-doc/scl_gui_widgets/index.html)

    The launcher graphical page component library, based on the [Druid](https://github.com/linebender/druid) framework, providing a large number of graphical page components designed based on the WinUI3 design specifications.

## About Open Source License and Code Contribution Agreement

Combined with Rust's single executable file feature, this SCL project uses the [LGPL 3.0 Open Source LICENSE](./LICENSE) and exempts static linking restrictions. For details, please refer to [COPYING](./COPYING).

In simple terms, you can statically link the libraries of this project without the need to open source your code.

Considering the development situation of SCL itself, if you want to contribute code to this repository, you will automatically and unconditionally agree that [SteveXMH](https://github.com/Steve-xmh) can profit from your contributed code without the possibility of revocation. The methods of profiting include but are not limited to: afdian support, WeChat Pay, Alipay, etc.

If you agree to this contribution agreement, please include the following text in the comments when submitting your first pull request (you can copy and paste it, and replace [Github Account ID] with your own Github account ID):

```
[Github account ID] unconditionally agrees [SteveXMH]（ https://github.com/Steve-xmh ）The code contributed by using [Github account ID] will be profitable in any form and will not be revoked.
```

## Version schedule

### 1.0 Schedule

- [x] Pure version support for 1.6+.
- [x] Third-party version support for 1.6+.
- [x] Game downloads for 1.6+.
- [x] Advanced settings and other options for game versions.
    - [x] Mod management.
    - [x] Custom startup parameters.
- [x] Curseforge mod downloads for 1.6+.
- [x] Modrinth mod downloads.
- [x] Offline login.
- [x] Authentic login（Mojang）
- [x] Authentic login（Microsoft）
- [x] Third-party login (Unified Passport).
- [x] Third-party login (Authlib-Injector).
- [x] Changing download sources (BMCLAPI, MCBBS, MC).
- [x] Multiple .minecraft folders.
- [x] MacOS support.
- [ ] Code structure optimization (currently the code is messy and naming conventions are not well-defined).
- [ ] Visual theme settings (this feature is indefinitely postponed due to memory leak bugs in Druid).

### 2.0 Schedule

- [ ] Migrate UI to FLTK-RS.
- [ ] Linux compilation using MUSL.
- [ ] Reduce dependencies for Linux to only one Webkit2GTK or make it dynamically imported.
- [ ] Visualization theme settings.
- [ ] Interruptible background tasks.
- [ ] Application Volume Optimization.

## Projects related to SCL

Here are some projects related to SCL that have been developed or modified by the author. They are planned to be used in the development of the SCL launcher. Some of these projects use more permissive or even CC0 open source licenses, so feel free to use them!
- [optifine-installer](https://github.com/Steve-xmh/optifine-installer):  A command-line installer module that can install almost all Optifine versions from 1.7.2 +. It supports specifying the version name to be installed and can be used for automating Optifine installation in the launcher. It is released under the CC0 open source license.

- [forge-install-bootstrapper](https://github.com/Steve-xmh/forge-install-bootstrapper):
A modified version based on  [bangbang93/forge-install-bootstrapper](https://github.com/bangbang93/forge-install-bootstrapper) . The purpose is to support automated installation of all Forge versions with installers (since 1.5.2 or any version that provides an installer).
- [alhc](https://github.com/Steve-xmh/alhc): A lightweight system asynchronous HTTP client framework that utilizes the system's built-in frameworks to perform HTTP requests and achieve asynchronous behavior. It is currently under development.

## Support Me

The author has been working on this project since January 2021, please give he a Star if you like it!

If you can, [Come website 'afdian' and support me](https://afdian.net/a/SteveXMH)！
