# 贡献指南 (Contributing)

感谢你对 Mem Reduct (Tauri + React 重构版) 的关注!这是一个用
**Tauri v2 + Rust + React** 重写的内存清理工具,复刻了
[henrypp/memreduct](https://github.com/henrypp/memreduct) 的私有清理 API 与功能。

## 开发环境

- Windows 10/11(64 位)
- [Node.js](https://nodejs.org/) 22+
- [Rust](https://rustup.rs/) stable(MSVC 工具链)
- Visual Studio 2022(含 C++ 桌面工作负载,用于 MSVC 链接器)

## 快速开始

```bash
git clone https://github.com/JackPolaris/memreduct-rs.git
cd memreduct-rs
npm install
npm run tauri dev
```

## 构建与测试

```bash
# 前端类型检查
npx tsc --noEmit
npm run build

# 后端格式 / 静态检查 / 测试
cd src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test

# 打包安装程序 (MSI + NSIS)
npm run tauri build
```

## 目录结构

```
src/            React 前端 (界面 / i18n / 组件)
src-tauri/src/  Rust 后端 (ntapi 绑定 / 内存清理 / 配置 / 托盘 / 热键)
scripts/        图标生成脚本
.github/        CI 工作流
```

## 提交规范

- 一次提交只做一件事,提交信息用简洁的英文或中文描述改动
- 保持 `cargo fmt`、`cargo clippy -- -D warnings`、`cargo test`、`npx tsc --noEmit` 全部通过
- 涉及功能变更时请补充或更新测试

## 许可

本项目基于 [henrypp/memreduct](https://github.com/henrypp/memreduct)
(GPL-3.0) 派生重写,采用相同的 **GPL-3.0** 许可。分发衍生产品时请遵守 GPL-3.0 条款。
