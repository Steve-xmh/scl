# SCL 的主题详细设置教程

## 要求

- 手
- 眼睛
- 脑袋
- 识字能力
- 理解能力
- 一个文本编辑器
- 了解如何编辑 JSON 数据

## 编辑

SCL 会在程序运行目录里写入文件 `.scl.json` 存储你的各种设置，在目前没有较为友好的图形设置页面的情况下，我们只能先手动编辑这个文件来修改主题配色/图标。

> 警告：如果文件格式修改有误，很有可能导致启动器读取错误而用默认配置覆盖掉你原有的配置文件！
> 故修改配置文件请注意格式正确哦！

`.scl.json` 的文件结构大概长这样：

```json
{
  "selected_version": "",
  "javas": {
    "selected_leagzy": "javaw",
    "selected_modern": "javaw",
    "javapaths": [
        // 此处忽略大量 Java 对象...
    ]
  },
  "wait_until_launched": true,
  "version_independent_method": "NoIndependent",
  "dont_close_window_when_launched": false,
  "hide_window_when_launched": false,
  "auto_check_when_first_start_failed": false,
  "auto_mem_size": true,
  "mem_size": 0,
  "theme": { // 我们要修改的主题设置就在这里
    "slide_page_animation": false,
    "icons_theme": {}
  },
  "download_source": "Default",
  "download_parallel_amount": 64,
  "client_token": "34228fd9-87c8-4876-8454-ed33693a2a9c"
}
```

可以看到有一个 `theme` 对象存储了一部分我们的主题配置信息，有一些不存在是因为被按照默认值处理了。

### 相关特殊类型

配置文件中的值除了字符串和数字之外还有以下特殊格式类型：

