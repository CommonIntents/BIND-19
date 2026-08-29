# CI-144 v2.0 测试向量文档

> **协议版本**：CI-144 v2.0（协议家族架构：PFP-xCF14 + SAP-xCF14）
> **生成日期**：2026-08-29
> **测试向量总数**：33 组（8 个分类）
> **机器可读格式**：[`ci-144-v2.0-test-vectors.json`](./ci-144-v2.0-test-vectors.json)
> **生成器**：[`examples/generate_test_vectors.rs`](../../examples/generate_test_vectors.rs)

---

## 使用说明

本测试向量集供其他兼容 CI-144 v2.0 协议家族的实现验证使用。每个测试向量包含：

- **`input`**：输入参数（人类可读描述）
- **`expected`**：期望输出（编码后的十六进制 / 处理结果）

实现者应验证：将 `input` 经过协议处理后，结果等于 `expected`。

### 快速验证示例（Python）

```python
import json

with open("ci-144-v2.0-test-vectors.json") as f:
    data = json.load(f)

# 验证 PFP 编码
for vec in data["vectors"]["pfp_codec"]:
    encoded = encode_pfp(vec["input"])  # 你的实现
    assert encoded.hex() == vec["expected"]["encoded_hex"], f"Failed: {vec['id']}"

print("All PFP vectors passed!")
```

---

## 测试向量分类总览

| 分类 | 数量 | 内容 |
|---|---|---|
| `pfp_codec` | 5 | PFP-xCF14 4 字节编解码（各种 Modality/Risk/Stance/Edge/标志位组合） |
| `sap_codec` | 5 | SAP-xCF14 28 字节编解码（Seq-Counter 边界值 + PAH-Hash/PAH-Signature） |
| `frame_codec` | 5 | 完整 BIND-19 帧编解码（v1.0兼容 / PFP-only / PFP+SAP / 带payload / 最大payload） |
| `replay_protection` | 5 | 防重放检查（正常递增 / 精确重放 / 旧seq / 大跳跃 / 新源） |
| `rule6_downgrade` | 3 | 规则6降级（Replay-Enable=0 时强制降级至 MEDIUM） |
| `key_rotation` | 4 | 密钥轮换状态机（阈值检测 / 开始轮换 / ACK成功 / 完成轮换） |
| `catastrophic_detection` | 3 | CATASTROPHIC 检测（Risk=Catastrophic + Override=HardOverride） |
| `pah_signature` | 3 | PAH 签名（完整 Ed25519 签名 / 64-bit 截断 / 错误签名拒绝） |

---

## 1. PFP 编解码测试向量（pfp_codec）

PFP-xCF14 是 4 字节物理特征协议（冻结层），结构：

```
Byte 0-1: Family-Magic = 0xCF14（大端序）
Byte 2:   [Modality(2) | Risk-Level(2) | Body-Stance(2) | Proximity-Edge(2)]
Byte 3:   [Output-Dest(1) | Override-Flag(1) | Replay-Enable(1) | Reserved(5)]
```

### pfp-all_zero

- **输入**：Modality=Cognitive, Risk=Low, Stance=Unknown, Edge=Safe, Dest=Internal, Override=Normal, Replay-Enable=false
- **期望编码**：`cf14 00 00`（4 字节）

### pfp-all_one

- **输入**：Modality=SensorFeed, Risk=Catastrophic, Stance=Moving, Edge=CriticalEdge, Dest=External, Override=HardOverride, Replay-Enable=true
- **期望编码**：`cf14 ff 0f`（4 字节）

### pfp-typical_executive

- **输入**：Modality=Executive, Risk=Medium, Stance=Standing, Edge=Warning, Dest=External, Override=Normal, Replay-Enable=true
- **期望编码**：见 JSON 文件

### pfp-cognitive_low

- **输入**：Modality=Cognitive, Risk=Low, Stance=Seated, Edge=Safe, Dest=Internal, Override=Normal, Replay-Enable=true
- **期望编码**：见 JSON 文件

### pfp-render_critical

- **输入**：Modality=Render, Risk=Critical, Stance=Moving, Edge=Danger, Dest=External, Override=Normal, Replay-Enable=true
- **期望编码**：见 JSON 文件

