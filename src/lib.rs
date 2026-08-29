//! BIND-19 — INTENT-7/Transport Binding Protocol
//!
//! CI-144 协议家族的传输绑定层。v2.0 新增协议家族架构：
//! - PFP-xCF14（Physical Feature Protocol，4 字节，冻结层）
//! - SAP-xCF14（Security Attestation Protocol，28 字节，演进层，按需加载）
//!
//! ## 模块
//! - `pfp` — Physical Feature Protocol（物理特征协议，Tuck 硬实时只读 4 字节）
//! - `sap` — Security Attestation Protocol（安全证明协议，防重放 + 完整性校验）
//! - `frame` — BIND-19 帧结构（8 字节头部 + 可选 PFP/SAP 扩展层 + Payload）
//! - `crypto` — PAH 第一层 64-bit 签名验证（ed25519 软件实现 + SHA-256 截断）
//! - `config` — 运行时配置（调试模式 CI144_DEBUG + 环境变量）
//! - `rotation` — 密钥轮换状态机（KEY_ROTATION 帧 + ACK 超时 fail-closed）
//! - `catastrophic` — CATASTROPHIC 硬覆盖（规则 1-3）+ 事件驱动总线 + 审计日志
//! - `replay_cache` — 高并发防重放缓存（DashMap + TTL 清理，多租户隔离）
//! - `processor` — 帧处理器（整合防重放缓存 + 帧解码 + 规则检查）

pub mod catastrophic;
pub mod config;
pub mod crypto;
pub mod frame;
pub mod pfp;
pub mod processor;
pub mod replay_cache;
pub mod rotation;
pub mod sap;

pub use catastrophic::{
    AuditEntry, CatastrophicAuditLog, CatastrophicEvent, CatastrophicEventBus,
    CatastrophicEventReceiver, CatastrophicManager, AUDIT_LOG_MAX_ENTRIES,
    EVENT_BUS_CAPACITY,
};
pub use config::{BindConfig, DEBUG_ENV_VAR};
pub use processor::{FrameProcessResult, FrameProcessor};
pub use replay_cache::{
    ReplayCache, ReplayCheckResult, ReplayKey, SourceId, TenantId,
    REPLAY_CACHE_DEFAULT_TTL, REPLAY_CACHE_MAX_CAPACITY,
};
pub use rotation::{
    KeyRotationPayload, KeyRotationStateMachine, RotationError, RotationState, TimeoutResult,
    ACK_TIMEOUT, MAX_RETRIES, NONCE_SIZE, ROTATION_THRESHOLD,
};

pub use crypto::{
    compute_pah_hash, sign, sign_truncated, truncate_signature, verify, verify_truncated_match,
    KeyPair, FULL_SIG_SIZE, TRUNCATED_SIG_SIZE,
};

pub use frame::{BindFrame, BindHeader, FrameError, FrameType, FLAG_CON, FLAG_FIN, FLAG_PFP_PRESENT, FLAG_SAP_PRESENT, FLAG_SEC, HEADER_SIZE, MAX_PAYLOAD_SIZE};

pub use pfp::{
    BodyStance, Modality, OutputDest, OverrideFlag, PfpHeader, ProximityEdge, RiskLevel,
    FAMILY_MAGIC, PFP_PROTOCOL_ID, PFP_SIZE,
};

pub use sap::{SapHeader, SAP_PROTOCOL_ID, SAP_SIZE, SAP_VERSION, SEQ_ROTATION_THRESHOLD};
