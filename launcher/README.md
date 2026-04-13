# Clix Launcher — 原型

这是一个最小原型，演示在 Windows 上使用 Rust + eframe（egui）构建以手柄为主的游戏启动器原型。

特性（原型）
- 扫描常见安装目录（Program Files / Program Files (x86) / Steam common）中的 `.exe` 文件
- 使用 `gilrs` 轮询手柄事件，支持 DPad 上下、Left Stick Y 和 South 按钮启动
- UI 使用 `eframe`（egui）实现简单的游戏列表与启动按钮

构建与运行

先安装 Rust 工具链，然后在项目目录运行：

```bash
cd launcher
cargo run --release
```

说明
- 这是一个演示性原型，扫描策略非常简单，仅用于快速验证手柄优先的交互与启动流程。
- 后续改进建议：使用更完整的已安装游戏识别（读取 Steam/Origin/Epic/Windows 注册表）、改进导航与焦点管理、增加图标/封面、并对打包体积做细化优化（例如使用静态链接或分发 Delta 更新）。
