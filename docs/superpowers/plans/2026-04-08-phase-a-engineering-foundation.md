# Phase A Engineering Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 交付 AeroTrans 的工程化基础（仓库规范、可测试代码骨架、安全凭据存储、数据库迁移机制、CI/CD 基线），为后续 Phase B/Phase C 功能开发提供稳定底座。

**Architecture:** 采用 Tauri（Rust）+ Vue 3（TypeScript）双层架构。Phase A 只实现“可验证的底层能力”和工程化护栏，不提前堆 UI 业务复杂度。通过独立模块（迁移、凭据、剪贴板策略、队列引擎）+ 单元测试先锁定边界行为。

**Tech Stack:** Tauri 2.x, Rust + rusqlite, Vue 3 + TypeScript + Tailwind CSS, Vitest, GitHub Actions, keyring/OS credential store

---

## Scope Check

当前规格已拆成三个子系统（Phase A/B/C）。本计划仅覆盖 `Phase A`，确保可以独立交付并通过 CI。Phase B（MVP 主链路）和 Phase C（学习系统增强）将分别生成独立计划文档，避免跨阶段耦合。

## File Structure Map

- `scripts/setup-environment.ps1`: 初始化本机依赖目录（`D:\environment`）并校验 Node/Rust/Pnpm/Tauri CLI。
- `docs/setup/environment.md`: 记录环境放置策略与安装命令。
- `.gitignore`: 仓库脱敏与本地产物忽略规则。
- `docs/engineering/git-flow.md`: 分支协作规范（main/dev/feature）。
- `src/lib/reviewQueue.ts`: 复习队列纯逻辑引擎（后续 Phase C 复用）。
- `src/lib/clipboardPolicy.ts`: Companion 剪贴板智能节流与文本校验策略。
- `src/lib/__tests__/reviewQueue.test.ts`: 队列边界测试。
- `src/lib/__tests__/clipboardPolicy.test.ts`: 剪贴板策略测试。
- `src-tauri/src/db/migrations.rs`: SQLite `user_version` 迁移执行器。
- `src-tauri/migrations/0001_initial.sql`: 首版表结构。
- `src-tauri/migrations/0002_vocab_ai_memory.sql`: AI 字段升级脚本。
- `src-tauri/src/security/credential_store.rs`: OS 凭据存储抽象层。
- `src-tauri/src/security/mod.rs`: 安全模块导出。
- `src-tauri/src/lib.rs`: 挂载 db/security 模块。
- `src-tauri/src/db/mod.rs`: db 模块导出。
- `src-tauri/tests/migration_tests.rs`: 迁移升级测试。
- `src-tauri/tests/credential_store_tests.rs`: 凭据存储行为测试（mock）。
- `.github/workflows/ci.yml`: `dev/main` PR 测试流水线。
- `.github/workflows/build.yml`: 面向 `main` PR 的探索型打包流水线（Windows/macOS）。

### Task 1: 环境目录与工具链自检脚本

**Files:**
- Create: `scripts/setup-environment.ps1`
- Create: `docs/setup/environment.md`
- Modify: `README.md`

- [ ] **Step 1: 写入环境自检脚本**

```powershell
param(
  [switch]$Fix
)

$ErrorActionPreference = "Stop"
$base = "D:\environment"
$components = @("nodejs", "pnpm", "rustup", "tauri-cli")

foreach ($name in $components) {
  $path = Join-Path $base $name
  if (-not (Test-Path $path)) {
    New-Item -ItemType Directory -Force -Path $path | Out-Null
  }
}

function Test-Cmd($cmd) {
  $null = Get-Command $cmd -ErrorAction SilentlyContinue
  return $null -ne $null
}

$checks = @(
  @{ Name = "node"; Ok = (Test-Cmd "node") },
  @{ Name = "pnpm"; Ok = (Test-Cmd "pnpm") },
  @{ Name = "cargo"; Ok = (Test-Cmd "cargo") },
  @{ Name = "rustup"; Ok = (Test-Cmd "rustup") }
)

$failed = $checks | Where-Object { -not $_.Ok }

if ($failed.Count -eq 0) {
  Write-Host "ENV CHECK: PASS"
  exit 0
}

Write-Host "ENV CHECK: FAIL"
$failed | ForEach-Object { Write-Host ("Missing command: " + $_.Name) }

if (-not $Fix) {
  Write-Host "Run with -Fix after placing installers under D:\environment\\<component>."
  exit 1
}

Write-Host "-Fix mode enabled. Install missing tools using files in D:\environment."
exit 1
```

