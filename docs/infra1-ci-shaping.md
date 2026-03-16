---
shaping: true
---

# INFRA-1: CI/CD Pipeline — Shaping

## Frame

### Source

> I want proper CI.

From the codebase audit (INFRA-1): "No CI/CD pipeline — no GitHub Actions, no automated checks." R7 promoted from Nice-to-have to Must-have.

### Problem

14 of 25 audit findings have been fixed, but nothing prevents regressions. Any commit can silently reintroduce clippy warnings, break tests, or skip compile-time query checks.

### Outcome

Every push to `main` and every PR targeting `main` is automatically checked against R1–R6 before merging.

---

## Requirements (R)

| ID | Requirement | Status |
|----|-------------|--------|
| R0 | Pipeline enforces R1–R6 on every push to `main` and every PR to `main` | Core goal |
| R1 | Rust checks: `cargo fmt --check`, `cargo clippy -- -D warnings`, `SQLX_OFFLINE=true cargo check`, `cargo test` | Must-have |
| R2 | SQLx offline verification: queries compile against `.sqlx/` cache | Must-have |
| R3 | Frontend checks: `bun test`, `bun build` | Must-have |
| R4 | CI must pass before PR can merge to `main` (branch protection) | Must-have |
| R5 | Pipeline is fast enough to not block workflow | Nice-to-have |

---

## A: Single Workflow, Two Parallel Jobs

| Part | Mechanism |
|------|-----------|
| **A1** | Single workflow file `.github/workflows/ci.yml`, triggers on `push: [main]` + `pull_request: [main]` |
| **A2** | **Rust job**: checkout → install Rust toolchain (stable) → cache `~/.cargo` and `target/` → `cargo fmt --check` → `cargo clippy -- -D warnings` → `SQLX_OFFLINE=true cargo check` → `cargo test` |
| **A3** | **Frontend job**: checkout → install Bun → cache `node_modules/` → `bun install` → `bun test` → `bun build app/src/index.html` |
| **A4** | **Branch protection**: document step-by-step instructions to enable "Require status checks to pass" on `main` in GitHub repo settings |

---

## Fit Check: R × A

| Req | Requirement | Status | A |
|-----|-------------|--------|---|
| R0 | Pipeline enforces R1–R6 on every push/PR to `main` | Core goal | ✅ |
| R1 | Rust checks: fmt, clippy, check, test | Must-have | ✅ |
| R2 | SQLx offline verification | Must-have | ✅ |
| R3 | Frontend checks: bun test, bun build | Must-have | ✅ |
| R4 | CI must pass before PR merges (branch protection) | Must-have | ✅ |
| R5 | Pipeline is fast enough | Nice-to-have | ✅ |

**Notes:**
- R5 passes: Rust and Frontend jobs run in parallel on separate runners; caching reduces install time.

---

## A4: Branch Protection Setup

Steps to enable required status checks on `main`:

1. Go to the GitHub repo **Settings** > **Branches**.
2. Click **Add branch protection rule** (or edit the existing rule for `main`).
3. Set **Branch name pattern** to `main`.
4. Check **Require a pull request before merging**.
5. Check **Require status checks to pass before merging**.
6. In the search box, find and add both required checks: **Rust** and **Frontend**.
7. Check **Require branches to be up to date before merging** (ensures the PR branch includes the latest `main` before merging).
8. Click **Create** (or **Save changes**).

After this, PRs targeting `main` cannot merge until both CI jobs pass.

---

## Scope Boundary

**Out of scope:**
- Deployment (CD)
- Docker image building
- Release automation
- Notifications (Slack, email)
