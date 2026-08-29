# PFP-xCF14 — Physical Feature Protocol Specification

> **版本**：v1.0（冻结层）
> **日期**：2026-08-29
> **状态**：Frozen（一旦定稿，永不变化。任何修改必须产生新版本，如 PFP-xCF15）
> **所属协议家族**：CI-144 Protocol Family
> **家族魔数**：`0xCF14`
> **子协议 ID**：`0x00`
> **总长度**：4 字节（32 bits）
> **权威来源**：[CommonIntents/BIND-19/docs/spec/pfp-xcf14.md](https://github.com/CommonIntents/BIND-19/blob/v2.0-rc.1/docs/spec/pfp-xcf14.md)
> **对应 BIND-19 版本**：[v2.0-rc.1](https://github.com/CommonIntents/BIND-19/tree/v2.0-rc.1)
> **发布窗口**：[CommonIntents/PFP-xCF14](https://github.com/CommonIntents/PFP-xCF14)

---

## 1. 协议概述

PFP-xCF14（Physical Feature Protocol）是 CI-144 协议家族的**物理特征协议**，属于**冻结层**。它定义了数字生命体在物理世界中的姿态、风险、环境等可被硬实时读取的物理特征元数据。

### 1.1 设计哲学

| 原则 | 体现 |
|---|---|
| **极致节能** | 仅 4 字节，Tuck 硬实时决策只读 PFP，不解密载荷 |
| **确定性优先** | 固定偏移、固定长度、固定枚举，无分支判断 |
| **物理事实优先** | 所有字段由传感器驱动生成，AI 无权修改 |
| **极致解耦** | 与安全证明（SAP）分离，PFP 不依赖任何加密机制 |
| **冻结不朽** | 一旦定稿永不变化，像石头一样不朽 |

### 1.2 在协议栈中的位置

```
[ 8-byte BIND-19 Header ] + [ PFP 4 bytes (optional) ] + [ SAP 28 bytes (optional) ] + [ Payload ]
```

PFP 插入在 BIND-19 传输头之上、SAP 安全证明层之下。Tuck 等硬实时闸门仅读取 PFP，不解析 SAP 或载荷。

---

## 2. 字节布局（4 字节 / 32 bits）

所有字段均为**明文、固定偏移、固定长度**。即使整个帧被加密，PFP 也在传输层可见。

| 字节 | 位偏移 | 长度(bit) | 字段名 | 枚举值 / 说明 |
|---|---|---|---|---|
| 0-1 | 0-15 | 16 | `Family-Magic` | 固定 `0xCF14`（大端序），CI-144 协议家族魔数 |
| 2 | 0-1 | 2 | `Modality` | 操作模态 |
| 2 | 2-3 | 2 | `Risk-Level` | 风险等级 |
| 2 | 4-5 | 2 | `Body-Stance` | 本体姿态 |
| 2 | 6-7 | 2 | `Proximity-Edge` | 临边/高危环境 |
| 3 | 0 | 1 | `Output-Dest` | 输出目的地 |
| 3 | 1 | 1 | `Override-Flag` | 硬覆盖标志 |
| 3 | 2 | 1 | `Replay-Enable` | 防重放使能 |
| 3 | 3-7 | 5 | `Reserved` | 保留，强制为 0 |

### 2.1 字节 2 位布局详图

```
Bit:   7  6  5  4  3  2  1  0
     [ Proximity-Edge ][ Body-Stance ][ Risk-Level ][ Modality ]
       (2 bits)          (2 bits)        (2 bits)      (2 bits)
```

### 2.2 字节 3 位布局详图

```
Bit:   7  6  5  4  3  2        1             0
     [        Reserved        ][ Replay- ][ Override- ][ Output- ]
       (5 bits, 强制为0)         Enable     Flag         Dest
```

---

## 3. 字段定义

### 3.1 Family-Magic（家族魔数）

- **偏移**：字节 0-1，16 bits
- **固定值**：`0xCF14`（大端序，即字节 0 = `0xCF`，字节 1 = `0x14`）
- **用途**：识别 CI-144 协议家族。接收端先检查此魔数，非 `0xCF14` 则按非 CI-144 帧处理。
- **设计参考**：Ethernet EtherType（2 字节，40+ 年验证）

### 3.2 Modality（操作模态）

- **偏移**：字节 2，位 0-1，2 bits
- **枚举值**：

| 值 | 名称 | 说明 |
|---|---|---|
| 0 | `COGNITIVE` | 认知操作（思考、推理、记忆检索） |
| 1 | `RENDER` | 渲染操作（文本生成、图像生成、语音合成） |
| 2 | `EXECUTIVE` | 执行操作（工具调用、物理动作、API 调用） |
| 3 | `SENSOR_FEED` | 传感器反馈（摄像头、麦克风、触觉、IMU） |

### 3.3 Risk-Level（风险等级）

- **偏移**：字节 2，位 2-3，2 bits
- **枚举值**：

| 值 | 名称 | 说明 |
|---|---|---|
| 0 | `LOW` | 低风险（只读操作、信息查询） |
| 1 | `MEDIUM` | 中风险（有状态变更、外部调用） |
| 2 | `CRITICAL` | 高风险（不可逆操作、资金操作、物理动作） |
| 3 | `CATASTROPHIC` | 灾难级（可能造成物理伤害、数据丢失、系统崩溃） |

- **特殊规则**：当 `Replay-Enable = 0` 时，有效风险等级强制降级为 `MEDIUM`（规则 6），无论此字段原始值为何。

### 3.4 Body-Stance（本体姿态）

- **偏移**：字节 2，位 4-5，2 bits
- **枚举值**：

| 值 | 名称 | 说明 |
|---|---|---|
| 0 | `SEATED` | 坐姿（稳定，低动能） |
| 1 | `STANDING` | 站姿（中等动能） |
| 2 | `MOVING` | 移动中（高动能，行走/奔跑/飞行） |
| 3 | `UNKNOWN` | 未知（传感器不可用或未初始化） |

### 3.5 Proximity-Edge（临边/高危环境）

- **偏移**：字节 2，位 6-7，2 bits
- **枚举值**：

| 值 | 名称 | 说明 |
|---|---|---|
| 0 | `SAFE` | 安全环境（无物理危险，室内受控环境） |
| 1 | `WARNING` | 警告环境（接近危险源，需注意） |
| 2 | `DANGER` | 危险环境（高温/高压/高处/水域，需防护） |
| 3 | `CRITICAL_EDGE` | 临界边缘（悬崖/高速/爆炸物，需立即停止） |

### 3.6 Output-Dest（输出目的地）

- **偏移**：字节 3，位 0，1 bit
- **枚举值**：

| 值 | 名称 | 说明 |
|---|---|---|
| 0 | `INTERNAL` | 内部输出（组件间通信，不离开本地） |
| 1 | `EXTERNAL` | 外部输出（出网、发送到外部系统、物理输出） |

### 3.7 Override-Flag（硬覆盖标志）

- **偏移**：字节 3，位 1，1 bit
- **枚举值**：

| 值 | 名称 | 说明 |
|---|---|---|
| 0 | `NORMAL` | 正常模式（按本地策略处理） |
| 1 | `HARD_OVERRIDE` | 硬覆盖（灾难场景下无条件放行，优先级高于任何本地策略） |

- **CATASTROPHIC 硬覆盖规则**：当 `Risk-Level == CATASTROPHIC (3)` 且 `Override-Flag == HARD_OVERRIDE (1)` 时，接收端必须在物理层优先响应（事件驱动，禁止轮询），并行通过任何可用通道向人类发送紧急信号。此规则是协议的不可协商部分。

### 3.8 Replay-Enable（防重放使能）

- **偏移**：字节 3，位 2，1 bit
- **枚举值**：

| 值 | 名称 | 说明 |
|---|---|---|
| 0 | `DISABLED` | 防重放禁用（节能模式，跳过 Seq-Counter 检查） |
| 1 | `ENABLED` | 防重放使能（正常模式，强制检查 Seq-Counter） |

- **规则 6 降级**：当 `Replay-Enable = 0` 时：
  1. 有效风险等级强制降级为 `MEDIUM`（无论原始 Risk-Level 为何）
  2. 跳过防重放检查（不检查 Seq-Counter）
  3. 审计日志必须记录 `REPLAY_DISABLED` 事件
  4. 从根本上杜绝利用重放发动高危物理攻击

### 3.9 Reserved（保留位）

- **偏移**：字节 3，位 3-7，5 bits
- **强制值**：全 0
- **治理规则**：未来任何使用保留位的扩展，必须提交正式 ADR 并经协议委员会审查分配。未分配前，接收端检测到保留位非零应触发版本协商流程，不得静默忽略。

---

## 4. 编码示例

### 4.1 典型执行帧（Executive / Medium / Standing / Warning / External / Normal / Replay-Enabled）

```
字段值:
  Family-Magic   = 0xCF14
  Modality       = EXECUTIVE   (2)
  Risk-Level     = MEDIUM      (1)
  Body-Stance    = STANDING    (1)
  Proximity-Edge = WARNING     (1)
  Output-Dest    = EXTERNAL    (1)
  Override-Flag  = NORMAL      (0)
  Replay-Enable  = ENABLED     (1)
  Reserved       = 00000

字节 2 = (Proximity-Edge << 6) | (Body-Stance << 4) | (Risk-Level << 2) | Modality
        = (1 << 6) | (1 << 4) | (1 << 2) | 2
        = 0x40 | 0x10 | 0x04 | 0x02
        = 0x56

字节 3 = (Reserved << 3) | (Replay-Enable << 2) | (Override-Flag << 1) | Output-Dest
        = (0 << 3) | (1 << 2) | (0 << 1) | 1
        = 0x00 | 0x04 | 0x00 | 0x01
        = 0x05

编码结果: CF 14 56 05  (4 字节)
```

### 4.2 CATASTROPHIC 硬覆盖帧

```
字段值:
  Risk-Level     = CATASTROPHIC (3)
  Override-Flag  = HARD_OVERRIDE (1)
  其他字段       = 典型值

字节 2 = ... | (3 << 2) | ... = ... | 0x0C | ...
字节 3 = ... | (1 << 1) | ... = ... | 0x02 | ...

编码结果: CF 14 ?? ??  (Risk=3, Override=1)
```

---

## 5. 解码伪代码

```rust
fn decode_pfp(bytes: &[u8; 4]) -> Result<PfpHeader, Error> {
    // 1. 验证家族魔数
    let magic = u16::from_be_bytes([bytes[0], bytes[1]]);
    if magic != 0xCF14 {
        return Err(Error::InvalidMagic);
    }

    // 2. 解析字节 2
    let modality = (bytes[2] & 0b11) as u8;
    let risk_level = ((bytes[2] >> 2) & 0b11) as u8;
    let body_stance = ((bytes[2] >> 4) & 0b11) as u8;
    let proximity_edge = ((bytes[2] >> 6) & 0b11) as u8;

    // 3. 解析字节 3
    let output_dest = (bytes[3] & 0b1) as u8;
    let override_flag = ((bytes[3] >> 1) & 0b1) as u8;
    let replay_enable = ((bytes[3] >> 2) & 0b1) as u8;
    let reserved = (bytes[3] >> 3) & 0b11111;

    // 4. 验证保留位
    if reserved != 0 {
        return Err(Error::ReservedBitNonZero);
    }

    Ok(PfpHeader { /* ... */ })
}
```

---

## 6. CATASTROPHIC 检测（纯位运算，~3 CPU cycles）

```rust
fn is_catastrophic_override(bytes: &[u8; 4]) -> bool {
    let risk_level = (bytes[2] >> 2) & 0b11;
    let override_flag = (bytes[3] >> 1) & 0b1;
    risk_level == 3 && override_flag == 1
}
```

此函数是 Tuck 硬实时决策路径的核心，仅需 2 次位运算 + 1 次比较，约 3 个 CPU 周期（~0.3 ps）。

---

## 7. 测试向量

完整测试向量见 [CI-144 v2.0 测试向量集](../../tests/test_vectors/ci-144-v2.0-test-vectors.json)，其中 `pfp_codec` 分类包含 5 组 PFP 编解码测试向量：

| ID | 描述 | 编码（hex） |
|---|---|---|
| `pfp-all_zero` | 全零（Cognitive/Low/Unknown/Safe/Internal/Normal/Replay-Disabled） | `cf14 00 00` |
| `pfp-all_one` | 全一（SensorFeed/Catastrophic/Moving/CriticalEdge/External/HardOverride/Replay-Enabled） | `cf14 ff 0f` |
| `pfp-typical_executive` | 典型执行帧 | `cf14 56 05` |
| `pfp-cognitive_low` | 认知低风险 | 见 JSON |
| `pfp-render_critical` | 渲染高风险 | 见 JSON |

---

## 8. 与 SAP 的关系

| 维度 | PFP-xCF14 | SAP-xCF14 |
|---|---|---|
| 层级 | 冻结层 | 演进层 |
| 长度 | 4 字节 | 28 字节 |
| 依赖 | 无（独立存在） | 依赖 PFP（SAP 不能单独存在） |
| Tuck 读取 | 必须（硬实时决策） | 不依赖（可选验证） |
| 变化频率 | 永不变化（冻结） | 可演进（v1/v2 可并行） |
| 核心价值 | 物理特征描述 | 安全证明（防重放+签名） |

---

## 9. 版本历史

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-29 | 初始冻结版本。4 字节结构，8 个字段，家族魔数 0xCF14。 |

---

## 10. 权威来源声明

> **本规范的唯一权威来源在 [CommonIntents/BIND-19/docs/spec/pfp-xcf14.md](https://github.com/CommonIntents/BIND-19/blob/v2.0-rc.1/docs/spec/pfp-xcf14.md)。**
>
> [CommonIntents/PFP-xCF14](https://github.com/CommonIntents/PFP-xCF14) 仓库是本规范的**发布窗口**，内容从 BIND-19 同步而来。所有规范变更的 PR 必须提在 BIND-19 仓库，同时更新 `docs/spec/` + `src/` + `tests/`。
>
> 本规范对应 BIND-19 版本：**[v2.0-rc.1](https://github.com/CommonIntents/BIND-19/tree/v2.0-rc.1)**

---

**文档结束**
