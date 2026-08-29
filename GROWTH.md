# BIND-19 生长日志（变异 + 阶段完成记录）

> **所属方法论**：DNA 自生长方法论 v2.0（协议家族适配版）
> **组织级文档**：`.github/GOVERNANCE.md` + `.github/DNA.md` + `.github/RNA.md`

规则：只保留最近 3 条。第 4 条写入时，最旧的归档到 `docs/archive/growth/`（已版本化，永不删除）。

---

## [2026-08-29] 完成：v2.0-alpha 启动 — DNA 方法论适配 + 文档迁移 + 前置设计锁定

### 触发条件
CI-144 v2.0 升级计划通过完整审查（11 项设计锁定 + 3 处安全降级驳回 + 9 个 ADR 占位符），DNA 方法论适配完成（组织级 GOVERNANCE/DNA/RNA + 协议级 PLAN/GROWTH/ADR），文档从 helix-mind 迁移至 BIND-19 权威仓库。

### 变更性质
- **组织级 DNA 方法论适配**：
  - `.github/DNA.md` v1.0：CI-144 协议家族不可变原则（6 条公理：spec 源真相、Append-Only、决策先于变更、向后兼容、物理事实优先、极致节能）
  - `.github/RNA.md` v1.0：AI 协作铁律（8 条）+ 三层加载协议（组织级 → 协议级 → 规范级）
  - `.github/GOVERNANCE.md` v1.1：引用 DNA.md 和 RNA.md，明确方法论文档体系
- **协议级文档迁移**：
  - `docs/v2.0-upgrade-plan.md`：CI-144 v2.0 升级计划（权威版本，从 helix-mind 迁移）
  - `docs/decisions/ADR-0001~0009`：9 个 ADR 占位符（重新编号，从 0001 开始）
  - `PLAN.md` v1.0：v2.0-alpha 阶段导航
  - `docs/archive/growth/`：归档目录
- **安全闭环确认**（3 处安全降级驳回）：
  1. Seq-Counter 保持 16-bit + 密钥轮换；32-bit 降级为 ADR-0009（Beta 评估）
  2. PAH 第一层 64-bit 验证必须实现（软件 ed25519，不可跳过）；截断算法 = SHA-256 前 64 位
  3. Replay-Enable=0 恢复强制降级至 MEDIUM（补丁 A）；调试模式例外（CI144_DEBUG=1，规则 1-3 仍生效）

### 兼容性
- v1.0 规范正文（`spec/BIND-19.md`）零变更
- v2.0 向后兼容：BIND-19 新增 PAL-Present 标志位，v1.0 接收端忽略
- 纯文档阶段，零代码变更

### 验收
- 组织级 DNA/RNA/GOVERNANCE 完整 ✅
- 协议级 PLAN/GROWTH/ADR 完整 ✅
- 9 个 ADR 占位符创建（ADR-0001~0009）✅
- v2.0 升级计划迁移至权威仓库 ✅
- 11 项前置设计全部锁定 ✅
- 3 处安全降级驳回，恢复安全设计 ✅

### 状态
🧬 v2.0-alpha 前置锁定完成，待编码开工

---

## [2026-08-28] 完成：BIND-19 v1.0.0-RFC-4 冻结

### 触发条件
BIND-19 传输绑定协议通过 RFC 审查，v1.0.0-RFC-4 正式冻结。

### 变更性质
- `spec/BIND-19.md`：传输格式协商（binary/JSON）、完整性检查（CRC32）、版本绑定声明
- `spec/BIND-19.zh-CN.md`：中文版
- 协议栈定位：INTENT-7（语义）→ BIND-19（传输绑定）→ INTENT-7-SECURE（mTLS）→ CAPABILITY-13（共识）

### 兼容性
- v1.0 首次发布，无历史兼容问题

### 验收
- spec 正文冻结 ✅
- 中英文版本对齐 ✅
- README/SECURITY/CONTRIBUTING 完整 ✅

### 状态
✅ 已冻结
