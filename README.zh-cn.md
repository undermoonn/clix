# Big Screen Launcher

[English](./README.md)
[License](./LICENSE)
[许可说明（简体中文）](./LICENSE.zh-cn.md)
[Privacy Policy](./PRIVACY.md)
[隐私政策](./PRIVACY.zh-cn.md)

一个面向手柄交互优先设计的 Windows 游戏启动器，使用 Rust 与 eframe（egui）构建，旨在为 Windows玩家提供接近家用游戏主机风格的使用体验。

## 功能特性

- 游戏库支持
    - 可检测本地已安装的 Steam 游戏，并支持显示成就列表。
    - 可检测本地已安装的 Epic 游戏
    - 可检测本地已安装的 Xbox 游戏
- 手柄支持
    - Xbox 手柄（xinput）
    - DualSense USB 连接
- 支持在游戏中通过 Xbox Home / PS 键返回
- 支持应用内界面的手柄震动反馈
- 流畅的页面动画效果
- 支持开机启动
- 支持对系统电源的关机、睡眠、重启操作

## 截图

![alt text](screenshots/zh-cn/big-screen-launcher_2026_04_24_18_46_55_124.png)

![alt text](screenshots/zh-cn/big-screen-launcher_2026_04_24_18_46_58_108.png)

![alt text](screenshots/zh-cn/big-screen-launcher_2026_04_24_18_47_02_234.png)

![alt text](screenshots/zh-cn/big-screen-launcher_2026_04_24_18_47_06_689.png)

![alt text](screenshots/zh-cn/big-screen-launcher_2026_04_24_18_47_10_348.png)

## Microsoft Store / MSIX

- 打包安装版本使用 Windows 的 startup task 机制，而不是 HKCU Run 注册表项。
- 如果要让 Store 或 MSIX 包支持开机启动，需要在应用清单中声明 desktop:StartupTask，并将 TaskId 设为 "BigScreenLauncherStartup"。

## 许可证

本项目采用 GNU General Public License v3.0（GPLv3）许可证。正式许可证文本请参见 [LICENSE](./LICENSE)，中文说明请参见 [LICENSE.zh-cn.md](./LICENSE.zh-cn.md)。
