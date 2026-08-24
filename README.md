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

- **8 区域私有内存清理** — 通过未文档化的 `NtSetSystemInformation` (NT API) 清理系统缓存
- **实时内存监控** — 物理内存 / 页面文件 / 系统缓存的用量与百分比
- **托盘图标** — 显示实时内存百分比,支持圆角/边框/透明与颜色自定义
- **自动清理** — 按占用率阈值 (默认 90%) 或时间间隔 (默认 30 分钟) 自动清理
- **全局热键** — 默认 `Ctrl+F1` 一键清理
- **命令行** — `-clean` / `-clean:full`
- **多语言** — 简体中文(主)、繁體中文、English、日本語
- **主题** — dark / light,现代化卡片式界面
- **自动更新** — 基于 tauri-plugin-updater

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

> 完整清理动作需要 **管理员权限**(与原版一致)。

## 🚀 从源码构建

环境要求:Windows 10/11、Node.js 22+、Rust stable (MSVC)、VS2022 C++ 桌面负载。

```bash
git clone https://github.com/JackPolaris/memreduct-rs.git
cd memreduct-rs
npm install
npm run tauri dev      # 开发运行
npm run tauri build    # 打包 MSI + NSIS
```

## 🧪 测试与检查

```bash
npx tsc --noEmit                        # 前端类型检查
cd src-tauri
cargo fmt --check                       # 格式检查
cargo clippy --all-targets -- -D warnings  # 静态检查
cargo test                              # 后端单元测试
```

## 📁 项目结构

```
mem-reduct-tauri/
├─ src/                    # React 前端
│  ├─ App.tsx              # 主界面 + 设置面板
│  ├─ api.ts               # Tauri command 封装
│  ├─ i18n/                # 多语言资源 (zh-CN/zh-TW/en-US/ja-JP)
│  └─ regions.ts           # 清理区域掩码定义
├─ src-tauri/              # Rust 后端
│  ├─ src/
│  │  ├─ ntapi.rs          # 私有 NT API 绑定
│  │  ├─ memory.rs         # 内存采集 + 8 区域清理
│  │  ├─ config.rs         # portable/appdata 配置存储
│  │  ├─ tray.rs / trayicon.rs  # 系统托盘 + 动态图标
│  │  ├─ hotkey.rs         # 全局热键
│  │  ├─ elevation.rs      # 管理员权限检测
│  │  ├─ updater.rs        # 自动更新
│  │  └─ cmdline.rs        # 命令行解析
│  └─ tauri.conf.json
├─ assets/                 # 应用图标源文件 (SVG)
├─ scripts/                # 图标生成脚本
│  ├─ make_icon.py         # 生成 PNG 图标
│  └─ make_ico.py          # 生成 ICO 图标
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
