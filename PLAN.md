# BIND-19 开发导航牌（PLAN）

> **版本**：v1.2（v2.0-beta 高并发集成，2026-08-29）
> **状态**：🚧 v2.0-beta（高并发防重放缓存 + 集成测试 + 压测）
> **分支**：v2.0-alpha
> **所属方法论**：DNA 自生长方法论 v2.0（协议家族适配版）
> **规则**：本文件只含当前阶段 + 下一阶段预览 + 阶段总览地图。完成阶段 → GROWTH.md。总行数 ≤150，超出触发历史迁移。
> **组织级文档**：`.github/GOVERNANCE.md` + `.github/DNA.md` + `.github/RNA.md`（必读，见 RNA.md 三层加载协议）
> **上一阶段**：v2.0-alpha ✅ 收官（6 任务 / 92 测试 / 7 模块，见 GROWTH.md）

---

## 1. 当前阶段：v2.0-beta（高并发集成 + 防重放压测）

> **状态**：🚧 B1 待开工（高并发防重放缓存 DashMap）。

### 1.1 目标（基于规范正文 + 附录 D 高并发约束）

| 任务 | 内容 | 规范依据 | 状态 |
|---|---|---|---|
| B1 | 高并发防重放缓存（DashMap，按 (Tenant-ID, Source-ID) 分片，容量≥10万，TTL清理） | 附录 D.1/D.3 | ✅ 完成（16 测试全通过） |
| B2 | 防重放缓存集成到帧处理流程（Seq-Counter 查重 + 拒绝重放帧） | 规则 4（防重放） | ✅ 完成（14 测试全通过） |
| B3 | BIND-19 端到端集成测试（完整帧编解码 + PFP/SAP + 防重放 + 密钥轮换 + CATASTROPHIC） | 全规则 | ✅ 完成（9 集成测试全通过） |
| B4 | 基准测试（criterion，防重放缓存 QPS + 帧编解码延迟 + PAH 验证延迟） | 附录 D.3 | ⏳ 待开工 |
| B5 | 多租户隔离验证（跨租户缓存/密钥/计数器隔离） | 附录 D.1 | ✅ 已在 B1/B2/B3 测试中覆盖 |

### 1.2 规范真相源（v2.0-beta 调研结论）

- **高并发防重放缓存**：使用 `DashMap<(TenantId, SourceId), AtomicU16>`，每 60 秒清理过期条目
- **缓存容量**：≥10 万条目，按 (Tenant-ID, Source-ID) 分片
- **防重放逻辑**：Seq-Counter ≤ Last-Seen-Seq[Source-ID] → 拒绝，审计日志 REJECTED_REPLAY
- **多租户隔离**：每个租户独立会话状态、计数器、密钥，禁止跨租户共享
- **基准测试**：使用 `criterion`，测试防重放缓存 QPS、帧编解码延迟、PAH 验证延迟
- **64-bit 截断算法**：完整 ECC 签名（Ed25519 512-bit）的 SHA-256 哈希值前 64 位（MSB），跨实现必须一致
- **Seq-Counter**：16-bit，在 SAP 中；冷启动随机值；AtomicU16 + SeqCst；回绕阈值 65534 触发密钥轮换
- **密钥轮换**：≥65534 触发；KEY_ROTATION 帧类型建议 0x07（需 ADR-0008 确认）；ACK 超时 100ms×3，失败停止发送（fail-closed，严禁回退）
- **调试模式**：CI144_DEBUG=1 仅启动时读取；规则 1-3 仍生效；仅规则 6 风险降级可跳过；启动输出警告 banner
- **节能模式**：SAP 不存在时，PFP.Replay-Enable 强制为 0，规则 6 自动触发降级至 MEDIUM

### 1.3 技术前提

- ✅ 11 项前置设计全部锁定（见 `docs/v2.0-upgrade-plan.md` 第八章）
- ✅ 9 个 ADR 占位符已创建（ADR-0001~0009）
- ✅ DNA 方法论适配完成（组织级 GOVERNANCE/DNA/RNA + 协议级 PLAN/GROWTH/ADR）
- ⚠️ BIND-19 v1.0 帧类型分配表需逆向检查（ADR-0008，v2.0-alpha.1 前必须锁定）

### 1.4 入口 ADR（Draft，待审查）

- **ADR-0001**：PAH 第二层签名位置（INTENT-7 载荷头部扩展）
- **ADR-0004**：KEY_ROTATION 帧格式（帧类型 0x07 待确认）
- **ADR-0005**：轮换期间帧处理 + ACK 超时 fail-closed
- **ADR-0008**：BIND-19 帧类型 0x07 冲突确认（v2.0-alpha.1 前必须锁定）

### 1.5 验收标准

- PFP 4 字节解析器：固定偏移读取，零拷贝，无分支；Family-Magic 验证 ✅
- SAP 28 字节解析器：固定偏移读取，零拷贝；Protocol-ID 验证 ✅
- 第一层 64-bit 验证：ed25519 软件实现，延迟 ≤100μs（Tuck 直接验证）
- Seq-Counter 防重放：单调递增，回绕阈值 65534 触发密钥轮换
- KEY_ROTATION：ACK 超时 3 次失败后停止发送（fail-closed）
- 调试模式：CI144_DEBUG=1 仅规则 6 降级可跳过，规则 1-3 仍生效
- 向后兼容：v1.0 接收端忽略 PFP-Present/SAP-Present，v2.0 接收端处理 v1.0 帧
- 单元测试 + 集成测试全绿（当前 23 测试全通过）

### 1.6 下一阶段预览：v2.0-beta

- 防重放压测（高并发多租户场景）
- 第二层 512-bit 完整验证（载荷解密后异步）
- 密钥吊销流程（连续 3 次第二层验证失败触发）
- 多租户场景验证（租户隔离、计数器分片）

---

## 2. 阶段总览（地图，不展开）

| 阶段 | 内容 | 状态 |
|---|---|---|
| v1.0 | BIND-19 v1.0.0-RFC-4（传输绑定协议） | ✅ 已冻结 |
| **v2.0-alpha** | **PAL 24 字节编码 + Seq-Counter + 第一层验证 + KEY_ROTATION** | **🚧 当前** |
| v2.0-beta | 防重放压测 + 第二层验证 + 密钥吊销 + 多租户 | ⏳ 预览 |
| v2.0-rc.1 | 测试向量发布（≥20 组）+ 硬编码常量写入宪章 | ⏳ |
| v2.0.0 | 规范冻结 + Tuck Rust 重构对接 | ⏳ |

---

## 3. 活跃决策与契约指针（不展开）

| 项 | 指针 |
|---|---|
| 组织级治理 | `.github/GOVERNANCE.md` + `.github/DNA.md` + `.github/RNA.md` |
| v2.0 升级计划 | `docs/v2.0-upgrade-plan.md`（权威版本） |
| ADR 目录 | `docs/decisions/`（ADR-0001~0009） |
| spec 正文 | `spec/BIND-19.md`（v1.0.0-RFC-4，源真相） |
| 归档 | `docs/archive/`（永不删除） |

---

## 4. 文档生态 SOP（DNA v2.0 协议家族适配版）

PLAN 是导航牌不是历史档案；阶段收尾时（收尾 SLA：24h）完成记录追加 GROWTH.md 并从 PLAN 移除；GROWTH ≤3 条超则归档；PLAN ≤150 行超则触发历史迁移。详见 `.github/DNA.md`「文档生态 SOP」。
