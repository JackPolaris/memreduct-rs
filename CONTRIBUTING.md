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

## 多语言

界面支持 16 种语言，安装程序也提供同样 16 种。**新增一种语言要改 5 个地方**，
而且除了第一处之外都不会报错，只会静默失效：

| 位置 | 作用 | 漏改的后果 |
| --- | --- | --- |
| `src/i18n/<code>.json` | 翻译文案，键集合必须与 `en-US.json` 完全相同（当前 121 条） | 缺键回落显示中文 |
| `src/i18n/index.ts` 的 `SUPPORTED_LANGUAGES` | 下拉选项 / `LanguageCode` 类型 / 前缀归一化表 | 选项不出现；`RESOURCES` 是 `Record<LanguageCode, …>`，漏了会编译报错 |
| `src-tauri/src/config.rs` 的 `LANGUAGES` | 后端校验白名单 | **静默**：保存时被 `sanitize()` 改回 `zh-CN` |
| `src-tauri/src/installer_lang.rs` 的 `LCIDS` | 安装器语言 → 界面语言映射 | 安装时选了也不会被继承（有单测拦着） |
| `tauri.conf.json` 的 `bundle.windows.nsis.languages` | 安装界面语言列表 | 安装界面没有该语言 |

### 安装程序语言的两个坑

- **Tauri 只自带 21 种安装界面译文**（`tauri-bundler/.../nsis/languages/`）。
  `languages` 里列了却没有对应 `.nsh` 的语言**不会报错** —— Tauri 只打一条
  `log::warn!`，然后把该语言从安装包里丢掉。官方的 21 种里没有波兰语、越南语、
  泰语、印尼语，这 4 种由 `src-tauri/nsis-lang/*.nsh` 补齐；文件里的键必须与
  Tauri 的 `English.nsh` 逐字一致（连 `choowHowToInstall` 这个拼写错误也要照抄，
  它是契约）。
- `customLanguageFiles` 的 key 必须同时出现在 `languages` 里，否则**完全静默忽略**
  —— bundler 只拿 `languages` 去查 `customLanguageFiles`，从不反向校验。

### 安装时选择的语言如何传给程序

1. `displayLanguageSelector` 打开后，安装器弹出语言选择框。不选择时跟随**系统
   语言**；系统语言不在列表内则回退到 `languages` 的第一项（刻意放的英语）。
2. `nsis-lang/hooks.nsh` 的 `NSIS_HOOK_POSTINSTALL` 把 `$LANGUAGE`（LCID 数字）
   写进 `HKCU\Software\memreduct\Mem Reduct` 的 `Installer Language`。
   **这一步不能省**：Tauri 的模板声明了 `MUI_LANGDLL_REGISTRY_*`，却从不插入
   唯一会写值的 `MUI_LANGDLL_SAVELANGUAGE`，所以那个值只读不写。不写回还有第二个
   后果 —— 语言框只在 `/S` 下被跳过，而自动更新走 `/P`，于是每次后台更新都会
   弹一个模态语言框（点"取消"＝`Abort`，整次安装中止）。
3. 程序**首次启动**（`config.json` 还不存在）时，`installer_lang.rs` 读该值、
   映射成语言码写进配置。此后 `config.json` 已存在就不会再读注册表，
   **所以在应用内改的语言不会被后续更新或重装覆盖**。

注册表路径里的 `memreduct` 来自 `identifier` 的第二段（`bundle.publisher` 未设置
时的回退值）。`tauri.conf.json` 已显式写出 `publisher` 把它固定住；改动
`publisher` 必须同步改 `installer_lang.rs` 的 `REG_SUBKEY`。

## 发布新版本

自动更新依赖两样东西：**GitHub Release 上的安装包** 与 **更新清单 JSON**。
客户端的更新地址固定为：

```
https://github.com/JackPolaris/memreduct-rs/releases/latest/download/update-x86_64-pc-windows-msvc.json
```

（`aarch64` / `i686` 会各自请求 `update-aarch64-pc-windows-msvc.json` 等，
由 `src-tauri/src/updater.rs` 按编译架构拼出。当前只发布 x86_64。）

