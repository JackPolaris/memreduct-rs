# Mem Reduct (Tauri + React 重构版)

用 **Tauri v2 + Rust 后端 + React 前端** 完全重构的开源内存清理工具，
完整复刻原版 [henrypp/memreduct](https://github.com/henrypp/memreduct) 的私有清理 API 和全部功能。

## 技术栈

- **前端**: React 18 + TypeScript + Vite 5（现代化卡片/面板式界面，dark/light 主题）
- **后端**: Rust + Tauri v2（`ntdll` 私有 API 手动绑定 + `windows` crate）
- **打包**: MSI / NSIS 安装程序

## 核心功能（完整复刻原版）

### 私有清理 API（通过 `NtSetSystemInformation` 调用未文档化的 NT 接口）

| 清理区域 | 掩码 | 底层调用 |
|---|---|---|
| 工作集 Working Set | `0x01` | `MemoryEmptyWorkingSets` |
| 系统文件缓存 System File Cache | `0x02` | `SystemFileCacheInformationEx`（Min/MaxWS=MAXSIZE_T） |
| 修改文件缓存 Modified File Cache | `0x80` | 枚举卷并 `FlushFileBuffers` |
| 修改页列表 Modified List | `0x10` | `MemoryFlushModifiedList` |
| Standby 列表 Standby List | `0x08` | `MemoryPurgeStandbyList` |
| Standby 优先级0列表 | `0x04` | `MemoryPurgeLowPriorityStandbyList` |
| 注册表缓存 Registry Cache | `0x40` | `SystemRegistryReconciliationInformation`（win8.1+） |
| 合并内存列表 Combine Lists | `0x20` | `SystemCombinePhysicalMemoryInformation`（win10+） |

### 内存监控（`NtQuerySystemInformation`）
- 物理内存、页面文件、系统缓存的总量/已用/可用/百分比
- 实时刷新（1s 轮询 + 后端事件推送）

### 功能项
- **托盘图标**：显示内存百分比数字（NIF_TITLE），tooltip 状态
- **自动清理**：阈值触发（默认 90%）/ 间隔触发（默认 30 分钟），共享 30s 冷却，auto 模式默认排除 freeze 区域
- **全局热键**：默认 Ctrl+F1（`RegisterHotKey`）
- **命令行**：`-clean`（默认区域）、`-clean:full`（全部区域）
- **8 区域勾选矩阵** + 全选/默认
- **配置存储**：portable（应用目录 `memreduct.json`）/ appdata（`%APPDATA%\Henry++\Mem Reduct`）
- **设置面板**：General / Memory / Appearance / Tray / Advanced 五大板块
- **dark/light 主题切换、颜色自定义、通知气泡/声音/日志**

## 项目结构

```
mem-reduct-tauri/
├─ src/                    # React 前端
│  ├─ App.tsx              # 主界面 + 设置面板
│  ├─ api.ts               # Tauri command 封装
│  ├─ regions.ts           # 清理区域掩码定义
│  └─ styles.css           # 主题与样式
├─ src-tauri/              # Rust 后端
│  ├─ src/
│  │  ├─ ntapi.rs          # 私有 NT API 绑定（未文档化类型/枚举）
│  │  ├─ memory.rs         # 内存采集 + 8 区域清理
│  │  ├─ config.rs         # 配置存储（portable/appdata）
│  │  ├─ tray.rs           # 系统托盘
│  │  ├─ hotkey.rs         # 全局热键
│  │  ├─ cmdline.rs        # 命令行解析
│  │  └─ lib.rs            # Tauri 状态、命令、后台循环
│  └─ tauri.conf.json      # Tauri 配置
└─ scripts/make_icon.py    # 图标生成脚本
```

## 开发与构建

```bash
# 安装依赖
npm install

# 开发运行
npm run tauri dev

# 前端类型检查
npx tsc --noEmit

# 后端单元测试
cd src-tauri && cargo test

# 打包安装程序
npm run tauri build
```

> 完整复刻的清理动作需**管理员权限**（原版同样如此）。
> 原版项目为 GPL-3.0 许可，使用/衍生分发时请遵守相应开源许可。
