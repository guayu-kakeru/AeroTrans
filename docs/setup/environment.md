# AeroTrans 环境准备

本项目约定：
- 所有新增环境组件放在 `D:\environment\<component-name>`
- 推荐组件目录：
  - `D:\environment\nodejs`
  - `D:\environment\pnpm`
  - `D:\environment\rustup`
  - `D:\environment\tauri-cli`

## 自检命令

```powershell
powershell -ExecutionPolicy Bypass -File scripts/setup-environment.ps1
```

## 修复模式

```powershell
powershell -ExecutionPolicy Bypass -File scripts/setup-environment.ps1 -Fix
```

该脚本不会自动下载未知来源安装包，只负责目录创建和命令可用性检查。
