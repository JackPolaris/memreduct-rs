<p align="center">
  <img src="assets/icon-reference.png" alt="Mem Reduct" width="120" height="120" />
</p>

<h1 align="center">Mem Reduct</h1>
<p align="center">
  轻量级实时内存管理工具 — Tauri v2 + React + Rust 重构版
</p>

<p align="center">
  <a href="https://github.com/JackPolaris/memreduct-rs/blob/master/LICENSE"><img alt="License" src="https://img.shields.io/github/license/JackPolaris/memreduct-rs"></a>
  <a href="https://github.com/JackPolaris/memreduct-rs/actions"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/JackPolaris/memreduct-rs/ci.yml?branch=master"></a>
  <a href="https://github.com/JackPolaris/memreduct-rs/releases"><img alt="Release" src="https://img.shields.io/github/v/release/JackPolaris/memreduct-rs"></a>
</p>

用 **Tauri v2 + Rust 后端 + React 前端** 重写的开源内存清理工具，复刻原版
[henrypp/memreduct](https://github.com/henrypp/memreduct) 的核心机制：
通过未公开的 NT 接口清理系统缓存，配合托盘百分比图标、全局热键与自动清理。

> 本项目是**独立重写**，并非原版的逐项复刻。原版有、本版尚未提供的能力见
> [已知未实现](#-已知未实现)，避免预期落差。

---

## ✨ 特性

- **8 个可选的清理区域** — 工作集 / 系统文件缓存 / 待机列表 / 修改页列表 /
  注册表缓存 等，可自由勾选组合；当前系统不支持的区域会自动禁用并说明
- **实时内存监控** — 物理内存、页面文件、系统缓存的用量与百分比
- **托盘图标** — 百分比直接绘制在图标上，颜色随占用率变化，圆角 / 边框 / 透明可调
- **自动清理** — 按占用率阈值（默认 90%）或时间间隔（默认 30 分钟）自动触发
- **全局热键** — 默认 `Ctrl+F1` 一键清理，组合键冲突时会明确提示
- **结果可核实** — 每个区域的实际结果都会反馈，部分失败时明确显示失败数量，
  而不是只丢一个「释放了 0 B」
- **命令行** — `-clean` / `-clean:full`，清理后直接退出、不弹界面；
  全部失败时返回退出码 `1`，便于脚本判断
- **单实例** — 重复启动不会产生第二个托盘图标，而是唤起已打开的窗口
- **多语言 / 主题** — 16 种界面语言（简中、繁中、英、日、韩、德、法、西、葡（巴西）、
  意、俄、波、土、越、泰、印尼）；浅色、深色、跟随系统
- **安装程序也是多语言的** — 安装界面同样 16 种语言，默认跟随系统语言，也可以
  在安装时直接选好程序语言，装完首次打开就是该语言
- **开机静默自启** — 计划任务以最高权限在登录时静默启动到托盘
- **自动更新** — 启动时检查更新，发现新版可一键下载并静默安装后重启
- **便携模式** — 在程序目录放置 `memreduct.json` 即启用，配置写在程序旁边；
  否则使用 `%APPDATA%\Henry++\Mem Reduct`

## 🧹 清理的是什么

上面那些"区域"是 Windows 内存管理里的可回收部分，大致分三类：

| 类别 | 包含区域 | 说明 |
|---|---|---|
| 进程占用 | 工作集 | 各进程当前常驻的内存页 |
| 系统缓存 | 系统文件缓存、修改文件缓存、注册表缓存 | 文件与注册表的读写缓存 |
| 待回收列表 | 待机列表、修改页列表、合并内存列表 | 已被释放但系统暂时留作缓存的内存 |

被清理的内存**本来就是可回收的**，系统有需要时会自行回收 —— 清理的意义在于提前腾出，
而不是"把已用内存降下来"。因此：

- 清理后任务管理器里的内存数字**未必有明显变化**，属正常现象；
- 清空**工作集**会让各进程的常驻页被换出、之后需从磁盘重新加载，
  短期内切回这些程序可能变慢；
- 清空**待机 / 修改列表**可能造成短暂卡顿，所以**自动清理默认跳过这两个区域**，
  需要的话可在设置里放开。

> 详细的区域掩码与底层调用对应关系见
> [CONTRIBUTING.md](CONTRIBUTING.md#清理实现与区域掩码)。

## 📦 安装

从 [Releases](https://github.com/JackPolaris/memreduct-rs/releases) 下载 **NSIS 安装程序**，
文件名形如 `Mem.Reduct_3.5.13_x64-setup.exe`。目前只提供 NSIS 安装包，
需要 MSI（域内静默部署）请从源码构建。

> ⚠️ **首次运行会看到 SmartScreen 警告**
>
> 安装包没有购买代码签名证书，首次运行会弹出「Windows 已保护你的电脑」。
> 点击 **更多信息 → 仍要运行** 即可；如不放心，可按下面的
> [从源码构建](#-从源码构建)自行编译。

### 关于管理员权限

清理区域依赖系统级接口，因此需要管理员权限。本版复刻了原版的 UAC 流程：

- **启动**：普通权限，不弹 UAC；
- **手动清理**：若未提权，弹**一次** UAC，应用重启为管理员进程，之后所有清理
  （含自动清理）都在管理员进程中运行，**不再弹 UAC**；
- **开机静默自启**（可选，设置 → 常规）：计划任务以最高权限静默启动，永不弹 UAC。

### 其他行为

- 关闭窗口 = 最小化到托盘（程序继续运行），首次关闭会提示一次；
  退出请用托盘菜单的「退出」。
- 主界面勾选的清理区域会立即保存，托盘菜单、全局热键、自动清理共用同一份设置。

## 🖥️ 系统要求

Windows 10 / 11（x64）。注册表缓存需 Win8.1+、合并内存列表需 Win10+，
在更低版本系统上这些区域会被自动禁用。

## 🚀 从源码构建

需要 Windows 10/11、Node.js 22+、Rust stable（MSVC）、Visual Studio 2022（C++ 桌面负载）。

```bash
git clone https://github.com/JackPolaris/memreduct-rs.git
cd memreduct-rs
npm install
npm run tauri dev      # 开发运行
npm run tauri build    # 打包 NSIS + MSI
```

> 打包正式版需要 updater 签名私钥（`bundle.createUpdaterArtifacts` 已开启），
> 否则 `tauri build` 会在签名步骤失败。只想验证打包能否通过时，可用
> `npx tauri build --config src-tauri/tauri.ci.conf.json` 关闭 updater 产物。

## 📁 项目结构

```
src/          React 前端（界面 / i18n / 主题）
src-tauri/    Rust 后端（NT 接口绑定、内存清理、托盘、热键、自动更新）
assets/       图标源文件
scripts/      图标生成与校验脚本
docs/         审查与设计文档
```

各模块职责与开发约定见 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 🔀 已知未实现

以下能力原版有、本版**暂时没有**，属于后续规划而非承诺：

- 运行统计与历史（累计释放量、清理次数）
- 清理日志输出到文件
- 内存区域占用明细列表（本版只汇总物理内存 / 页面文件 / 系统缓存三项）
- 命令行开关少于原版（本版只有 `-clean` / `-clean:full`）

## 🤝 贡献

欢迎提交 Issue 与 Pull Request，详见 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 📄 许可

本项目基于 [henrypp/memreduct](https://github.com/henrypp/memreduct)（GPL-3.0）
派生重写，采用相同的 [GPL-3.0](LICENSE) 许可。分发衍生产品时请遵守 GPL-3.0 条款，
以二进制形式分发时必须同时提供完整对应源码。

## 🙏 致谢

- [henrypp/memreduct](https://github.com/henrypp/memreduct) — 原版项目与私有 API 原理
- [Tauri](https://tauri.app) / [React](https://react.dev) / [Rust](https://www.rust-lang.org)
