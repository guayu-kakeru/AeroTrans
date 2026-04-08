# Execution Checkpoints

用于应对长会话或上下文切换时的续作能力，记录“已完成 / 待完成 / 验证证据”。

## 2026-04-08

### 已完成
- 搭建 `Vue 3 + Vite + Tailwind + Tauri` 可运行骨架。
- 实现核心页面：`Spotlight`、`Companion`、`单词本复习`、`设置中心`。
- 实现 Rust 命令层：
  - 设置读写与 API 配置读写
  - API Key 通过系统凭据存储（Keyring）
  - 翻译请求（AI + fallback）
  - 词本增删查
  - AI 记忆生成与重生成
  - 剪贴板读取
  - 快捷键冲突检测（系统保留组合拦截）
- 增量迁移升级到 `user_version = 3`（新增 `app_settings` 表）。
- 补充队列边界测试（连续翻页、删除焦点迁移、非法索引防护）。
- 打包生成可运行文件：`src-tauri/target/release/aerotrans.exe`。
- 进程烟雾验证：`SMOKE_OK`。

### 最近验证结果
- 前端测试：`13 passed`
- Rust 测试：`credential_store_tests`、`migration_tests` 通过
- 前端构建：`vite build` 通过
- Tauri 构建：`tauri build` 通过

### 待补充（后续可选）
- 全局快捷键“系统级占用检测”的更深层平台 API 集成（当前为保留组合冲突拦截 + 可扩展接口）。
- Companion 模式的窗口置顶、图钉、鼠标穿透的即时窗口行为控制命令（UI 已有设置项）。
