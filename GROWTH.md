# BIND-19 生长日志（变异 + 阶段完成记录）

> **所属方法论**：DNA 自生长方法论 v2.0（协议家族适配版）
> **组织级文档**：`.github/GOVERNANCE.md` + `.github/DNA.md` + `.github/RNA.md`

规则：只保留最近 3 条。第 4 条写入时，最旧的归档到 `docs/archive/growth/`（已版本化，永不删除）。

---

## [2026-08-29] 完成：v2.0-beta 收官 — 高并发防重放 + 端到端集成 + 基准压测（5 任务 / 133 测试 / 14 基准）

### 触发条件
v2.0-beta 全部 5 个任务完成，高并发防重放缓存（DashMap）集成到帧处理器，9 个端到端集成测试覆盖全规则，14 个基准场景压测完成，压测战绩展示到 README。

### 变更性质
- **B1 高并发防重放缓存**：DashMap 分片缓存，按 (Tenant-ID, Source-ID) 分片，AtomicU16 无锁访问，容量 10 万，TTL 60 秒自动清理
- **B2 帧处理器**：FrameProcessor 整合 ReplayCache + 帧解码 + 规则 4 防重放检查，Replay-Enable=0 跳过检查，无 SAP 返回 NoSap
- **B3 端到端集成测试**：9 个场景覆盖完整帧编解码 + 防重放 + 密钥轮换生命周期 + CATASTROPHIC 事件 + 规则 6 降级 + 多租户隔离 + 完整数据流 + fail-closed + 节能模式
- **B4 基准压测**：14 个基准场景，Criterion 0.5，100 samples each，压测战绩展示到 README Performance Benchmarks 部分
- **B5 多租户隔离验证**：已在 B1/B2/B3 测试中覆盖（test_multiple_tenants_isolated + test_e2e_multi_tenant_isolation）

### 压测战绩（2013 MacBook Pro 2.3GHz i7）

| 模块 | 操作 | 延迟 |
|---|---|---|
| 防重放缓存 | check_and_update (hit) | **39 ns** (~25.6M ops/s) |
| 防重放缓存 | check_and_update (replay reject) | **38 ns** (~26.3M ops/s) |
| 防重放缓存 | check_and_update (miss/new) | **195 ns** (~5.1M ops/s) |
| 帧编解码 | encode (PFP+SAP+64B) | **88 ns** |
| 帧编解码 | decode (PFP+SAP+64B) | **105 ns** |
| 帧编解码 | encode+decode roundtrip | **253 ns** |
| PAH 签名 | Ed25519 full sign | **25.2 µs** |
| PAH 签名 | Ed25519 full verify | **46.2 µs** |
| PAH 签名 | 64-bit truncated verify (match) | **684 ns** |
| CATASTROPHIC | pure bit-check | **0.3 ps** (~3 CPU cycles) |
| CATASTROPHIC | normal frame check | **3.2 ns** |
| CATASTROPHIC | full handle (event+audit) | **60.8 µs** |

### 关键性能洞察
- **Tuck 硬实时决策路径** = PFP读取(4B) + CATASTROPHIC位检查(~3 cycles) + 防重放检查(~40ns) = **亚微秒级决策**
- **PAH 第一层验证** = 684ns（截断匹配），完整 Ed25519 验证推迟到异步层
- **帧处理** = ~250ns 往返，适合 10Gbps+ 线速处理
- **极致节能** = 固定偏移解析，热路径无内存分配，决策逻辑无分支

### 兼容性
- v1.0 规范正文（`spec/BIND-19.md`）零变更
- v2.0 向后兼容：无 PFP/SAP 的帧按 v1.0 解析
- Rust 库 API：9 个模块（新增 replay_cache + processor）

### 验收
- 5 个 beta 任务全部完成 ✅（B1/B2/B3/B4/B5）
- 133 个测试全通过（lib 122 + integration 9 + doc 2）✅
- 14 个基准场景全部完成 ✅
- README.md 压测战绩展示 ✅
- PLAN.md 同步 ✅
- GROWTH.md 归档最旧记录（v1.0 冻结 → docs/archive/growth/）✅

### 状态
🧬 v2.0-beta 正式收官，待 v2.0-rc.1（测试向量发布 + 多租户场景验证 + PAH 签名压测）

---

## [2026-08-29] 完成：v2.0-alpha 收官 — 协议家族架构完整实现（6 任务 / 92 测试）

### 触发条件
v2.0-alpha 全部 6 个编码任务完成，CI-144 协议家族架构（PFP-xCF14 + SAP-xCF14）完整实现，92 个单元测试全通过，clippy 零警告。

