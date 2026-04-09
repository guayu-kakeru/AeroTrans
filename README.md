# AeroTrans

> 极简、无感、AI 驱动的桌面翻译与词汇记忆工具。  
> Built with **Tauri + Vue 3 + Rust + SQLite**.

## ✨ 项目亮点
- **Spotlight 极速翻译**：全局快捷键一键呼出，输入后即时翻译，适合阅读流场景。
- **划词即翻译**：在任意应用选中文本，按快捷键即可自动带入翻译页并完成翻译。
- **透明悬浮与置顶**：支持窗口透明度调节、窗口置顶（图钉）、阅读场景小窗。
- **鼠标穿透模式**：Companion 常驻时可开启鼠标穿透，减少遮挡与打断。
- **多翻译引擎**：有道 / Google / MyMemory / AI（OpenAI 兼容接口）。
- **单词本学习闭环**：查词后可加入单词本，支持复习模式与列表模式。
- **AI 联想记忆生成**：可自定义提示词并重新生成，形成个性化记忆内容。
- **本地优先与轻量运行**：SQLite 本地存储，Tauri 架构低资源占用。

## 🧩 核心能力
### 1) 翻译模式
- 主窗口翻译页：输入词/句翻译，支持音标、来源标识、词汇收录。
- Spotlight 窗口：极简输入 + 结果面板，适合高频查词。

### 2) Companion 阅读模式
- 剪贴板监听刷新翻译
- 阅读小窗模式
- 透明度调节
- 鼠标穿透
- 窗口置顶

### 3) 单词本与记忆
- 复习模式（聚焦回忆）
- 列表模式（总览管理）
- AI 联想记忆生成 / 重新生成

## ⌨️ 默认快捷键
- `Spotlight`：`Alt + Shift + F`
- `划词翻译`：`Alt + Shift + L`

> 快捷键可在设置页录制和修改。

## 🚀 快速开始
### 运行已构建版本
在 `artifacts/` 目录运行：
- `AeroTrans.exe`
- `WebView2Loader.dll`（需与 exe 同目录）

### 本地开发
```bash
npm install
npm run dev
```

### Tauri 调试
```bash
npm run tauri:dev
```

### 测试
```bash
npm test
cargo +stable-x86_64-pc-windows-gnullvm test --manifest-path src-tauri/Cargo.toml
```

### 构建（Windows）
```bash
$env:RUSTUP_TOOLCHAIN='stable-x86_64-pc-windows-gnullvm'
npm run tauri:build -- --target x86_64-pc-windows-gnullvm
```

## ⚙️ AI 配置说明
AeroTrans 支持接入 OpenAI 兼容 API（如 DeepSeek / Kimi / OpenRouter 等）：
- API Base URL
- API Key
- Model Name

API Key 采用系统凭据管理机制存储（Windows Credential Manager / macOS Keychain），不以明文写入 SQLite。

## ⚠️ 已知说明（联想记忆）
使用**免费 AI 模型**生成“联想记忆”时，可能出现以下情况：
- 输出风格不稳定（有时偏长、偏空泛）
- 场景联想质量波动较大
- 偶发格式不一致

建议：
- 免费模型用于功能体验与轻量使用
- 对记忆质量有较高要求时，建议使用更稳定的高质量模型
- 结合自定义提示词与“重新生成”获得更优结果

## 🗂️ 项目结构
```text
src/            # Vue 前端（UI、状态管理、样式）
src-tauri/      # Rust 后端（系统能力、快捷键、SQLite、API 调用）
artifacts/      # 可分发运行产物
```

## 🛣️ 路线图
- [x] Phase A - Engineering Foundation
- [ ] Phase B - MVP Translation Flow
- [ ] Phase C - Vocabulary Learning Enhancements

## 📄 License
MIT