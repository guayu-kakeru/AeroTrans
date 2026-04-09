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