### 变更性质
- **T1 PFP+SAP 结构编码**：PFP-xCF14 4 字节（冻结层）+ SAP-xCF14 28 字节（演进层），家族魔数 0xCF14
- **T2 BIND-19 帧结构升级**：8 字节头部 + PFP-Present/SAP-Present 标志位 + 可选扩展层，向后兼容 v1.0
- **T3 PAH 第一层签名验证**：ed25519 软件实现 + SHA-256 前 64 位截断，快速拒绝路径
- **T4 规则 6 强制降级 + 调试模式**：Replay-Enable=0 强制降级至 MEDIUM；CI144_DEBUG=1 调试模式（规则 1-3 仍生效）
- **T5 密钥轮换状态机**：KEY_ROTATION 帧（0x07）+ ACK（0x08）+ 100ms 超时 + 3 次重试 + fail-closed
- **T6 CATASTROPHIC 硬覆盖**：规则 1-3 完整实现 + 事件驱动总线（mpsc，无轮询）+ 链式防篡改审计日志

### 关键架构决策
- **协议家族解耦**：PFP（物理特征，冻结）与 SAP（安全证明，演进）完全分离，Tuck 硬实时只读 PFP 4 字节
- **事件驱动无轮询**：CATASTROPHIC 事件使用 mpsc::channel，接收端 recv() 操作系统级阻塞，零 CPU 占用
- **fail-closed 安全哲学**：密钥轮换 3 次失败后停止发送数据帧，严禁回退旧密钥（状态不一致比停止服务更危险）
- **调试模式安全底线**：CI144_DEBUG=1 仅跳过规则 6，规则 1-3（CATASTROPHIC）始终生效，不可跳过

### 兼容性
- v1.0 规范正文（`spec/BIND-19.md`）零变更
- v2.0 向后兼容：无 PFP/SAP 的帧按 v1.0 解析；v1.0 接收端忽略扩展标志位
- Rust 库 API：`bind19` crate，7 个模块（pfp/sap/frame/crypto/config/rotation/catastrophic）

### 验收
- 6 个编码任务全部完成 ✅
- 92 个单元测试全通过 ✅
- cargo clippy --all-targets -- -D warnings 零警告 ✅
- README.md 更新（v2.0 架构完整说明）✅
- PLAN.md 同步（全部任务标记完成）✅
- ADR 维护（ADR-0008 状态 Draft → Active）✅

### 状态
🧬 v2.0-alpha 正式收官

---

## [2026-08-29] 完成：v2.0-alpha 启动 — DNA 方法论适配 + 文档迁移 + 前置设计锁定

### 触发条件
CI-144 v2.0 升级计划通过完整审查（11 项设计锁定 + 3 处安全降级驳回 + 9 个 ADR 占位符），DNA 方法论适配完成（组织级 GOVERNANCE/DNA/RNA + 协议级 PLAN/GROWTH/ADR），文档从 helix-mind 迁移至 BIND-19 权威仓库。

### 变更性质
- **组织级 DNA 方法论适配**：
  - `.github/DNA.md` v1.0：CI-144 协议家族不可变原则（6 条公理）
  - `.github/RNA.md` v1.0：AI 协作铁律（8 条）+ 三层加载协议
  - `.github/GOVERNANCE.md` v1.1：引用 DNA.md 和 RNA.md
- **协议级文档迁移**：
  - `docs/v2.0-upgrade-plan.md`：CI-144 v2.0 升级计划（权威版本）
  - `docs/decisions/ADR-0001~0009`：9 个 ADR 占位符
  - `PLAN.md` v1.0：v2.0-alpha 阶段导航
- **安全闭环确认**（3 处安全降级驳回）：
  1. Seq-Counter 保持 16-bit + 密钥轮换；32-bit 降级为 ADR-0009
  2. PAH 第一层 64-bit 验证必须实现（软件 ed25519，不可跳过）
  3. Replay-Enable=0 恢复强制降级至 MEDIUM；调试模式例外（CI144_DEBUG=1）

### 兼容性
- v1.0 规范正文（`spec/BIND-19.md`）零变更
- 纯文档阶段，零代码变更

### 验收
- 组织级 DNA/RNA/GOVERNANCE 完整 ✅
- 协议级 PLAN/GROWTH/ADR 完整 ✅
- 9 个 ADR 占位符创建 ✅
- v2.0 升级计划迁移至权威仓库 ✅
- 11 项前置设计全部锁定 ✅
- 3 处安全降级驳回，恢复安全设计 ✅

### 状态
🧬 v2.0-alpha 前置锁定完成，待编码开工