|格式名称|说明|
|:------|:--|
|SVG 路径字符串|一个合法的 SVG 绘图路径字符串，可以参考[此处文档](https://developer.mozilla.org/zh-CN/docs/Web/SVG/Tutorial/Paths)学习如何编写|
|颜色字符串|一个符合 `#RRGGBB` 或 `#RRGGBBAA` 格式的颜色字符串，支持透明颜色但不推荐（因为有可能会在部分情况下有瑕疵）|
|图标对象|一个有三个对象的数组 `[0xRRGGBBAA, 0xRRGGBBAA, "SVGPATH"]` 分别存储亮色模式时的 RGBA 颜色、黑暗模式时的 RGBA 颜色和进行奇偶填充使用的 SVG 路径字符串。注意颜色值必须是 RGBA 数字，而不是上面的颜色字符串！|

那么我们开始按以下清单和规则向里面增加东西吧！

### 修改 `theme` 对象

在 `theme` 有很多可供调节的颜色和属性：

|键名|类型|说明|
|:---|:---|:---|
|`dark_theme`|布尔值|是否使用黑暗模式|
|`slide_page_animation`|布尔值|是否使用左右滑动的切换页面的动画而不是缩放动画，这在你特别想用半透明配色时能让你背景看起来不会很突兀|
|`icons_theme`|对象|[参见下文](#修改图标的-themeicons_theme-对象)|
|`theme_background_image_path`|字符串|对窗口使用图片作为背景，此处写入绝对路径字符串或相对于启动器文件所在位置的相对路径字符串|
|`theme_color_primary`|颜色字符串|设定大部分控件的主要配色|
|`theme_color_secondary`|颜色字符串|设定大部分控件的次要配色|
|`theme_color_accent`|颜色字符串|设定大部分控件的主配色|
|`theme_color_accent_1`|颜色字符串|设定大部分控件的主配色的次级配色|
|`theme_color_accent_dark_1`|颜色字符串|设定大部分控件的主配色的次级暗色配色|
|`theme_color_accent_light_1`|颜色字符串|设定大部分控件的主配色的次级亮色配色|
|`theme_color_title_bar`|颜色字符串|设定窗口标题栏的背景颜色，你可以设置成透明（这样可以直接看到背景颜色）|
|`theme_color_background`|颜色字符串|设定窗口背景颜色，当使用了图片时颜色将会被图片覆盖|

### 修改图标的 `theme.icons_theme` 对象

在 `theme.icons_theme` 对象内可以设置不同情况下的图标颜色和填充路径形状，以下列出了可以修改的图标们：

|键名|类型|说明|
|:---|:---|:---|
|`empty`|图标对象|空白图标，不建议修改它|
|`versions`|图标对象|设定主页面下方按钮组左上的“版本列表”图标按钮的图标|
|`settings`|图标对象|设定主页面下方按钮组右下的“启动器设置”图标按钮的图标|
|`download`|图标对象|设定主页面下方按钮组左下的“游戏版本下载安装”图标按钮的图标|
|`mods`|图标对象|设定主页面下方按钮组右上的“游戏模组安装”图标按钮的图标|
|`curseforge`|图标对象|设定模组安装页面中的“Curseforge”模组平台图标|
|`modrinth`|图标对象|设定模组安装页面中的“Modrinth”模组平台图标|
|`folder`|图标对象|一个文件夹图标，用在了版本高级设置里右上角的打开版本所在文件夹图标按钮上|
|`search`|图标对象|一个搜索图标，用在了很多需要搜索功能的页面上|
|`game_vanilla`|图标对象|设定版本列表里被 SCL 识别为纯净版的版本左侧的图标和下载页面中选择纯净版本按钮左侧的图标|
|`game_forge`|图标对象|设定版本列表里被 SCL 识别为 Forge 模组版的版本左侧的图标和下载页面中选择 Forge 版本按钮左侧的图标|
|`game_fabric`|图标对象|设定版本列表里被 SCL 识别为 Fabric 模组版的版本左侧的图标和下载页面中选择 Fabric 版本按钮左侧的图标|
|`game_optifine`|图标对象|设定版本列表里被 SCL 识别为 Optifine 模组版的版本左侧的图标和下载页面中选择 Optifine 版本按钮左侧的图标|
|`login_offline`|图标对象|设定增加账户页面中使用离线登录的图标|
|`login_mojang`|图标对象|设定增加账户页面中使用 Mojang 登录的图标|
|`login_microsoft`|图标对象|设定增加账户页面中使用微软登录（嵌入式网页）的图标|
|`login_microsoft_manual`|图标对象|设定增加账户页面中使用手动微软登录（外部浏览器复制回调）的图标|
|`login_authlib`|图标对象|设定增加账户页面中使用 Authlib 第三方登录的图标|
|`sort_by_name`|图标对象|设定版本列表页面左上角的排序方式中按照名称排序的图标|
|`sort_by_version_release_date`|图标对象|设定版本列表页面左上角的排序方式中按照版本发布日期排序的图标|
|`sort_by_download_date`|图标对象|设定版本列表页面左上角的排序方式中按照安装时间排序的图标|
|`categoried_by_version_type`|图标对象|设定版本列表页面左上角的排序方式中按照版本类型分类的图标|
|`desktop`|图标对象|一个计算机图标，设定模组安装位置页面时保存到本地时的图标|
|`delete`|图标对象|一个垃圾箱删除图标，设定 Java 管理中删除 Java 实例的图标|
|`paint_brush`|图标对象|一个刷子图标，设定账户管理时的皮肤编辑图标|
|`java`|图标对象|设定设置页面里进入 Java 管理页面按钮的线描式图标|
|`java_fulled`|图标对象|设定设置页面里进入 Java 管理页面按钮的填充式图标|
|`java_8`|图标对象|设定 Java 管理页面中未被选中的 Java 8+ 运行时的图标|
|`java_8_fulled`|图标对象|设定 Java 管理页面中被选中的 Java 8+ 运行时的图标|
|`java_16`|图标对象|设定 Java 管理页面中未被选中的 Java 16+ 运行时的图标|
|`java_16_fulled`|图标对象|设定 Java 管理页面中被选中的 Java 16+ 运行时的图标|