### 用脚本发布（推荐）

```bash
# 一次跑完:预检 → 改版本号 → 构建 → 生成清单 → 发布 → 校验
npm run release -- all 3.5.14

# 也可以分步执行,便于定位问题
npm run release -- preflight     # 只做检查,无副作用
npm run release -- bump 3.5.14
npm run release -- build
npm run release -- manifest
npm run release -- publish
npm run release -- verify
```

选项：`--dry-run` 预演、`--bundles nsis|msi|all`、`--yes` 免确认、`--allow-dirty`。
实现与设计取舍见 `scripts/release.mjs` 头部注释。

**发版请优先用脚本**，它把下面这些踩过的坑都固化了：

| 脚本做的事 | 不这么做会怎样 |
|---|---|
| 预检私钥与 `tauri.conf.json` 里的 pubkey 是否配对 | 配错要等 11 分钟构建完才在签名步骤失败 |
| 显式设置空的 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 缺这个变量时 CLI 会挂在密码提示上，非交互环境等于永久卡死 |
| 同步全部 5 个文件的版本号 | `Cargo.lock` 最容易漏，且不会导致构建失败，只是元数据不一致 |
| 清单 `url` 与资产文件名出自同一处 | 手抄文件名不一致 → 客户端下载 404 |
| `notes` 从 `CHANGELOG.md` 对应小节自动提取 | 漏写 `notes` 时更新提示没有内容 |
| 发布前用内置公钥对安装包做**真实 Ed25519 验签** | 签名不匹配时每个客户端都会拒绝这次更新 |
| `git push` 被代理拦截时自动改用 GitHub REST API 推送 | 代理过滤 `github.com` 时无法发布（对象按本地重建，逐条校验 SHA 一致） |
| 发布后校验资产、`releases/latest` 指向与远端清单 | 只有用户装不上时才发现问题 |

## 手动流程（脚本出问题时的参考）

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

### 5. 发布后自检

1. `gh release view vX.Y.Z` 确认三个资产都是 `uploaded`，且 `releases/latest` 指向新 tag；
2. 用 `gh api repos/OWNER/REPO/releases/tags/vX.Y.Z --jq '.assets[] | "\(.name) \(.digest)"'`
   把远端 sha256 与本地 `sha256sum` 对照，确认上传的字节就是本地产物；
3. 在应用「关于」页点「更新地址」旁的链接，浏览器应能直接看到清单 JSON。

**关于"检查更新失败"的排查**：客户端的更新请求走 `github.com`，
而 `api.github.com`、`uploads.github.com` 是另外的域名 —— 有些代理只放行后者，
于是 `gh` 一切正常、应用却检查不到更新。两个已知表现：

- 端点被拦截（返回 502/403）时，**更新插件会把非 2xx 响应报成「未找到发布」**，
  而不是网络错误（它只为真正的传输失败记录错误）。所以"未找到发布"不等于清单有问题；
- 因此 `src-tauri/src/updater.rs` 在代理失败后会**忽略代理重试一次**，
  并把实际使用的端点与失败原因显示在「关于」页。

排查清单：浏览器能否打开清单地址 → 应用「关于」页显示的失败原因 → 该机代理是否过滤 `github.com`。

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
│  ├─ i18n/                # 多语言资源（16 种，见「多语言」一节）
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
│  │  ├─ installer_lang.rs # 首次启动继承安装时选择的语言
│  │  └─ cmdline.rs        # 命令行解析
│  ├─ nsis-lang/           # 安装程序语言文件与钩子（见「多语言」一节）
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
- **加一种语言要改 5 处**，漏任何一处都不会报错，只会静默失效：
  `src/i18n/<code>.json`（键集合必须与 `en-US.json` 完全相同）、
  `src/i18n/index.ts` 的 `SUPPORTED_LANGUAGES`、`config.rs` 的 `LANGUAGES`、
  `installer_lang.rs` 的 LCID 映射表、`tauri.conf.json` 的安装器语言列表。
  细节见「多语言」一节。
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