- [ ] **Step 2: 运行脚本确认当前状态**

Run: `powershell -ExecutionPolicy Bypass -File scripts/setup-environment.ps1`
Expected: `ENV CHECK: PASS`（或列出缺失命令并返回非 0）

- [ ] **Step 3: 编写环境说明文档**

```markdown
# AeroTrans 环境准备

本项目要求：
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

脚本不会自动下载未知来源安装包，只负责目录和命令可用性验证。
```

- [ ] **Step 4: 在 README 增加入口**

```markdown
## Development Environment

1. Place required installers/tools in `D:\environment\<component-name>`.
2. Run environment check:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/setup-environment.ps1
```
```

- [ ] **Step 5: 提交**

```bash
git add scripts/setup-environment.ps1 docs/setup/environment.md README.md
git commit -m "chore: add environment bootstrap checks under D:\\environment"
```

### Task 2: 仓库规范与分支协作护栏

**Files:**
- Modify: `.gitignore`
- Create: `docs/engineering/git-flow.md`

- [ ] **Step 1: 写入 .gitignore（先写测试数据行，保证规则生效）**

```gitignore
# Frontend dependencies
node_modules/

# Tauri/Rust build output
src-tauri/target/
Cargo.lock

# Environment and secrets
.env
.env.*

# Local database
*.db
*.sqlite

# Local brainstorm artifacts
.superpowers/
```

- [ ] **Step 2: 验证忽略规则**

Run: `git check-ignore -v node_modules/foo.js src-tauri/target/debug/app.exe .env.local test.sqlite`
Expected: 每个路径都命中 `.gitignore` 对应规则

- [ ] **Step 3: 编写 Git Flow 规范文档**

```markdown
# AeroTrans Git Flow

## Branches
- `main`: 仅可发布版本
- `dev`: 集成测试分支
- `feature/*`: 功能分支，从 `dev` 切出并 PR 回 `dev`

## Rules
- 禁止直接向 `main` 提交业务代码
- 所有功能通过 PR 合入 `dev`
- `dev` -> `main` 通过发布 PR 完成

## Suggested Commands
```bash
git checkout -b dev

git checkout -b feature/phase-a-foundation dev
```
```

- [ ] **Step 4: 校验文档可读性**

Run: `rg -n "main|dev|feature" docs/engineering/git-flow.md`
Expected: 至少命中 3 类分支规则

- [ ] **Step 5: 提交**

```bash
git add .gitignore docs/engineering/git-flow.md
git commit -m "docs: add git flow and repository hygiene rules"
```

### Task 3: 前端基础测试框架 + Review Queue 引擎（TDD）

**Files:**
- Create: `src/lib/reviewQueue.ts`
- Create: `src/lib/__tests__/reviewQueue.test.ts`
- Modify: `package.json`
- Modify: `vitest.config.ts`

- [ ] **Step 1: 先写失败测试（覆盖边界）**

```ts
import { describe, expect, it } from "vitest";
import { createReviewQueue, nextIndex, prevIndex, removeAt } from "../reviewQueue";

describe("reviewQueue", () => {
  it("cycles next index", () => {
    expect(nextIndex(0, 3)).toBe(1);
    expect(nextIndex(2, 3)).toBe(0);
  });

  it("cycles previous index", () => {
    expect(prevIndex(0, 3)).toBe(2);
    expect(prevIndex(2, 3)).toBe(1);
  });

  it("removes current and focuses next item", () => {
    const state = createReviewQueue(["a", "b", "c"], 1);
    const updated = removeAt(state, 1);
    expect(updated.items).toEqual(["a", "c"]);
    expect(updated.index).toBe(1);
  });

  it("returns empty focus when removing last item", () => {
    const state = createReviewQueue(["x"], 0);
    const updated = removeAt(state, 0);
    expect(updated.items).toEqual([]);
    expect(updated.index).toBe(-1);
  });
});
```

- [ ] **Step 2: 运行测试确认失败**

Run: `pnpm vitest src/lib/__tests__/reviewQueue.test.ts`
Expected: FAIL，提示 `Cannot find module '../reviewQueue'`

- [ ] **Step 3: 实现最小通过代码**

```ts
export interface ReviewQueueState<T> {
  items: T[];
  index: number;
}

export function createReviewQueue<T>(items: T[], index = 0): ReviewQueueState<T> {
  if (items.length === 0) {
    return { items: [], index: -1 };
  }

  const safe = Math.min(Math.max(index, 0), items.length - 1);
  return { items: [...items], index: safe };
}

