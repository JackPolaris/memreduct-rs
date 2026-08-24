# Changelog

本项目用 Tauri v2 + React + Rust 完整重写
[henrypp/memreduct](https://github.com/henrypp/memreduct),
复刻其私有清理 API 与全部功能。所有版本均遵循 GPL-3.0。

## v3.5.3 (2026-08-24)

### 修复
- 修复物理内存占用始终显示 0% 的问题 — 改用 `GlobalMemoryStatusEx` 采集
  物理内存(原字节缓冲读取 `SystemPerformanceInformation` 在缓冲不足时失败)
- 修复 release 版 UAC 通过后应用静默崩溃的问题 — 补充 `plugins.updater`
  配置,修复托盘图标/页文件解析的潜在 panic(panic=abort 会静默退出)
- 修复设置面板卡片内容边距不足的问题
- 修复通知条每条消息显示两条的问题

### 新增
- **首次启动自动提权 + 权限持久化**:首次启动弹一次 UAC 创建计划任务,
  之后每次启动(含手动双击)通过 `schtasks /run` 静默提权,不再弹 UAC
- **开机静默自启 + 永久提权**:计划任务 (`schtasks` 登录时最高权限)
  首次授权一次性 UAC,之后每次开机静默以管理员权限启动到托盘,
  手动/自动清理直接生效,不再弹 UAC
- **自动更新简化**:更新源写死官方仓库 `JackPolaris/memreduct-rs`,
  无需用户配置仓库/密钥
- **主题切换**:浅色 / 深色 / 跟随系统三态(外观设置)
- **主题颜色预设**:绿/紫/蓝/橙/红/青/粉 7 个预设色(外观设置色块)
- 多语言支持 (i18n): 简体中文(主语言)、繁體中文、English、日本語
- 全新应用图标 (紫色清理主题 SVG → PNG/ICO)
- 自动更新检查 (tauri-plugin-updater)
- 系统通知 (tauri-plugin-notification)
- 托盘百分比图标动态渲染 (颜色随警告/危险阈值变化)

### 优化
- 界面排版全面重构: 统一视觉体系、状态色与动效
- 清理确认对话框
- 托盘右键菜单 (显示窗口/清理内存/设置/官方网站/关于/退出)
- 全局热键动态注册/注销
- 代码质量: `cargo fmt` / `cargo clippy -D warnings` 全绿

## 核心功能(自初版)

- 8 区域内存清理: 工作集 / 系统文件缓存 / 修改文件缓存 / 修改页列表 /
  Standby 列表 / Standby 优先级0 / 注册表缓存 / 合并内存列表
- 内存监控: 物理内存 / 页面文件 / 系统缓存
- 自动清理 (阈值 / 间隔,30s 冷却)
- 全局热键 (默认 Ctrl+F1)
- 命令行: `-clean` / `-clean:full`
- portable / appdata 双模式配置存储
- dark/light 主题
