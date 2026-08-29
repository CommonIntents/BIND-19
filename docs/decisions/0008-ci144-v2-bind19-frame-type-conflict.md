# ADR-0029：CI-144 v2.0 BIND-19 帧类型 0x07 冲突确认

## 状态
**Draft**（2026-08-29，v2.0-alpha.1 前必须锁定）

## 上下文
CI-144 v2.0 规则 7 定义了 KEY_ROTATION 控制帧，建议使用 BIND-19 帧类型 Type=0x07。但 BIND-19 v1.0 可能已分配该帧类型，需确认是否冲突。

## 决策（初步定案，v2.0-alpha.1 前最终确认）
- 由 Anaphase 协议维护组负责在 v2.0-alpha.1 前完成 BIND-19 v1.0 类型分配表逆向检查
- 若 Type=0x07 未被占用，确认使用 0x07
- 若 Type=0x07 已被占用，重新分配（备用 0x0F），并将确认结果写入本 ADR
- 帧类型分配结果必须在 v2.0-alpha.1 发布前锁定，避免实现不一致

## 后果
- KEY_ROTATION 帧的帧类型在 v2.0-alpha.1 前可能变更
- 实现方需等待本 ADR 锁定后再硬编码帧类型
- 若重新分配，需同步更新规范正文（规则 7）和所有实现

## 关联
- CI-144 v2.0 升级计划（docs/vision/ci-144-v2.0-upgrade.md）
- 规则 7（密钥轮换流程，KEY_ROTATION 帧）
- ADR-0025（KEY_ROTATION 帧格式）
