# ADR-0025：CI-144 v2.0 KEY_ROTATION 控制帧格式（BIND-19 新帧类型）

## 状态
**Draft**（2026-08-29，Alpha 实现期间待锁定）

## 上下文
Seq-Counter 回绕时需触发密钥轮换，KEY_ROTATION 控制帧的格式（帧类型、载荷结构）需明确定义。

## 决策（初步定案，Alpha 编码前最终确认）
- **帧类型**：BIND-19 新帧类型（建议 Type=0x07，Alpha 编码前需检查 BIND-19 v1.0 现有帧类型分配，确认 0x07 未被占用；若冲突则重新分配，如 0x0F）
- **载荷格式**：`[new_key_encrypted] + [nonce]`
  - new_key_encrypted：新会话密钥，由主密钥加密保护（AES-256-GCM）
  - nonce：防重放随机数
- 理由：BIND-19 新帧类型是正确的设计选择；new_key_encrypted 由主密钥加密保护，nonce 防重放

## 后果
- 需检查 BIND-19 v1.0 现有帧类型分配，避免冲突
- 需定义 new_key_encrypted 的加密算法（AES-256-GCM 由主密钥加密）
- 需定义 nonce 的生成和验证机制

## 关联
- CI-144 v2.0 升级计划（docs/vision/ci-144-v2.0-upgrade.md）
- ADR-0026（轮换期间帧处理 + ACK 超时机制）
- ADR-0027（主密钥管理）
