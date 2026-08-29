# BIND-19 开发导航牌（PLAN）

> **版本**：v1.0（v2.0-alpha 启动，2026-08-29）
> **状态**：🚧 v2.0-alpha（物理锚定层编码）
> **分支**：v2.0-alpha
> **所属方法论**：DNA 自生长方法论 v2.0（协议家族适配版）
> **规则**：本文件只含当前阶段 + 下一阶段预览 + 阶段总览地图。完成阶段 → GROWTH.md。总行数 ≤150，超出触发历史迁移。
> **组织级文档**：`.github/GOVERNANCE.md` + `.github/DNA.md` + `.github/RNA.md`（必读，见 RNA.md 三层加载协议）

---

## 1. 当前阶段：v2.0-alpha（物理锚定层编码）

> **状态**：🚧 前置设计锁定完成（2026-08-29），待编码开工（DNA 方法论：计划先于代码）。

### 1.1 目标（基于规范正文调研）

| 任务 | 内容 | 规范依据 | 状态 |
|---|---|---|---|
| T1 | PAL 24 字节固定偏移结构编码 + 解析器 | `docs/v2.0-upgrade-plan.md` 第四章 | ⏳ ADR-0001 |
| T2 | BIND-19 帧结构升级（PAL-Present + 16-bit Seq-Counter） | 第五章 | ⏳ ADR-0004 |
| T3 | PAH 第一层 64-bit 验证（ed25519 软件实现，SHA-256 前 64 位截断） | 规则 5 | ⏳ |
| T4 | Replay-Enable=0 强制降级（规则 6）+ 调试模式例外（CI144_DEBUG=1） | 规则 6 | ⏳ |
| T5 | KEY_ROTATION 控制帧实现 + ACK 超时 fail-closed | 规则 7 | ⏳ ADR-0005 |
| T6 | CATASTROPHIC 硬覆盖（规则 1-3）+ 事件驱动（无轮询） | 规则 1-3 | ⏳ |

### 1.2 规范真相源（v2.0-alpha 调研结论）

- **PAL 结构**：24 字节固定偏移，192 bits = 3×64-bit 对齐 SIMD；字段定义见 `docs/v2.0-upgrade-plan.md` 第四章
- **64-bit 截断算法**：完整 ECC 签名（Ed25519 512-bit）的 SHA-256 哈希值前 64 位（MSB），跨实现必须一致
- **Seq-Counter**：16-bit，位置复用 BIND-19 v1.0 第 6-7 字节（原 Reserved）；冷启动随机值；AtomicU16 + SeqCst
- **密钥轮换**：≥65534 触发；KEY_ROTATION 帧类型建议 0x07（需 ADR-0008 确认）；ACK 超时 100ms×3，失败停止发送（fail-closed，严禁回退）
- **调试模式**：CI144_DEBUG=1 仅启动时读取；规则 1-3 仍生效；仅规则 6 风险降级可跳过；启动输出警告 banner

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

- PAL 24 字节解析器：固定偏移读取，零拷贝，无分支
- 第一层 64-bit 验证：ed25519 软件实现，延迟 ≤100μs（Tuck 直接验证）
- Seq-Counter 防重放：单调递增，回绕触发密钥轮换
- KEY_ROTATION：ACK 超时 3 次失败后停止发送（fail-closed）
- 调试模式：CI144_DEBUG=1 仅规则 6 降级可跳过，规则 1-3 仍生效
- 向后兼容：v1.0 接收端忽略 PAL-Present，v2.0 接收端处理 v1.0 帧
- 单元测试 + 集成测试全绿

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
