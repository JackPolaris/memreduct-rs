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
[henrypp/memreduct](https://github.com/henrypp/memreduct) 的私有清理 API，
以及托盘图标、全局热键、自动清理等核心机制。

> 本项目是**独立重写**，并非原版的逐项复刻。原版有、本版尚未提供的能力见
> [已知未实现](#-已知未实现)，避免预期落差。

---

## ✨ 特性

- **8 区域私有内存清理** — 通过未文档化的 `NtSetSystemInformation`（NT API）清理系统缓存；
  当前系统不支持的区域（注册表缓存需 Win8.1+、合并列表需 Win10+）会在界面上禁用并说明，
  而不是静默跳过
- **实时内存监控** — 物理内存 / 页面文件 / 系统缓存的用量与百分比
- **托盘图标** — 图标内绘制实时内存百分比，支持圆角 / 边框 / 透明与颜色自定义，
  右键菜单跟随应用语言
- **自动清理** — 按占用率阈值（默认 90%）或时间间隔（默认 30 分钟）自动清理
- **全局热键** — 默认 `Ctrl+F1` 一键清理；被其他程序占用时会在界面上提示注册失败，
  不会假装已生效
- **清理结果可核实** — 每个清理区域的调用结果都会上报，部分失败时会明确提示失败数量，
  而不是只显示「释放了 0 B」
- **单实例保护** — 重复启动不会产生第二个托盘图标，而是唤起已打开的窗口
- **命令行** — `-clean` / `-clean:full`，清理后直接退出、不弹界面，未提权时自动请求 UAC；
  全部区域失败时返回退出码 `1`，便于脚本判断
- **多语言** — 简体中文（默认）、繁體中文、English、日本語（界面与托盘菜单同时生效）
- **主题** — 浅色 / 深色 / 跟随系统，7 种主题颜色预设
- **开机静默自启** — 计划任务以最高权限在登录时静默启动到托盘（可另设「启动时最小化」）
- **自动更新** — 启动时检查更新，发现新版可一键下载并静默安装后重启
  （清单地址按编译架构自动选择，支持 x86_64 / aarch64 / i686）
- **便携模式** — 在程序目录放置 `memreduct.json` 即切换到便携模式，配置写在程序旁边；
  否则使用 `%APPDATA%\Henry++\Mem Reduct`

## 🔧 私有清理 API

通过 `NtSetSystemInformation` 调用未文档化的 NT 接口（与原版一致，**需要管理员权限**）：

| 清理区域 | 掩码 | 底层调用 |
|---|---|---|
| 工作集 Working Set | `0x01` | `MemoryEmptyWorkingSets` |
| 系统文件缓存 System File Cache | `0x02` | `SystemFileCacheInformationEx`（Min/MaxWS=`MAXSIZE_T`） |
| Standby 优先级 0 列表 | `0x04` | `MemoryPurgeLowPriorityStandbyList` |
| Standby 列表 Standby List | `0x08` | `MemoryPurgeStandbyList` |
| 修改页列表 Modified List | `0x10` | `MemoryFlushModifiedList` |
| 合并内存列表 Combine Lists | `0x20` | `SystemCombinePhysicalMemoryInformation`（Win10+） |
| 注册表缓存 Registry Cache | `0x40` | `SystemRegistryReconciliationInformation`（Win8.1+） |
| 修改文件缓存 Modified File Cache | `0x80` | 枚举本地卷并 `FlushFileBuffers` |

**自动清理**默认排除 `Standby List` 与 `Modified List`（这两个列表被清空时可能造成短暂卡顿）。

**手动清理**可选全部区域，但请留意其代价：

- 清空**工作集**会让所有进程的常驻页被换出，之后它们需要从磁盘重新加载，
  短期内切回这些程序可能变慢；
- 清理的是系统缓存与待机/修改列表，**不会降低任务管理器里「已用内存」的长期水平** ——
  这些内存本来就是可回收的，系统有需要时会自己回收。

## 📦 安装

从 [Releases](https://github.com/JackPolaris/memreduct-rs/releases) 下载 **NSIS 安装程序**，
文件名形如 `Mem.Reduct_3.5.13_x64-setup.exe`。

> Release 目前只提供 NSIS 安装包。MSI（便于域内静默部署）需要从源码构建：
> `npm run tauri build` 会同时产出 `nsis/*-setup.exe` 与 `msi/*.msi`。

> ⚠️ **首次运行会看到 SmartScreen 警告**
>
> 安装包没有购买 Authenticode 代码签名证书（只有用于自动更新的 minisign 签名，
> Windows 不认这个），因此首次运行会弹出「Windows 已保护你的电脑」。
> 点击 **更多信息 → 仍要运行** 即可。如果不放心，可以按下面的「从源码构建」自行编译。

### 管理员权限（复刻原版的 UAC 流程）

- **启动**：普通权限，不弹 UAC；
- **手动清理**：若未提权，弹**一次** UAC，整个应用重启为管理员进程
  （原版 `_r_app_runasadmin` 的 runas 重启方式），之后所有清理 / 自动清理
  都在管理员进程中运行，**不再弹 UAC**；
- **开机静默自启**（可选，设置 → 常规）：计划任务在登录时以最高权限静默启动，
  直接提权，永不弹 UAC。

### 其他行为说明

- 关闭窗口 = 最小化到托盘（程序继续运行），首次关闭会提示一次；
  退出请用托盘菜单的「退出」。
- 重复启动不会产生第二个托盘图标，而是直接唤起已经打开的窗口。
- 主界面勾选的清理区域会立即保存，托盘菜单、全局热键、自动清理共用同一份设置。

## ❓ 常见问题

**清理完为什么任务管理器里的内存数字没怎么变？**
清理回收的是系统缓存与待机 / 修改列表，这些本来就是「可用内存」的一部分。
数字变化大小取决于当时的缓存量，属于正常现象。

**为什么需要管理员权限？**
上表中的 NT 调用是系统级接口。未提权时工作集、待机列表等区域会返回
`STATUS_PRIVILEGE_NOT_HELD`，此时界面会提示失败的区域数量。

**会不会伤硬件或影响系统稳定性？**
不会。这些都是 Windows 自身也会执行的内存管理操作，只是由工具提前触发。

**为什么托盘图标旁边没有百分比文字？**
Windows 的托盘不提供图标标题（该能力只有 macOS / Linux 有），
本版把百分比直接绘制在图标位图内，并把详情放进鼠标悬停提示。

## 🖥️ 系统要求

- **运行**：Windows 10 / 11（x64；`aarch64`、`i686` 亦可自行构建）
- 部分区域在更低版本系统上不可用（注册表缓存需 Win8.1+，合并列表需 Win10+），
  界面上会自动禁用

## 🚀 从源码构建

环境要求：Windows 10/11、Node.js 22+、Rust stable（MSVC 工具链）、
Visual Studio 2022（含 C++ 桌面工作负载）。

```bash
git clone https://github.com/JackPolaris/memreduct-rs.git
cd memreduct-rs
npm install
npm run tauri dev      # 开发运行
npm run tauri build    # 打包 NSIS + MSI
```

> 打包正式版需要 updater 签名私钥（`bundle.createUpdaterArtifacts` 已开启）：
> 设置 `TAURI_SIGNING_PRIVATE_KEY`（minisign 私钥内容或文件路径）即可，
> 否则 `tauri build` 会因缺少签名密钥而失败。只想验证打包能否通过时，
> 可用 `npx tauri build --config src-tauri/tauri.ci.conf.json` 关闭 updater 产物
> （CI 就是这么做的）。
>
> 只打一种安装包、节省时间：`npx tauri build --bundles nsis`。

## 🧪 测试与检查

```bash
npm run build                              # tsc 类型检查 + vite 构建
node scripts/verify_icon_png.mjs           # 图标非空白校验（CI 会跑）

cd src-tauri
cargo fmt --check                          # 格式检查
cargo clippy --all-targets -- -D warnings  # 静态检查
cargo test                                 # 后端单元测试
```

## 📁 项目结构

```
mem-reduct-tauri/
├─ src/                    # React 前端
│  ├─ App.tsx              # 主界面 + 设置面板
│  ├─ api.ts               # Tauri command 封装（类型与后端 Config 一一对应）
│  ├─ i18n/                # 多语言资源（zh-CN / zh-TW / en-US / ja-JP）
│  ├─ accents.ts           # 主题颜色预设
│  └─ regions.ts           # 清理区域掩码与系统支持判定
├─ src-tauri/              # Rust 后端
│  ├─ src/
│  │  ├─ main.rs           # 入口：命令行清理 / 单次提权助手 / 启动 UI
│  │  ├─ lib.rs            # Tauri 装配、统一清理入口 perform_clean、后台循环
│  │  ├─ ntapi.rs          # 私有 NT API 绑定
│  │  ├─ memory.rs         # 内存采集 + 8 区域清理
│  │  ├─ config.rs         # portable/appdata 配置存储（原子写入 + 取值校验）
│  │  ├─ single_instance.rs    # 单实例互斥与提权交接
│  │  ├─ tray.rs / trayicon.rs # 系统托盘 + 动态位图图标
│  │  ├─ hotkey.rs         # 全局热键
│  │  ├─ autostart.rs      # 计划任务静默提权自启
│  │  ├─ elevation.rs      # 管理员权限检测与 runas 重启
│  │  ├─ updater.rs        # 自动更新
│  │  └─ cmdline.rs        # 命令行解析
│  └─ tauri.conf.json
├─ assets/                 # 图标源文件（品牌色为蓝色 #3366FF，源为 icon-reference.png）
├─ scripts/                # 图标生成/校验脚本（见 scripts/README.md）
├─ docs/                   # 审查与设计文档
└─ .github/workflows/ci.yml   # CI
```

## 🔀 已知未实现

以下能力原版有、本版**暂时没有**，属于后续规划而非承诺：

- 运行统计与历史（累计释放量、清理次数）
- 清理日志输出到文件
- 内存区域占用明细列表（本版只汇总物理内存 / 页面文件 / 系统缓存三项）
- 命令行开关数量少于原版（本版只有 `-clean` / `-clean:full`）

## 🤝 贡献

欢迎提交 Issue 与 Pull Request，详见 [CONTRIBUTING.md](CONTRIBUTING.md)
（其中包含发布流程与版本号同步清单）。

## 📄 许可

本项目基于 [henrypp/memreduct](https://github.com/henrypp/memreduct)（GPL-3.0）
派生重写，采用相同的 [GPL-3.0](LICENSE) 许可。分发衍生产品时请遵守 GPL-3.0 条款，
以二进制形式分发时必须同时提供完整对应源码。

## 🙏 致谢

- [henrypp/memreduct](https://github.com/henrypp/memreduct) — 原版项目与私有 API 原理
- [Tauri](https://tauri.app) / [React](https://react.dev) / [Rust](https://www.rust-lang.org)
