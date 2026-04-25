# Big Screen Launcher 隐私政策

最后更新日期：2026-04-25

Big Screen Launcher 是一款以本地使用为主的 Windows 游戏启动器。本隐私政策说明该应用在使用过程中会读取、存储和传输哪些数据。

## 1. 概要

- Big Screen Launcher 不要求您注册账户。
- Big Screen Launcher 不出售个人数据。
- Big Screen Launcher 不内置广告。
- Big Screen Launcher 主要读取您设备上已经存在的游戏安装信息和成就相关数据。
- Big Screen Launcher 可能连接 Steam 相关端点，以下载游戏封面资源并获取 Steam 游戏的公开全局成就完成率。

## 2. 应用会访问的本地数据

为了构建本地游戏库并显示相关信息，应用可能会读取您电脑上已经存在的数据，包括：

- Steam 的安装元数据、库元数据以及本地成就相关文件。
- Epic Games Launcher 的清单文件，以及用于判断已安装游戏和最近游玩时间的本地启动器设置。
- Xbox / Microsoft Store 在本机可用的安装元数据。
- 用于启用或关闭开机启动功能的 Windows 注册表项。
- 用于展示游戏库所需的游戏可执行文件路径、安装目录、本地图标资源及相关本地元数据。

这些访问仅用于检测本地已安装游戏、显示成就信息、支持启动相关功能，以及展示封面和图标。

## 3. Big Screen Launcher 在本地存储的数据

Big Screen Launcher 会将自身的本地数据写入当前用户的本地应用数据目录，通常位于 LocalAppData/Big Screen Launcher/。根据您使用的功能，可能包括：

- 保存在 LocalAppData/Big Screen Launcher/config/settings.ini 中的应用设置。
- 保存在 LocalAppData/Big Screen Launcher/config/game_last_played.json 中、由启动器记录的最近游玩时间。
- 保存在 LocalAppData/Big Screen Launcher/caches/achievement_cache/ 中的成就摘要缓存和全局成就完成率缓存。
- 保存在 LocalAppData/Big Screen Launcher/caches/ 中的封面图、Logo、游戏图标、成就图标和 DLSS 检测结果缓存。
- 在触发相关功能后生成的本地日志，例如 LocalAppData/Big Screen Launcher/logs/scan_timings.log 和成就诊断日志。

除非您自行删除，这些数据会保留在您的设备上。

## 4. 网络请求

Big Screen Launcher 主要依赖本地数据运行，但部分功能会发起出站网络请求：

- Steam 资源下载：应用可能会从 Steam 的 CDN 端点下载游戏横幅图、Logo 和成就图标。
- Steam 公开成就统计：应用可能会根据游戏的 App ID 请求 Steam Web API，以获取公开的全局成就完成率。

这些请求仅用于显示游戏资源和公开的社区成就统计。当前版本不需要 Big Screen Launcher 账户，也不会为了这些功能上传您的个人资料数据。

与任何直接网络请求一样，您连接到的服务可能会收到技术信息，例如您的 IP 地址、请求头和所请求的资源路径。

## 5. 应用不会主动收集的内容

基于当前版本的应用行为，Big Screen Launcher 不会主动收集或传输以下数据：

- 您的姓名、电子邮箱地址或电话号码。
- 支付信息。
- 精确位置数据。
- 通讯录、麦克风输入、摄像头输入或您主动上传的文件。
- 您本地游戏库的云端副本。

## 6. 开机启动功能

如果您启用开机启动，Big Screen Launcher 会根据当前安装类型使用对应的登录后自启动机制。传统桌面安装会在当前 Windows 用户配置下写入启动项，而 Microsoft Store / MSIX 打包安装会使用包的 startup task 机制；如果您关闭该设置，应用会移除或禁用对应的启动注册。

## 7. 您的选择

您可以通过以下方式限制或移除相关数据使用：

- 在应用设置中关闭游戏平台检测选项。
- 在应用设置中关闭开机启动。
- 删除 LocalAppData/Big Screen Launcher/ 下的 config、caches 和 logs 目录。
- 如果您不希望应用下载封面资源或获取 Steam 全局成就完成率，可以通过系统或防火墙阻止应用联网。

请注意，删除缓存后，应用下次使用相关功能时可能会重新扫描游戏库或重新下载资源。

## 8. 数据保留期限

本地设置、缓存和日志会一直保留在您的设备上，直到它们被覆盖、刷新，或由您手动删除。

## 9. 儿童隐私

Big Screen Launcher 属于通用软件，并非专门面向儿童设计。应用不会明知故犯地收集儿童个人信息。

## 10. 第三方服务

部分功能依赖第三方平台提供的数据或资源，尤其是 Steam。您对这些服务的使用，也同时受到相关第三方隐私政策和条款的约束。

## 11. 本政策的变更

随着应用功能变化，本隐私政策可能会更新。最新版本应随项目源码或发布材料一并提供。

## 12. 联系方式

如果您对 Big Screen Launcher 的隐私处理有疑问，请通过项目公开支持渠道或发布渠道联系项目维护者。