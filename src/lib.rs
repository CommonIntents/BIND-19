//! BIND-19 — INTENT-7/Transport Binding Protocol
//!
//! CI-144 协议家族的传输绑定层。v2.0 新增 Physical Anchor Layer (PAL)。
//!
//! ## 模块
//! - `pal` — Physical Anchor Layer（24 字节固定偏移头部）

pub mod pal;

pub use pal::{
    BodyStance, Modality, OutputDest, OverrideFlag, PalHeader, ProximityEdge, RiskLevel,
    PAL_SIZE, PAH_SIZE, SIG_SIZE,
};
