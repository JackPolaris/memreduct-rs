<p align="center">
  <img src="assets/icon.svg" alt="Mem Reduct" width="120" height="120" />
</p>

<h1 align="center">Mem Reduct</h1>
<p align="center">
  轻量级实时内存管理工具 — Tauri v2 + React + Rust 完整重构版
</p>

<p align="center">
  <a href="https://github.com/JackPolaris/memreduct-rs/blob/master/LICENSE"><img alt="License" src="https://img.shields.io/github/license/JackPolaris/memreduct-rs"></a>
  <a href="https://github.com/JackPolaris/memreduct-rs/actions"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/JackPolaris/memreduct-rs/ci.yml?branch=master"></a>
  <a href="https://github.com/JackPolaris/memreduct-rs/releases"><img alt="Release" src="https://img.shields.io/github/v/release/JackPolaris/memreduct-rs"></a>
</p>

用 **Tauri v2 + Rust 后端 + React 前端** 完全重构的开源内存清理工具,
完整复刻原版 [henrypp/memreduct](https://github.com/henrypp/memreduct) 的私有清理 API
和全部功能。

---

## ✨ 特性

- **8 区域私有内存清理** — 通过未文档化的 `NtSetSystemInformation` (NT API) 清理系统缓存；
  当前系统不支持的区域(注册表缓存需 Win8.1+、合并列表需 Win10+)会在界面上禁用而不是静默跳过
- **实时内存监控** — 物理内存 / 页面文件 / 系统缓存的用量与百分比
- **托盘图标** — 图标内绘制实时内存百分比,支持圆角/边框/透明与颜色自定义,右键菜单跟随应用语言
- **自动清理** — 按占用率阈值 (默认 90%) 或时间间隔 (默认 30 分钟) 自动清理
- **全局热键** — 默认 `Ctrl+F1` 一键清理;被其他程序占用时会在界面上提示注册失败
- **单实例保护** — 重复启动只保留一个托盘图标/一个后台循环,第二次启动会唤起已有窗口
- **命令行** — `-clean` / `-clean:full`(清理后直接退出,不弹界面,未提权时自动请求 UAC);
  完全失败时返回退出码 1,便于脚本判断
- **多语言** — 简体中文(主)、繁體中文、English、日本語(界面与托盘菜单同时生效)
- **主题** — 浅色 / 深色 / 跟随系统 + 7 种主题颜色预设,现代化卡片式界面
- **开机静默自启** — 计划任务登录时最高权限静默启动到托盘(可另设「启动时最小化」)
- **自动更新** — 启动自动检查更新,一个「检查更新」按钮,发现新版自动下载并静默安装后重启
  (更新清单地址按编译架构自动选择,支持 x86_64 / aarch64 / i686)

## 🔧 私有清理 API

通过 `NtSetSystemInformation` 调用未文档化的 NT 接口(与原版一致,需管理员权限):

| 清理区域 | 掩码 | 底层调用 |
|---|---|---|
| 工作集 Working Set | `0x01` | `MemoryEmptyWorkingSets` |
| 系统文件缓存 System File Cache | `0x02` | `SystemFileCacheInformationEx` (Min/MaxWS=`MAXSIZE_T`) |
| 修改文件缓存 Modified File Cache | `0x80` | 枚举卷并 `FlushFileBuffers` |
| 修改页列表 Modified List | `0x10` | `MemoryFlushModifiedList` |
| Standby 列表 Standby List | `0x08` | `MemoryPurgeStandbyList` |
| Standby 优先级0列表 | `0x04` | `MemoryPurgeLowPriorityStandbyList` |
| 注册表缓存 Registry Cache | `0x40` | `SystemRegistryReconciliationInformation` (win8.1+) |
| 合并内存列表 Combine Lists | `0x20` | `SystemCombinePhysicalMemoryInformation` (win10+) |

自动清理默认排除 `Standby List` 与 `Modified List`(可能造成短暂卡顿)。

## 📦 安装

从 [Releases](https://github.com/JackPolaris/memreduct-rs/releases) 下载:

- **NSIS 安装程序** `*.exe` — 图形化安装
- **MSI 安装包** `*.msi` — 支持静默部署

> **管理员权限(复刻原项目 Mem Reduct 的 UAC 流程)**:
> - **启动**:普通权限,不弹 UAC;
> - **手动清理**:若未提权,弹**一次** UAC,整个应用重启为管理员进程
>   (原项目 `_r_app_runasadmin` 的 runas 重启方式),之后所有清理/自动清理
>   都在管理员进程中运行,**不再弹 UAC**;
> - **开机静默自启**(可选,设置→常规):计划任务登录时以最高权限静默
>   启动,直接提权,永不弹 UAC。

> **其他行为说明**:
> - 关闭窗口 = 最小化到托盘(程序继续运行),首次关闭会弹一次提示;
>   退出请用托盘菜单的「退出」。
> - 重复启动不会产生第二个托盘图标,而是直接唤起已经打开的窗口。
> - 主界面勾选的清理区域会立即保存,托盘菜单、全局热键、自动清理都使用同一份设置。

## 🚀 从源码构建

环境要求:Windows 10/11、Node.js 22+、Rust stable (MSVC)、VS2022 C++ 桌面负载。

```bash
git clone https://github.com/JackPolaris/memreduct-rs.git
cd memreduct-rs
npm install
npm run tauri dev      # 开发运行
npm run tauri build    # 打包 MSI + NSIS
```

> 打包正式版需要 updater 签名私钥(`bundle.createUpdaterArtifacts` 已开启):
> 设置 `TAURI_SIGNING_PRIVATE_KEY`(minisign 私钥内容或文件路径)即可,
> 否则 `tauri build` 会因缺少签名密钥而失败。只想验证打包能否通过时,
> 可用 `npx tauri build --config src-tauri/tauri.ci.conf.json` 关闭 updater 产物
> (CI 就是这么做的)。

## 🧪 测试与检查

```bash
npm run build                           # tsc 类型检查 + vite 构建
cd src-tauri
cargo fmt --check                       # 格式检查
cargo clippy --all-targets -- -D warnings  # 静态检查
cargo test                              # 后端单元测试
```

## 🚢 发布新版本

1. 同步版本号:`package.json`、`package-lock.json`、`src-tauri/Cargo.toml`、
   `src-tauri/tauri.conf.json`,并在 `CHANGELOG.md` 记录变更;
2. 带签名密钥本地构建(会生成安装包 + updater 清单):
   ```bash
   TAURI_SIGNING_PRIVATE_KEY=... npm run tauri build
   ```
   产物:`src-tauri/target/release/bundle/nsis/*-setup.exe`、`msi/*.msi`
   以及 `src-tauri/target/release/bundle/*/latest.json`;
3. 打 tag(`vX.Y.Z`)并创建 GitHub Release,上传安装包;
4. 把 Release 中的更新清单重命名为 `update-x86_64-pc-windows-msvc.json` 一并上传
   — 应用的更新 endpoint 固定指向
   `releases/latest/download/update-x86_64-pc-windows-msvc.json`;
5. 仓库根目录的同名文件是最近一次发布清单的副本,便于对照签名/URL 格式,
   发布后可用新生成的 `latest.json` 覆盖它。

## 📁 项目结构

```
mem-reduct-tauri/
├─ src/                    # React 前端
│  ├─ App.tsx              # 主界面 + 设置面板
│  ├─ api.ts               # Tauri command 封装
│  ├─ i18n/                # 多语言资源 (zh-CN/zh-TW/en-US/ja-JP)
│  ├─ accents.ts           # 主题颜色预设
│  └─ regions.ts           # 清理区域掩码定义
├─ src-tauri/              # Rust 后端
│  ├─ src/
│  │  ├─ main.rs           # 入口:命令行清理 / 单次提权助手 / 启动 UI
│  │  ├─ lib.rs            # Tauri 应用装配、后台循环、命令
│  │  ├─ ntapi.rs          # 私有 NT API 绑定
│  │  ├─ memory.rs         # 内存采集 + 8 区域清理
│  │  ├─ config.rs         # portable/appdata 配置存储(原子写入)
│  │  ├─ tray.rs / trayicon.rs  # 系统托盘 + 动态图标
│  │  ├─ hotkey.rs         # 全局热键
│  │  ├─ autostart.rs      # 计划任务静默提权自启
│  │  ├─ elevation.rs      # 管理员权限检测与 runas 重启
│  │  ├─ updater.rs        # 自动更新
│  │  └─ cmdline.rs        # 命令行解析
│  └─ tauri.conf.json
├─ assets/                 # 图标源文件(当前品牌色为蓝色 #3366FF,源为 icon-reference.png)
├─ scripts/                # 图标生成/校验脚本(见 scripts/README.md,含已废弃链路说明)
└─ .github/workflows/ci.yml  # CI
```

## 🤝 贡献

欢迎提交 Issue 与 Pull Request,详见 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 📄 许可

本项目基于 [henrypp/memreduct](https://github.com/henrypp/memreduct) (GPL-3.0)
派生重写,采用相同的 [GPL-3.0](LICENSE) 许可。分发衍生产品时请遵守 GPL-3.0 条款。

## 🙏 致谢

- [henrypp/memreduct](https://github.com/henrypp/memreduct) — 原版项目与私有 API 原理
- [Tauri](https://tauri.app) / [React](https://react.dev) / [Rust](https://www.rust-lang.org)
