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
# 前端类型检查 / 构建 / 图标校验
npx tsc --noEmit
npm run build
node scripts/verify_icon_png.mjs

# 后端格式 / 静态检查 / 测试
cd src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test

# 打包安装程序 (MSI + NSIS)
npm run tauri build

# 只打 NSIS（省时间；发布流程目前只上传 NSIS）
npx tauri build --bundles nsis
```

## 发布新版本

自动更新依赖两样东西：**GitHub Release 上的安装包** 与 **更新清单 JSON**。
客户端的更新地址固定为：

```
https://github.com/JackPolaris/memreduct-rs/releases/latest/download/update-x86_64-pc-windows-msvc.json
```

（`aarch64` / `i686` 会各自请求 `update-aarch64-pc-windows-msvc.json` 等，
由 `src-tauri/src/updater.rs` 按编译架构拼出。当前只发布 x86_64。）

### 1. 同步版本号

**五个文件**都要改，漏掉任何一个都会出问题：

| 文件 | 位置 |
|---|---|
| `package.json` | `version` |
| `package-lock.json` | 根 `version` 与 `packages.""` 里的 `version`（共 2 处） |
| `src-tauri/Cargo.toml` | `[package] version` |
| `src-tauri/Cargo.lock` | `[[package]] name = "mem-reduct"` 的 `version` |
| `src-tauri/tauri.conf.json` | `version`（决定安装包文件名与更新版本号） |

`Cargo.lock` 最容易被忘，但它不影响构建，只影响 `cargo` 元数据的一致性。

同时在 `CHANGELOG.md` 记录本次变更。

### 2. 带签名密钥构建

```bash
export TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/memreduct.key)"
npx tauri build --bundles nsis        # 需要 MSI 时去掉 --bundles
```

- 私钥必须与 `tauri.conf.json` 的 `plugins.updater.pubkey` 配对，
  **不能重新生成** —— 换密钥会让所有已安装版本再也无法验证新签名，
  自动更新直接失效。校验方法：`memreduct.key.pub` 的内容应等于
  `tauri.conf.json` 里 `pubkey` 字段的值。
- 缺少 `TAURI_SIGNING_PRIVATE_KEY` 时，开启了 `createUpdaterArtifacts` 的
  `tauri build` 会直接失败。

产物（`src-tauri/target/release/bundle/` 下）：

- `nsis/Mem Reduct_X.Y.Z_x64-setup.exe` — 安装包
- `nsis/Mem Reduct_X.Y.Z_x64-setup.exe.sig` — 安装包的 minisign 签名
- 同目录下的 `latest.json` — 更新清单（用
  `find src-tauri/target/release/bundle -name latest.json` 定位）

### 3. 处理更新清单（最容易出错的一步）

先把安装包改成 URL 友好的名字（**空格换成点**，与历史 release 保持一致）：

```bash
cd src-tauri/target/release/bundle/nsis
mv "Mem Reduct_X.Y.Z_x64-setup.exe" "Mem.Reduct_X.Y.Z_x64-setup.exe"
```

然后编辑 `latest.json`，改两处：

1. **补 `notes`** —— Tauri 不生成这个字段，而它会作为更新说明展示给客户端。
   通常写「版本号 + 几条要点」；
2. **核对 `platforms.windows-x86_64.url`** —— 必须指向 release 资产的实际文件名：

   `https://github.com/JackPolaris/memreduct-rs/releases/download/vX.Y.Z/Mem.Reduct_X.Y.Z_x64-setup.exe`

   以及确认 `signature` 字段就是 `.sig` 文件的内容。

最后把它改名为 `update-x86_64-pc-windows-msvc.json`，并**同步覆盖仓库根目录的同名文件**
（那份是最近一次发布清单的副本，便于对照签名与 URL 格式，也会随代码一起进版本管理）。

### 4. 打 tag 并上传

```bash
git add -A && git commit -m "release: vX.Y.Z"
git tag vX.Y.Z
git push origin master --tags

gh release create vX.Y.Z \
  --title "Mem Reduct X.Y.Z" \
  --notes "见 CHANGELOG.md" \
  "src-tauri/target/release/bundle/nsis/Mem.Reduct_X.Y.Z_x64-setup.exe" \
  "src-tauri/target/release/bundle/nsis/Mem.Reduct_X.Y.Z_x64-setup.exe.sig" \
  "src-tauri/target/release/bundle/nsis/update-x86_64-pc-windows-msvc.json"
```

三个资产缺一不可：安装包、`.sig` 签名、更新清单。
上传完成后 `releases/latest` 会自动指向新版本，客户端下次启动即可收到更新。

## 目录结构

```
src/            React 前端 (界面 / i18n / 组件)
src-tauri/src/  Rust 后端 (ntapi 绑定 / 内存清理 / 配置 / 托盘 / 热键)
scripts/        图标生成/校验脚本 (见 scripts/README.md：当前只用蓝色位图那条链路)
.github/        CI 工作流
```

## 提交规范

- 一次提交只做一件事,提交信息用简洁的英文或中文描述改动
- 保持 `cargo fmt`、`cargo clippy -- -D warnings`、`cargo test`、`npx tsc --noEmit` 全部通过
- 涉及功能变更时请补充或更新测试

## 许可

本项目基于 [henrypp/memreduct](https://github.com/henrypp/memreduct)
(GPL-3.0) 派生重写,采用相同的 **GPL-3.0** 许可。分发衍生产品时请遵守 GPL-3.0 条款。