---

## 2. SAP 编解码测试向量（sap_codec）

SAP-xCF14 是 28 字节安全证明协议（演进层），结构：

```
Byte 0-1:   Family-Magic = 0xCF14
Byte 2:     Protocol-ID = 0x01
Byte 3:     [Version(4) | Reserved(4)]
Byte 4-5:   Seq-Counter（16-bit 大端序）
Byte 6-19:  PAH-Hash（14 字节，SHA-256 截断高 112 位）
Byte 20-27: PAH-Signature（8 字节，64-bit 截断签名）
```

### sap-seq_zero

- **输入**：Seq-Counter=0, PAH-Hash=全0, PAH-Signature=全0
- **期望编码**：`cf14 01 10 0000 00...00 00...00`（28 字节）

### sap-seq_max

- **输入**：Seq-Counter=65535, PAH-Hash=全FF, PAH-Signature=全FF
- **期望编码**：见 JSON 文件

### sap-seq_42

- **输入**：Seq-Counter=42, PAH-Hash=0xAB×14, PAH-Signature=0xCD×8
- **期望编码**：见 JSON 文件

### sap-seq_rotation_threshold

- **输入**：Seq-Counter=65534（轮换阈值）, PAH-Hash=0x11×14, PAH-Signature=0x22×8
- **期望编码**：见 JSON 文件

### sap-seq_1000

- **输入**：Seq-Counter=1000, PAH-Hash=0xDE×14, PAH-Signature=0xAD×8
- **期望编码**：见 JSON 文件

---

## 3. 完整帧编解码测试向量（frame_codec）

BIND-19 v2.0 帧结构：

```
[8 字节 BIND-19 头] + [可选 PFP 4字节] + [可选 SAP 28字节] + [可变长 Payload]
```

BIND-19 头部第 6 字节的 flags 字段：
- Bit 0: PFP-Present
- Bit 1: SAP-Present

### frame-v1_compat

- **输入**：无 PFP，无 SAP，无 Payload（v1.0 兼容帧）
- **期望**：总大小 8 字节，flags=0x00

### frame-pfp_only

- **输入**：有 PFP，无 SAP，无 Payload
- **期望**：总大小 12 字节（8+4），flags=0x01

### frame-pfp_sap

- **输入**：有 PFP，有 SAP，无 Payload
- **期望**：总大小 40 字节（8+4+28），flags=0x03

### frame-pfp_sap_payload

- **输入**：有 PFP，有 SAP，64 字节 Payload
- **期望**：总大小 104 字节（8+4+28+64），flags=0x03

### frame-max_payload

- **输入**：有 PFP，有 SAP，1024 字节 Payload
- **期望**：总大小 1064 字节（8+4+28+1024），flags=0x03

---

## 4. 防重放测试向量（replay_protection）

防重放规则：
- 新源（缓存中不存在）→ Allowed（注册该源）
- 已知源，seq > last_seq → Allowed（更新缓存）
- 已知源，seq ≤ last_seq → Rejected（不更新缓存）

预填充状态：tenant_id=1, source_id=100, last_seq=100

### replay-normal_increment

- **输入**：tenant=1, source=100, seq=101
- **期望**：Allowed（101 > 100）

### replay-exact_replay

- **输入**：tenant=1, source=100, seq=100
- **期望**：Rejected（100 ≤ 100）

### replay-old_seq

- **输入**：tenant=1, source=100, seq=50
- **期望**：Rejected（50 ≤ 100）

### replay-large_jump

- **输入**：tenant=1, source=100, seq=65000
- **期望**：Allowed（65000 > 100，大跳跃允许）

### replay-new_source

- **输入**：tenant=1, source=200（新源）, seq=1
- **期望**：Allowed（新源注册，无历史）

---

## 5. 规则6降级测试向量（rule6_downgrade）

规则 6：当 Replay-Enable=0 时，无论原始 Risk-Level 为何，有效风险等级强制为 MEDIUM。

### rule6-catastrophic_replay_disabled

- **输入**：原始 Risk=Catastrophic, Replay-Enable=false
- **期望**：有效 Risk=Medium（降级）

### rule6-critical_replay_disabled