export function nextIndex(index: number, length: number): number {
  if (length <= 0) return -1;
  return (index + 1) % length;
}

export function prevIndex(index: number, length: number): number {
  if (length <= 0) return -1;
  return (index - 1 + length) % length;
}

export function removeAt<T>(state: ReviewQueueState<T>, removeIndex: number): ReviewQueueState<T> {
  if (state.items.length === 0) return { items: [], index: -1 };

  const nextItems = state.items.filter((_, i) => i !== removeIndex);
  if (nextItems.length === 0) return { items: [], index: -1 };

  let nextFocus = state.index;
  if (removeIndex < state.index) {
    nextFocus = state.index - 1;
  } else if (removeIndex === state.index && state.index >= nextItems.length) {
    nextFocus = nextItems.length - 1;
  }

  return { items: nextItems, index: nextFocus };
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `pnpm vitest src/lib/__tests__/reviewQueue.test.ts --run`
Expected: PASS（4 passed）

- [ ] **Step 5: 提交**

```bash
git add src/lib/reviewQueue.ts src/lib/__tests__/reviewQueue.test.ts package.json vitest.config.ts
git commit -m "test: add review queue engine with boundary-safe index behavior"
```

### Task 4: Companion 剪贴板策略模块（智能节流 + 文本校验）

**Files:**
- Create: `src/lib/clipboardPolicy.ts`
- Create: `src/lib/__tests__/clipboardPolicy.test.ts`

- [ ] **Step 1: 写失败测试**

```ts
import { describe, expect, it } from "vitest";
import { shouldReadClipboard, defaultClipboardPolicy } from "../clipboardPolicy";

describe("clipboardPolicy", () => {
  it("accepts short plain text", () => {
    const ok = shouldReadClipboard(defaultClipboardPolicy(), Date.now(), 0, "hello world");
    expect(ok).toBe(true);
  });

  it("rejects too frequent polling", () => {
    const now = Date.now();
    const ok = shouldReadClipboard(defaultClipboardPolicy(), now, now - 50, "hello");
    expect(ok).toBe(false);
  });

  it("rejects oversized text", () => {
    const value = "a".repeat(2000);
    const ok = shouldReadClipboard(defaultClipboardPolicy(), Date.now(), 0, value);
    expect(ok).toBe(false);
  });

  it("rejects non-text payload", () => {
    const ok = shouldReadClipboard(defaultClipboardPolicy(), Date.now(), 0, null);
    expect(ok).toBe(false);
  });
});
```

- [ ] **Step 2: 运行测试确认失败**

Run: `pnpm vitest src/lib/__tests__/clipboardPolicy.test.ts --run`
Expected: FAIL，提示 `Cannot find module '../clipboardPolicy'`

- [ ] **Step 3: 实现策略代码**

```ts
export interface ClipboardPolicy {
  minIntervalMs: number;
  minLength: number;
  maxLength: number;
}

export function defaultClipboardPolicy(): ClipboardPolicy {
  return {
    minIntervalMs: 350,
    minLength: 1,
    maxLength: 512,
  };
}

export function shouldReadClipboard(
  policy: ClipboardPolicy,
  nowMs: number,
  lastReadMs: number,
  value: string | null,
): boolean {
  if (nowMs - lastReadMs < policy.minIntervalMs) return false;
  if (typeof value !== "string") return false;

  const text = value.trim();
  if (text.length < policy.minLength) return false;
  if (text.length > policy.maxLength) return false;

  return true;
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `pnpm vitest src/lib/__tests__/clipboardPolicy.test.ts --run`
Expected: PASS（4 passed）

- [ ] **Step 5: 提交**

```bash
git add src/lib/clipboardPolicy.ts src/lib/__tests__/clipboardPolicy.test.ts
git commit -m "test: add throttled clipboard policy with text guards"
```

### Task 5: Rust 数据库迁移引擎（`user_version`）

**Files:**
- Create: `src-tauri/src/db/mod.rs`
- Create: `src-tauri/src/db/migrations.rs`
- Create: `src-tauri/migrations/0001_initial.sql`
- Create: `src-tauri/migrations/0002_vocab_ai_memory.sql`
- Create: `src-tauri/tests/migration_tests.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: 写失败测试（先定义迁移预期）**

```rust
use rusqlite::Connection;

use aerotrans_lib::db::migrations::run_migrations;

#[test]
fn migrates_from_empty_to_latest() {
    let conn = Connection::open_in_memory().unwrap();
    run_migrations(&conn).unwrap();

    let version: i32 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();

    assert_eq!(version, 2);

    let exists: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('vocabulary') WHERE name = 'ai_memory'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(exists, 1);
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --test migration_tests`
Expected: FAIL，提示 `could not find db in aerotrans_lib`

- [ ] **Step 3: 实现迁移执行器与 SQL**

```rust
use rusqlite::{Connection, Result};

pub struct Migration {
    pub version: i32,
    pub sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        sql: include_str!("../../migrations/0001_initial.sql"),
    },
    Migration {
        version: 2,
        sql: include_str!("../../migrations/0002_vocab_ai_memory.sql"),
    },
];

pub fn run_migrations(conn: &Connection) -> Result<()> {
    let current: i32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;

    for migration in MIGRATIONS {
        if migration.version > current {
            conn.execute_batch("BEGIN;")?;
            if let Err(err) = conn.execute_batch(migration.sql) {
                let _ = conn.execute_batch("ROLLBACK;");
                return Err(err);
            }
            conn.execute_batch(&format!("PRAGMA user_version = {};", migration.version))?;
            conn.execute_batch("COMMIT;")?;
        }
    }

    Ok(())
}
```

```sql
CREATE TABLE IF NOT EXISTS vocabulary (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  term TEXT NOT NULL,
  translation TEXT NOT NULL,
  context_text TEXT,
  starred INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

```sql
ALTER TABLE vocabulary ADD COLUMN ai_memory TEXT;
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --test migration_tests`
Expected: PASS（`migrates_from_empty_to_latest ... ok`）

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/db/mod.rs src-tauri/src/db/migrations.rs src-tauri/migrations/0001_initial.sql src-tauri/migrations/0002_vocab_ai_memory.sql src-tauri/tests/migration_tests.rs src-tauri/src/lib.rs src-tauri/Cargo.toml
git commit -m "feat: add sqlite migration runner with user_version"
```

### Task 6: OS 凭据存储抽象（API Key 不落 SQLite）

**Files:**
- Create: `src-tauri/src/security/mod.rs`
- Create: `src-tauri/src/security/credential_store.rs`
- Create: `src-tauri/tests/credential_store_tests.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: 写失败测试（基于内存 mock）**

```rust
use aerotrans_lib::security::credential_store::{InMemoryStore, SecretStore};

#[test]
fn stores_and_reads_api_key() {
    let store = InMemoryStore::default();
    store.set_secret("openai_api_key", "sk-test").unwrap();

    let loaded = store.get_secret("openai_api_key").unwrap();
    assert_eq!(loaded.as_deref(), Some("sk-test"));
}

#[test]
fn deletes_api_key() {
    let store = InMemoryStore::default();
    store.set_secret("openai_api_key", "sk-test").unwrap();
    store.delete_secret("openai_api_key").unwrap();

    let loaded = store.get_secret("openai_api_key").unwrap();
    assert!(loaded.is_none());
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --test credential_store_tests`
Expected: FAIL，提示 `could not find security in aerotrans_lib`

- [ ] **Step 3: 实现凭据抽象与 mock 存储**

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub trait SecretStore: Send + Sync {
    fn set_secret(&self, key: &str, value: &str) -> anyhow::Result<()>;
    fn get_secret(&self, key: &str) -> anyhow::Result<Option<String>>;
    fn delete_secret(&self, key: &str) -> anyhow::Result<()>;
}

#[derive(Default, Clone)]
pub struct InMemoryStore {
    data: Arc<Mutex<HashMap<String, String>>>,
}

impl SecretStore for InMemoryStore {
    fn set_secret(&self, key: &str, value: &str) -> anyhow::Result<()> {
        self.data.lock().unwrap().insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn get_secret(&self, key: &str) -> anyhow::Result<Option<String>> {
        Ok(self.data.lock().unwrap().get(key).cloned())
    }

    fn delete_secret(&self, key: &str) -> anyhow::Result<()> {
        self.data.lock().unwrap().remove(key);
        Ok(())
    }
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --test credential_store_tests`
Expected: PASS（2 passed）

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/security/mod.rs src-tauri/src/security/credential_store.rs src-tauri/tests/credential_store_tests.rs src-tauri/src/lib.rs src-tauri/Cargo.toml
git commit -m "feat: add credential store abstraction for api keys"
```

### Task 7: CI 与探索型打包流水线

**Files:**
- Create: `.github/workflows/ci.yml`
- Create: `.github/workflows/build.yml`

- [ ] **Step 1: 写 CI workflow（`dev` + `main` PR）**

```yaml
name: ci

on:
  pull_request:
    branches: [dev, main]

jobs:
  frontend:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
        with:
          version: 9
      - uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: pnpm
      - name: Install deps
        run: pnpm install --frozen-lockfile
      - name: Frontend tests
        run: pnpm vitest --run

  backend:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Rust tests
        working-directory: src-tauri
        run: cargo test --all-targets
```

- [ ] **Step 2: 写 build workflow（main PR 探索打包）**

```yaml
name: build-preview

on:
  pull_request:
    branches: [main]

jobs:
  bundle:
    strategy:
      matrix:
        os: [windows-latest, macos-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
        with:
          version: 9
      - uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: pnpm
      - name: Install deps
        run: pnpm install --frozen-lockfile
      - name: Build frontend
        run: pnpm build
      - name: Build tauri app (no signing)
        uses: tauri-apps/tauri-action@v0
        with:
          args: --bundles app
```

- [ ] **Step 3: 本地校验 YAML 语法**

Run: `pnpm dlx yaml-lint .github/workflows/ci.yml .github/workflows/build.yml`
Expected: `0 problems found`

- [ ] **Step 4: 执行本地最小验证**

Run: `pnpm vitest --run && cargo test --manifest-path src-tauri/Cargo.toml`
Expected: 前端与 Rust 测试均 PASS

- [ ] **Step 5: 提交**

```bash
git add .github/workflows/ci.yml .github/workflows/build.yml
git commit -m "ci: add pull-request quality gates and preview bundles"
```

### Task 8: Phase A 验证与交付记录

**Files:**
- Create: `docs/engineering/phase-a-verification.md`
- Modify: `README.md`

- [ ] **Step 1: 写验证记录模板**

```markdown
# Phase A Verification

## Environment
- Command: `powershell -ExecutionPolicy Bypass -File scripts/setup-environment.ps1`
- Result: PASS/FAIL

## Frontend
- Command: `pnpm vitest --run`
- Result: PASS/FAIL

## Rust
- Command: `cargo test --manifest-path src-tauri/Cargo.toml`
- Result: PASS/FAIL

## CI Trigger Scope
- pull_request -> dev: enabled
- pull_request -> main: enabled

## Migration Compatibility
- user_version from 0 to 2: PASS/FAIL
```

- [ ] **Step 2: 运行全部验证命令并填写结果**

Run: `powershell -ExecutionPolicy Bypass -File scripts/setup-environment.ps1; pnpm vitest --run; cargo test --manifest-path src-tauri/Cargo.toml`
Expected: 所有命令返回码为 0

- [ ] **Step 3: 在 README 添加 Phase A 完成标记**

```markdown
## Project Milestones

- [x] Phase A - Engineering Foundation
- [ ] Phase B - MVP Translation Flow
- [ ] Phase C - Vocabulary Learning Enhancements
```

- [ ] **Step 4: 回归检查 Git 状态**

Run: `git status --short`
Expected: 工作区干净（无未提交改动）

- [ ] **Step 5: 提交**

```bash
git add docs/engineering/phase-a-verification.md README.md
git commit -m "docs: record phase-a verification and milestone status"
```

## Self-Review

### 1. Spec coverage

- 安全性（API Key 不明文存储）：Task 6。
- 剪贴板隐私约束（节流 + 文本校验 + 备用触发）：Task 4。
- Migration（`user_version` + 增量 SQL）：Task 5。
- Git 仓库规范（`.gitignore` + Git Flow）：Task 2。
- CI 要求（`dev/main` PR 触发）：Task 7。
- 环境放置要求（`D:\environment`）：Task 1。

### 2. Placeholder scan

- 已检查：本计划不包含 `TODO`、`TBD`、`implement later`、`fill in details`。

### 3. Type consistency

- `ReviewQueueState` / `nextIndex` / `prevIndex` / `removeAt` 在 Task 3 的测试与实现签名一致。
- `SecretStore` 在 Task 6 的测试与实现签名一致。
- `run_migrations` 在 Task 5 的测试与实现签名一致。

## Next Plans

- `Phase B` 计划文件：`docs/superpowers/plans/2026-04-08-phase-b-mvp-translation.md`
- `Phase C` 计划文件：`docs/superpowers/plans/2026-04-08-phase-c-learning-system.md`
