# SCL WebView

一个用于 SCL 的微软正版登录的 WebView 框架，通过检测链接的跳转确认。

- 在 Windows 7+ 上使用 MSEdge WebView2 Runtime
- 在 Linux 上使用 Webkit2GTK
- 在 MacOS 上使用 WebKit

目前已知问题：

- MacOS 上的 WebKit 渲染进程在 WebView 关闭后仍然存在并占用内存