- **输入**：原始 Risk=Critical, Replay-Enable=false
- **期望**：有效 Risk=Medium（降级）

### rule6-low_replay_enabled

- **输入**：原始 Risk=Low, Replay-Enable=true
- **期望**：有效 Risk=Low（不降级，保持原值）

---

## 6. 密钥轮换测试向量（key_rotation）

密钥轮换状态机：Idle → Pending → Rotated → Idle

触发条件：Seq-Counter ≥ 65534（ROTATION_THRESHOLD）

### rotation-threshold-detection

- **输入**：seq_counter=65534
- **期望**：should_rotate=true

### rotation-start-pending

- **输入**：调用 start_rotation(payload)
- **期望**：状态=Pending（retries=0）

### rotation-ack-success

- **输入**：在 Pending 状态下调用 handle_ack()
- **期望**：状态=Rotated

### rotation-complete-idle

- **输入**：在 Rotated 状态下调用 complete_rotation()
- **期望**：状态=Idle（可开始下一次轮换）

---

## 7. CATASTROPHIC 检测测试向量（catastrophic_detection）

CATASTROPHIC 硬覆盖条件：Risk-Level == Catastrophic(3) AND Override-Flag == HardOverride(1)

PFP Byte 2 的位布局：`[Modality(2) | Risk-Level(2) | Body-Stance(2) | Proximity-Edge(2)]`
PFP Byte 3 的位布局：`[Output-Dest(1) | Override-Flag(1) | Replay-Enable(1) | Reserved(5)]`

### catastrophic-catastrophic_override

- **输入**：Risk=Catastrophic, Override=HardOverride
- **期望**：is_catastrophic_override=true

### catastrophic-catastrophic_no_override

- **输入**：Risk=Catastrophic, Override=Normal
- **期望**：is_catastrophic_override=false（缺少 Override 标志）

### catastrophic-critical_override

- **输入**：Risk=Critical, Override=HardOverride
- **期望**：is_catastrophic_override=false（Risk 不是 Catastrophic）

---

## 8. PAH 签名测试向量（pah_signature）

PAH 签名算法：
1. 完整签名：Ed25519（64 字节）
2. 截断签名：SHA-256(完整签名) 的前 8 字节（MSB，高 64 位）

固定测试种子：`0x42 × 32`（可复现）
测试消息：`"CI-144 v2.0 Physical Anchor Layer test message"`

### pah-full-signature

- **输入**：seed=0x42×32, message="CI-144 v2.0..."
- **期望**：完整 Ed25519 签名（64 字节，见 JSON）

### pah-truncated-signature

- **输入**：完整签名（上一个向量的输出）
- **期望**：64-bit 截断签名（8 字节）= SHA-256(full_sig)[0..8]
- **验证**：truncate_signature(full_sig) == sign_truncated(keypair, message)

### pah-wrong-signature-rejected

- **输入**：期望截断签名（正确消息）vs 实际截断签名（错误消息 "wrong message"）
- **期望**：不匹配，应拒绝

---

## 附录：关键常量

| 常量 | 值 | 说明 |
|---|---|---|
| Family-Magic | `0xCF14` | CI-144 协议家族魔数（2 字节大端序） |
| PFP-Size | 4 字节 | PFP-xCF14 物理特征协议（冻结层） |
| SAP-Size | 28 字节 | SAP-xCF14 安全证明协议（演进层） |
| PFP-Protocol-ID | `0x00` | PFP 子协议 ID |
| SAP-Protocol-ID | `0x01` | SAP 子协议 ID |
| ROTATION_THRESHOLD | 65534 | Seq-Counter 触发密钥轮换的阈值 |
| MAX_RETRIES | 3 | 密钥轮换 ACK 最大重试次数 |
| ACK_TIMEOUT | 100ms | 密钥轮换 ACK 超时时间 |
| PAH-Truncation | SHA-256 前 8 字节 | 64-bit 截断签名算法（MSB） |

---

## 验证状态

- ✅ 所有 33 组测试向量由 Rust 参考实现生成
- ✅ JSON 格式验证通过
- ✅ 生成器代码可复现（固定种子）
- ✅ 覆盖协议家族全部 8 个核心模块

---

**文档结束**
