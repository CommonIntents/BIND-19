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

pub mod frame;
pub mod pfp;
pub mod sap;

pub use frame::{BindFrame, BindHeader, FrameError, FrameType, FLAG_CON, FLAG_FIN, FLAG_PFP_PRESENT, FLAG_SAP_PRESENT, FLAG_SEC, HEADER_SIZE, MAX_PAYLOAD_SIZE};

pub use pfp::{
    BodyStance, Modality, OutputDest, OverrideFlag, PfpHeader, ProximityEdge, RiskLevel,
    FAMILY_MAGIC, PFP_PROTOCOL_ID, PFP_SIZE,
};

pub use sap::{SapHeader, SAP_PROTOCOL_ID, SAP_SIZE, SAP_VERSION, SEQ_ROTATION_THRESHOLD};
