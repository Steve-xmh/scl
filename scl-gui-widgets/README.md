## SCL GUI Animation

SCL Druid 组件库，其组件风格大量参考了 WinUI 3 和 Fluent Design 设计。

有部分组件是从 [linebender/druid-widget-nursery](https://github.com/linebender/druid-widget-nursery)
这个项目移植过来并修改成了大体符合 SCL 需求的组件们。在这里感谢他们的组件，原仓库是使用
MIT / Apache 2.0 开源协议开源的。

### 部分功能

为了辅助 SCL 的部分组件更好地实现部分效果，作者对 Druid/Piet 做了二次开发，其中添加了一些特性（例如文字裁切，全局透明度等）。
为了保证和主线 Druid 的兼容性，组件库提供了一个 `druid-ext` 特性用于开启这些增加的特殊效果，但必须使用来自 [`Steve-xmh/druid`](https://github.com/Steve-xmh/druid) 和 [`Steve-xmh/piet`](https://github.com/Steve-xmh/piet) 分支改版方可正确编译。
