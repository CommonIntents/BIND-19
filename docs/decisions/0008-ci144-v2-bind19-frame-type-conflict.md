# ADR-0008：CI-144 v2.0 BIND-19 帧类型 0x07 冲突确认

## 状态
**Active**（2026-08-29，T2 实现期间完成逆向检查，确认 0x07 未被占用）

## 上下文
CI-144 v2.0 规则 7 定义了 KEY_ROTATION 控制帧，建议使用 BIND-19 帧类型 Type=0x07。但 BIND-19 v1.0 可能已分配该帧类型，需确认是否冲突。

## 决策（已锁定）
- **BIND-19 v1.0 类型分配表逆向检查结果**（T2 实现期间完成）：
  - Standard Core（0x01-0x0E，Immutable）：
    - 0x01: Data
    - 0x02: Heartbeat
    - 0x03: Control
    - 0x04: Vector
    - 0x05: Handshake
    - 0x06: Error
    - 0x07-0x0E: **未分配**（Reserved for future Standard Core）
  - Standard Extensions（0x0F-0xEF）：需 RFC 流程
  - Private/Experimental（0xF0-0xFF）：零治理
- **确认结果**：0x07 未被占用，KEY_ROTATION 控制帧使用 **Type=0x07**
- 0x07 属于 Standard Core 范围（0x01-0x0E），一旦 v2.0 发布即冻结，不可重新分配

## 后果
- KEY_ROTATION 帧的帧类型锁定为 0x07
- 实现方可硬编码帧类型 0x07
- 规范正文（规则 7）需同步更新帧类型为 0x07

## 关联
- CI-144 v2.0 升级计划（docs/v2.0-upgrade-plan.md）
- 规则 7（密钥轮换流程，KEY_ROTATION 帧）
- ADR-0004（KEY_ROTATION 帧格式）
- spec/BIND-19.md（v1.0.0-RFC-4 帧类型分配表）
