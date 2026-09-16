<p align="center">
  <img src="Logo.png" alt="Plico logo" width="128">
</p>

<p align="center">本地优先的桌面剪贴板管理器。</p>

<p align="center">
  <img alt="License" src="https://img.shields.io/badge/license-MIT-blue?style=flat-square">
  <img alt="Platform" src="https://img.shields.io/badge/platform-Windows-0078D4?style=flat-square">
  <img alt="Tauri" src="https://img.shields.io/badge/Tauri-2-24C8DB?style=flat-square&logo=tauri&logoColor=white">
  <img alt="React" src="https://img.shields.io/badge/React-19-61DAFB?style=flat-square&logo=react&logoColor=white">
  <img alt="Rust" src="https://img.shields.io/badge/Rust-1.77%2B-000000?style=flat-square&logo=rust&logoColor=white">
</p>

---

Plico 是一个常驻托盘的剪贴板管理器：你复制过的文本、图片、文件路径都会被自动记下来，之后按一下全局快捷键就能唤起面板，搜到、回车、粘进当前光标处。

**所有数据只存在你自己的机器上**，不联网、不上传、没有账号。界面是像素风，挺好看的。

对标 Ditto / Paste / Raycast Clipboard History 这一类工具，但只做核心闭环，不塞花活。

### 功能

**记录**

- 支持文本、富文本（同时保留纯文本副本）、图片、文件/文件夹列表
- SHA-256 内容去重：重复复制同一内容只更新时间戳，不堆垃圾
- 记录复制时的来源应用（进程名 + 窗口标题）

**唤起与粘贴**

- 全局快捷键唤起面板（默认 `Ctrl+Shift+V`，可在设置里重新录制）
- 回车粘贴到唤起面板之前的光标位置；`Ctrl+Enter` 粘贴为纯文本
- 可选「粘贴后恢复剪贴板」—— 减少敏感内容在剪贴板里的驻留时间
- 面板失焦自动隐藏（延迟可调），位置、尺寸会被记住

**面板**

- 无边框弹出面板，左侧列表 + 右侧预览，分栏可折叠
- 实时搜索，命中关键字高亮；输入法组合输入期间不触发中间态过滤
- 类型筛选：全部 / 文本 / 图片 / 文件 / 链接 / 颜色
- 置顶、批量选择、删除、编辑（内容可直接改完再粘）
- 键盘优先，可选 Vim 模式（`J` / `K` 上下移动）
- 自动识别链接（显示域名，站点标识本地生成，零网络请求）、颜色值（`#hex` / `rgb()` / `hsl()`，显示色块）、代码片段（语法高亮）

**组织**

- 分组管理，支持拖拽归组
- 标签
- 片段库：常用的签名、代码模板存起来，支持 `{{cursor}}` 占位符 —— 粘贴后光标自动停在占位处

**隐私与容量**

- 应用排除：来自密码管理器等指定进程的复制不会被记录
- 内容规则：匹配自定义正则的内容直接跳过
- 条数上限、保留天数、图片总容量都可设置；置顶条目永远不受自动清理影响

**系统集成**

- 托盘常驻，左键唤起面板，右键菜单
- 开机自启（默认关）
- 单实例：重复启动只唤起已有窗口

### 快捷键

面板内：

| 按键 | 动作 |
| --- | --- |
| `↑` / `↓`、`Ctrl+P` / `Ctrl+N` | 移动选中 |
| `J` / `K` | 移动选中（Vim 模式，可开关） |
| `Enter` | 粘贴选中项 |
| `Ctrl+Enter` | 粘贴为纯文本 |
| `P` | 置顶 / 取消置顶 |
| `Ctrl+A` | 全选当前列表 |
| `Ctrl+B` | 折叠 / 展开预览区 |
| `Ctrl+E` | 编辑选中项 |
| `Delete` | 删除选中项 |
| `Tab` / `Shift+Tab` | 切换类型筛选 |
| `Esc` | 逐级退出（取消批量选择 → 清空搜索词 → 关闭面板） |

全局：

| 按键 | 动作 |
| --- | --- |
| `Ctrl+Shift+V` | 显示 / 隐藏面板（可改） |
| `Ctrl+Shift+Alt+V` | 直接粘贴为纯文本（可改） |

### 技术栈

| 层 | 选型 |
| --- | --- |
| 桌面壳 | Tauri 2 |
| 前端 | React 19 + TypeScript 5 + Vite 5 |
| 状态 | zustand |
| 样式 | Tailwind CSS 4 + 自建 CSS 变量主题 |
| 剪贴板读写 | `arboard` |
| 模拟粘贴 | `enigo` |
| 剪贴板变化检测 | 轮询 `GetClipboardSequenceNumber`（Windows）/ `NSPasteboard.changeCount`（macOS） |
| 存储 | SQLite（`rusqlite` bundled，WAL 模式） |
| 字体 | Fusion Pixel 12px（中文）、Press Start 2P（像素标题），均为 OFL-1.1 |

### 数据存储

固定放在安装目录下，**不提供修改入口**：

```
<安装目录>\data\
├── plico.db        # SQLite 主库
├── images/         # 原始图片（PNG）
├── thumbs/         # 列表缩略图（256px）
└── logs/
```

> 安装目录不可写时（比如装到了 `C:\Program Files`）会自动回退到 `%APPDATA%\com.plico.app\`。

### 安装

目前请从源码构建（预编译安装包会陆续发到 [Releases](../../releases)）。

Windows 需要先装 **MSVC C++ 生成工具 + Windows SDK**，Tauri 在 Windows 上依赖 MSVC 工具链（`webview2-com`、`windows` crate、以及 `rusqlite` 的 bundled SQLite 都要用 C 编译器）。

### 从源码构建

前置：**Node 22+**、**Rust 1.77+**、**MSVC C++ 生成工具 + Windows SDK**、**WebView2**（Windows 10/11 自带）。

```bash
# 克隆
git clone <repo-url>
cd Plico

# 安装前端依赖
npm install

# 开发模式（热重载）
npm run tauri:dev

# 打 NSIS 安装包 → src-tauri/target/release/bundle/nsis/
npm run tauri:build
```

其他命令：

| 命令 | 作用 |
| --- | --- |
| `npm run typecheck` | 只做前端类型检查（不需要 MSVC） |
| `npm run dev` | 只在浏览器里跑前端，走 mock 数据层 |
| `cd src-tauri && cargo test` | 跑 Rust 侧单元测试 |

> 直接在浏览器里 `npm run dev` 看到的是**假数据**（`src/lib/mock.ts`），仅用于调 UI。真实剪贴板数据只存在于 Tauri 窗口里。

### 平台支持

| 平台 | 状态 |
| --- | --- |
| Windows | ✅ 已完成，构建与打包跑通 |
| macOS | 🚧 未完成 |

macOS 的跨平台骨架已经在了（热键、修饰键、剪贴板读写、图标 `icon.icns`），但还有阻断性缺陷没处理，暂时不能直接用。

### 许可证

本项目基于 [MIT 许可证](LICENSE) 开源。
