# SAP-xCF14 — Security Attestation Protocol Specification

> **版本**：v1.0（演进层）
> **日期**：2026-08-29
> **状态**：Active（可演进，v1/v2 可并行存在）
> **所属协议家族**：CI-144 Protocol Family
> **家族魔数**：`0xCF14`
> **子协议 ID**：`0x01`
> **总长度**：28 字节（224 bits）
> **权威来源**：[CommonIntents/BIND-19/docs/spec/sap-xcf14.md](https://github.com/CommonIntents/BIND-19/blob/v2.0-rc.1/docs/spec/sap-xcf14.md)
> **对应 BIND-19 版本**：[v2.0-rc.1](https://github.com/CommonIntents/BIND-19/tree/v2.0-rc.1)
> **发布窗口**：[CommonIntents/SAP-xCF14](https://github.com/CommonIntents/SAP-xCF14)

---

## 1. 协议概述

SAP-xCF14（Security Attestation Protocol）是 CI-144 协议家族的**安全证明协议**，属于**演进层**。它为 PFP 描述的物理特征提供安全证明，包括防重放序列号和物理上下文哈希签名。

### 1.1 设计哲学

| 原则 | 体现 |
|---|---|
| **极致解耦** | 与 PFP 分离，SAP 可选加载，不影响 PFP 的硬实时决策 |
| **按需加载** | 低安全场景可跳过 SAP（仅发送 PFP），节能模式 |
| **极致复用** | 复用 Ed25519 签名 + SHA-256 哈希，不重新发明密码学原语 |
| **白盒可观测** | 所有字段明文可见，Tuck 可在不解密载荷的情况下验证 |
| **渐进生长** | 演进层，v1/v2 可并行，未来可扩展新的安全证明机制 |

### 1.2 在协议栈中的位置

```
[ 8-byte BIND-19 Header ] + [ PFP 4 bytes ] + [ SAP 28 bytes (optional) ] + [ Payload ]
```

SAP 依赖 PFP 存在（SAP 不能单独出现）。Tuck 硬实时决策仅依赖 PFP，SAP 是可选的安全增强层。

---

## 2. 字节布局（28 字节 / 224 bits）

所有字段均为**明文、固定偏移、固定长度**。

| 字节 | 位偏移 | 长度(bit) | 字段名 | 说明 |
|---|---|---|---|---|
| 0-1 | 0-15 | 16 | `Family-Magic` | 固定 `0xCF14`（大端序） |
| 2 | 0-7 | 8 | `Protocol-ID` | 固定 `0x01`（SAP-xCF14 子协议标识） |
| 3 | 0-3 | 4 | `Version` | SAP 版本，当前 v1.0 = `0001` |
| 3 | 4-7 | 4 | `Reserved` | 保留，强制为 0 |
| 4-5 | 0-15 | 16 | `Seq-Counter` | 防重放序列号（大端序，单调递增） |
| 6-19 | 0-111 | 112 | `PAH-Hash` | 物理上下文哈希（SHA-256 截断高 112 位） |
| 20-27 | 0-63 | 64 | `PAH-Signature` | PAH 签名（64-bit 截断，快速校验层） |

---

## 3. 字段定义

### 3.1 Family-Magic（家族魔数）

- **偏移**：字节 0-1，16 bits
- **固定值**：`0xCF14`（大端序）
- **用途**：识别 CI-144 协议家族，与 PFP 共享同一魔数。

### 3.2 Protocol-ID（子协议 ID）

- **偏移**：字节 2，8 bits
- **固定值**：`0x01`（SAP-xCF14）
- **子协议 ID 分配表**：

| ID | 协议 | 状态 |
|---|---|---|
| 0x00 | PFP-xCF14 | 冻结 |
| 0x01 | SAP-xCF14 | 当前（v1.0） |
| 0x02-0x7F | 预留 | 未来扩展 |
| 0x80-0xFF | 实验/私有 | 用户自定义 |

### 3.3 Version（版本号）

- **偏移**：字节 3，位 0-3，4 bits
- **当前值**：`0001`（v1.0）
- **支持最多 16 个版本**，v1/v2 可并行存在（演进层）。

### 3.4 Reserved（保留位）

- **偏移**：字节 3，位 4-7，4 bits
- **强制值**：全 0
- **治理规则**：同 PFP，未来使用需提交 ADR。

### 3.5 Seq-Counter（防重放序列号）

- **偏移**：字节 4-5，16 bits（大端序）
- **用途**：防重放攻击。发送端单调递增，接收端缓存并查重。
- **初始化**：每次冷启动时取随机值，避免重启后序列号重置被攻击者利用。
- **原子递增**：多线程环境下必须使用 `AtomicU16` + `fetch_add(1, Ordering::SeqCst)`。
- **回绕策略**：16-bit 空间（65535），回绕阈值 65534 触发密钥轮换（见第 5 节）。

#### 防重放检查规则

```
IF PFP.Replay-Enable == 1:
    IF Seq-Counter > Last-Seen-Seq[Source-ID]:
        → Allowed（更新缓存）
    ELSE:
        → Rejected（审计日志 REJECTED_REPLAY）
ELSE (Replay-Enable == 0):
    → 跳过检查（规则 6 降级，有效风险强制 MEDIUM）
```

#### 缓存要求

- 按 `(Tenant-ID, Source-ID)` 分片
- 至少缓存 1024 个源，每源保留最后 256 个序列号
- TTL 自动清理（默认 60 秒）

### 3.6 PAH-Hash（物理上下文哈希）

- **偏移**：字节 6-19，112 bits（14 字节）
- **算法**：SHA-256（物理上下文数据）的高 112 位（MSB）
- **物理上下文数据**：传感器读数（姿态/临边/模态/时间戳/设备 ID 等）的序列化字节
- **用途**：锁定物理上下文，防止 AI 伪造物理特征。PAH 由传感器数据哈希锁定，AI 无权修改。

### 3.7 PAH-Signature（PAH 签名，快速校验层）

- **偏移**：字节 20-27，64 bits（8 字节）
- **算法**：完整 Ed25519 签名（512-bit）的 SHA-256 哈希值前 64 位（MSB）
- **用途**：Tuck 硬实时快速校验（亚毫秒级），提供基础防伪，抵抗偶发噪声或简单篡改。
- **双层安全架构**：

| 层级 | 签名长度 | 验证时机 | 验证者 | 失败处理 |
|---|---|---|---|---|
| 第一层（快速校验） | 64-bit 截断 | 硬实时决策前 | Tuck | 立即拒绝帧 + ERROR 电平 + 声光告警 |
| 第二层（完整验证） | 512-bit 完整 | 载荷解密后异步 | Anaphase / 云端审计 | 不拒绝帧，触发异步告警，连续 3 次失败触发密钥吊销 |

#### 截断算法（必须跨实现一致）

```
truncated_signature = SHA-256(full_ed25519_signature)[0:8]  // 前 8 字节（MSB，高 64 位）
```

---

## 4. 编码示例

### 4.1 典型 SAP 帧（Seq=42, PAH-Hash=0xAB×14, PAH-Signature=0xCD×8）

```
字段值:
  Family-Magic   = 0xCF14
  Protocol-ID    = 0x01
  Version        = 0x1 (0001)
  Reserved       = 0x0
  Seq-Counter    = 42 (0x002A, 大端序)
  PAH-Hash       = 0xAB × 14
  PAH-Signature  = 0xCD × 8

字节 3 = (Reserved << 4) | Version = (0 << 4) | 1 = 0x10

编码结果:
  CF 14 01 10 00 2A AB AB AB AB AB AB AB AB AB AB AB AB AB CD CD CD CD CD CD CD CD
  └─魔数─┘ └ID┘ └Ver┘ └Seq─┘ └──── PAH-Hash (14字节) ────┘ └─PAH-Sig (8字节)─┘

总长度: 28 字节
```

---

## 5. 密钥轮换机制（Seq-Counter 回绕处理）

### 5.1 触发条件

发送端检测到 `Seq-Counter >= 65534`（ROTATION_THRESHOLD）时，必须在下一帧触发密钥轮换。

### 5.2 轮换流程

1. 发送端生成新的会话加密密钥（AES-GCM 新 Key）
2. 发送端发送 `KEY_ROTATION` 控制帧（BIND-19 FrameType=0x07），携带新密钥（由主密钥加密保护）
3. 接收端（Tuck）确认新密钥后，重置该源 ID 的 Last-Seen-Seq 缓存
4. 若接收端未收到轮换帧而检测到序列号回绕（突然从 65535 跳到 0），直接视为异常重放攻击，立即拒绝所有帧并触发硬件告警，直到人工复位

### 5.3 ACK 超时处理

- 发送轮换帧后，等待接收端 ACK（FrameType=0x08），超时时间 100ms
- 重试最多 3 次
- 若 3 次全部失败，发送端必须：
  1. 触发硬告警（ERROR 电平 + 声光指示）
  2. 停止发送所有数据帧，进入"密钥轮换失败"安全状态
  3. 等待人工物理复位或带外管理干预后方可恢复
- **严禁回退至旧密钥并继续发送数据帧**（fail-closed，状态不一致比停止服务更危险）

---

## 6. 规则 6：Replay-Enable=0 安全约束

当 PFP 中 `Replay-Enable == 0` 时，Tuck 必须强制执行以下约束：

1. **风险降级**：无论 PFP.Risk-Level 原始值为何，有效风险等级强制视为 `MEDIUM`。永远无法触发 CATASTROPHIC 硬覆盖，从根本上杜绝利用重放发动高危物理攻击。
2. **强化验证**：必须强制通过 PAH-Signature 验证（第一层 64-bit）。用防伪强度弥补防重放缺失，若签名验证失败，直接拒绝帧。
3. **审计强制标记**：审计日志必须显式记录 `REPLAY_DISABLED` 事件，且该条日志必须附带当前硬件时钟。

### 伪代码

```rust
if pfp.replay_enable() == 0 {
    // 强制降级，不可绕过
    let effective_risk = RiskLevel::Medium;
    if !verify_pah_signature_64(&sap) {
        return Decision::Reject;
    }
    write_audit("REPLAY_DISABLED", effective_risk);
    // 后续决策仅使用 effective_risk，丢弃原始高优先级
}
```

---

## 7. 测试向量

完整测试向量见 [CI-144 v2.0 测试向量集](../../tests/test_vectors/ci-144-v2.0-test-vectors.json)，其中 SAP 相关分类包括：

| 分类 | 数量 | 内容 |
|---|---|---|
| `sap_codec` | 5 | SAP 28 字节编解码（Seq-Counter 边界值 0/42/1000/65534/65535） |
| `replay_protection` | 5 | 防重放检查（正常递增/精确重放/旧seq/大跳跃/新源） |
| `rule6_downgrade` | 3 | 规则6降级（Replay-Enable=0 → MEDIUM） |
| `key_rotation` | 4 | 密钥轮换状态机（阈值/开始/ACK/完成） |
| `pah_signature` | 3 | PAH 签名（完整Ed25519/64-bit截断/错误签名拒绝） |

---

## 8. 与 PFP 的关系

| 维度 | PFP-xCF14 | SAP-xCF14 |
|---|---|---|
| 层级 | 冻结层 | 演进层 |
| 长度 | 4 字节 | 28 字节 |
| 依赖 | 无（独立存在） | 依赖 PFP（不能单独存在） |
| Tuck 读取 | 必须（硬实时决策） | 不依赖（可选验证） |
| 变化频率 | 永不变化（冻结） | 可演进（v1/v2 可并行） |
| 核心价值 | 物理特征描述 | 安全证明（防重放+签名） |

---

## 9. 版本历史

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-29 | 初始版本。28 字节结构，Seq-Counter 防重放 + PAH-Hash + PAH-Signature 双层安全架构 + 密钥轮换机制。 |

---

## 10. 权威来源声明

> **本规范的唯一权威来源在 [CommonIntents/BIND-19/docs/spec/sap-xcf14.md](https://github.com/CommonIntents/BIND-19/blob/v2.0-rc.1/docs/spec/sap-xcf14.md)。**
>
> [CommonIntents/SAP-xCF14](https://github.com/CommonIntents/SAP-xCF14) 仓库是本规范的**发布窗口**，内容从 BIND-19 同步而来。所有规范变更的 PR 必须提在 BIND-19 仓库，同时更新 `docs/spec/` + `src/` + `tests/`。
>
> 本规范对应 BIND-19 版本：**[v2.0-rc.1](https://github.com/CommonIntents/BIND-19/tree/v2.0-rc.1)**

---

**文档结束**
