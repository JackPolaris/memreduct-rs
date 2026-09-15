# scripts/ — 图标工具链

> 这个目录长期存在 **三套互相竞争、输出同名文件** 的图标生成脚本，
> 每套生成的图标颜色都不一样。下面按“是否仍在维护”分组，
> 并标注每套脚本实际读取的源文件，避免下次改图标时改错地方。

## 当前有效链路（蓝色图标）

现在的应用图标是 **蓝色 `#3366FF`**，来源于 `assets/icon-reference.png`
（200×200 位图，由早期 `清理.png` 重命名而来）。

```bash
# 1) 从参考位图生成 5 个尺寸的 PNG 到 src-tauri/icons/
node scripts/render_from_png.mjs

# 2) 用这些 PNG 生成 BMP 内嵌式 icon.ico（兼容性最好）
python scripts/make_ico_bmp.py

# 3) 校验产物不是空白/透明（失败会以退出码 1 结束）
node scripts/verify_icon_png.mjs
```

| 脚本 | 输入 | 输出 | 说明 |
|---|---|---|---|
| `render_from_png.mjs` | `assets/icon-reference.png` | `32/128/128@2x/256/icon.png` | **当前有效**。sharp 缩放，自带品牌蓝 |
| `make_ico_bmp.py` | 上面生成的 PNG | `icon.ico`（BMP/DIB 内嵌） | **当前有效**。BMP 内嵌比 PNG 内嵌在各版本 Windows API 下更稳 |
| `verify_icon_png.mjs` | `src-tauri/icons/*.png` | 退出码 | 校验非空白（不透明像素占比 ≥ 50%） |
| `extract_ico_frame.py` | `icon.ico` | 帧表 + 临时 PNG | 诊断用；可传 `argv[1]` 指定输出路径 |

## 已废弃 / 仅诊断用（不要用来改图标）

这些脚本仍然保留，因为它们是排查“图标为什么是空白 / 绿色占位符”的历史记录，
但**它们生成的图标与当前发行版不一致**，误用会把品牌色改回去：

| 脚本 | 输入 | 会生成 | 为什么不再使用 |
|---|---|---|---|
| `render_icon.mjs` | `assets/icon.svg` | 紫色 `#676EBB` PNG | `icon.svg` 是旧的紫色 tdesign 图标，已不是品牌色 |
| `make_icon_png.py` | 无（代码内程序化绘制） | 紫色 + 白色横条的 PNG | 同上，且与发行版字形不同 |
| `make_icon.py` | 无 | 绿色圆角方块 ICO/PNG | 最早的占位图标，`#008040` 绿色 |
| `check_icon_png.mjs` | `assets/icon-reference.png` | 仅打印像素 | 纯诊断输出，没有断言 |
| `make_ico.py` | `src-tauri/icons/*.png` | PNG 内嵌 ICO | 被 `make_ico_bmp.py` 取代（PNG 内嵌在部分 API 下读色错误） |

> ⚠️ `assets/icon.svg`（紫色）与实际发行的蓝色图标不一致。
> `README.md` 曾声称 “`icon.svg` 为唯一源”，与事实相反 —— 若要继续用 SVG 路线，
> 必须先更新 `icon.svg` 并确认 `render_icon.mjs` 的输出色与品牌色一致。

## 三个脚本都会写同样的 5 个文件

`render_from_png.mjs`、`render_icon.mjs`、`make_icon_png.py` 的输出文件完全相同
（`32x32.png`、`128x128.png`、`128x128@2x.png`、`256x256.png`、`icon.png`）。
运行顺序不同会得到不同颜色的图标 —— 这是本目录最需要消除的隐患。
建议后续只保留一条链路（推荐上面那条），其余脚本移到 `scripts/legacy/`。
