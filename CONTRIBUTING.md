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

注意 `tauri build` **只产出这两样，不会生成 `latest.json`** —— 更新清单需要手写，
见下一步。

### 3. 生成更新清单（最容易出错的一步）

先把安装包改成 URL 友好的名字（**空格换成点**，与历史 release 保持一致）：

```bash
cd src-tauri/target/release/bundle/nsis
mv "Mem Reduct_X.Y.Z_x64-setup.exe" "Mem.Reduct_X.Y.Z_x64-setup.exe"
```

然后手写清单（可参照仓库根的 `update-x86_64-pc-windows-msvc.json`），
`signature` 直接取 `.sig` 文件的全文：

```json
{
  "version": "X.Y.Z",
  "notes": "Mem Reduct X.Y.Z\n\n- 变更要点一\n- 变更要点二",
  "pub_date": "2026-09-15T03:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "<.sig 文件的全文，含 untrusted comment 行>",
      "url": "https://github.com/JackPolaris/memreduct-rs/releases/download/vX.Y.Z/Mem.Reduct_X.Y.Z_x64-setup.exe"
    }
  }
}
```

三个字段最容易错：

- `url` 必须与 release 资产的**实际文件名逐字一致**（含把空格换成点），否则客户端下载 404；
- `notes` 会作为更新说明展示给客户端，不能省；
- `signature` 就是 `.sig` 文件的全文（单行 base64，整段拷进去）。
  它解码后是 minisign 的两行格式（`untrusted comment` + 签名块），别只拷其中一行。

存为 `update-x86_64-pc-windows-msvc.json`，并**同步覆盖仓库根目录的同名文件**
（那份是最近一次发布清单的副本，便于对照格式，也会随代码一起进版本管理）。

生成后建议自查一遍签名用的是不是配对的密钥 —— 签名块的 key id
（解码后第 2 行 base64 的第 3–10 字节，小端）应当等于 `tauri.conf.json`
里 `plugins.updater.pubkey` 解码后的 key id。对不上就说明用错了私钥，
老客户端会拒绝这个更新。

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

## 清理实现与区域掩码

清理通过未文档化的 `NtSetSystemInformation` 完成（与原版一致，需管理员权限）。
`src-tauri/src/memory.rs` 中的 `mask` 模块是唯一事实来源：

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

自动清理（`is_autoclean`）在未放开「允许清理待机列表」时会去掉
`STANDBYLIST | MODIFIEDLIST` 两位，避免自动触发时卡顿。

**每个区域的调用结果都会收集到 `CleanResult.failed`**，界面据此提示失败数量；
新增区域时不要再用 `let _ =` 丢弃返回值。

改动清理区域时需要同步四处，否则界面与后端会不一致：

1. `src-tauri/src/memory.rs` 的 `mask` 常量与 `mask::names()`（顺序即界面顺序）
2. `src/regions.ts` 的 `REGIONS`（bit / key / min 版本）
3. `src/i18n/*.json` 四份语言包的 `regions.*` 文案
4. 本文件的掩码表

`mask::names()` 与 `REGIONS` 的顺序一致性有单测守护
（`mask_names_order_matches_frontend_regions`）。

## 目录结构

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

### 不要踩的约定

- **前后端契约同源**：`src/api.ts` 的 `Config` 必须与 `src-tauri/src/config.rs`
  的 `Config` 字段一一对应；三处白名单（`config.rs` 的 `THEMES`/`ACCENT_KEYS`/
  `LANGUAGES`、`src/accents.ts`、`src/i18n/index.ts`）也要同步。
- **锁**：不要用 `Mutex::lock().unwrap()`。release 是 `panic = "abort"`，
  持锁线程一旦 panic，后续取锁会直接终止整个进程。用 `lib.rs` 的 `lock_or_recover`。
- **配置值不可信**：一律经过 `Config::sanitize()`。
- **清理入口**：界面 / 托盘 / 热键 / 自动清理统一走 `lib.rs::perform_clean()`，
  不要各自调用 `memory::clean_memory`。

## 提交规范

- 一次提交只做一件事,提交信息用简洁的英文或中文描述改动
- 保持 `cargo fmt`、`cargo clippy -- -D warnings`、`cargo test`、`npx tsc --noEmit` 全部通过
- 涉及功能变更时请补充或更新测试

## 许可

本项目基于 [henrypp/memreduct](https://github.com/henrypp/memreduct)
(GPL-3.0) 派生重写,采用相同的 **GPL-3.0** 许可。分发衍生产品时请遵守 GPL-3.0 条款。
