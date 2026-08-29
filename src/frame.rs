//! BIND-19 帧结构（v2.0 协议家族扩展）
//!
//! BIND-19 v2.0 在 v1.0 的 8 字节固定头部基础上，增加 PFP/SAP 可选扩展层。
//!
//! ## 帧结构（v2.0）
//! ```text
//! [ 8 字节 BIND-19 固定头部 ] + [ PFP 4 字节（可选） ] + [ SAP 28 字节（可选） ] + [ Payload 可变长 ]
//! ```
//!
//! ## Flags 扩展（Byte 2）
//! - 0x01: FIN（v1.0，最终分片）
//! - 0x02: CON（v1.0，HITL 共识）
//! - 0x04: SEC（v1.0，帧层加密）
//! - **0x08: PFP-PRESENT（v2.0 新增，PFP 存在）**
//! - **0x10: SAP-PRESENT（v2.0 新增，SAP 存在）**
//! - 0x20-0x80: Reserved（未分配，强制 0）
//!
//! ## 向后兼容
//! - v1.0 接收端忽略 Flags 中的未分配位（规范明确要求）
//! - v2.0 发送端在 Handshake 阶段协商版本，仅当双方都支持 v2.0 时才设置 PFP/SAP 标志
//! - v2.0 接收端收到 PFP-Present=0 的帧时，按 v1.0 逻辑处理（无 PFP/SAP）
//!
//! 规范依据：spec/BIND-19.md（v1.0.0-RFC-4）+ 协议家族架构

use crate::pfp::PfpHeader;
use crate::sap::SapHeader;

/// BIND-19 固定头部长度（字节）
pub const HEADER_SIZE: usize = 8;

/// Payload 最大长度（64MB，防 OOM）
pub const MAX_PAYLOAD_SIZE: usize = 0x0400_0000;

// ─── Flags 位定义 ───────────────────────────────────────────

/// FIN：最终分片标志（v1.0）
pub const FLAG_FIN: u8 = 0x01;
/// CON：HITL 共识标志（v1.0）
pub const FLAG_CON: u8 = 0x02;
/// SEC：帧层加密标志（v1.0）
pub const FLAG_SEC: u8 = 0x04;
/// PFP-PRESENT：PFP 存在标志（v2.0 新增）
pub const FLAG_PFP_PRESENT: u8 = 0x08;
/// SAP-PRESENT：SAP 存在标志（v2.0 新增）
pub const FLAG_SAP_PRESENT: u8 = 0x10;

// ─── Frame Type 定义（v1.0 Standard Core） ─────────────────

/// BIND-19 帧类型（v1.0 Standard Core，0x01-0x0E 不可变）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameType {
    /// 0x01: Data Frame（INTENT-7 载荷）
    Data = 0x01,
    /// 0x02: Heartbeat（保活，Payload 必须为 0）
    Heartbeat = 0x02,
    /// 0x03: Control（控制信号：CANCEL/SUSPEND/RESUME）
    Control = 0x03,
    /// 0x04: Vector（高频增量向量）
    Vector = 0x04,
    /// 0x05: Handshake（传输协商）
    Handshake = 0x05,
    /// 0x06: Error（传输层错误）
    Error = 0x06,
    /// 0x07: KeyRotation（密钥轮换控制帧，v2.0 新增，ADR-0008 确认未被占用）
    KeyRotation = 0x07,
    /// 0x08: KeyRotationAck（密钥轮换确认帧，v2.0 新增）
    KeyRotationAck = 0x08,
    /// 未知帧类型（Standard Extensions 或 Private）
    Unknown(u8),
}

impl FrameType {
    pub fn from_byte(b: u8) -> Self {
        match b {
            0x01 => Self::Data,
            0x02 => Self::Heartbeat,
            0x03 => Self::Control,
            0x04 => Self::Vector,
            0x05 => Self::Handshake,
            0x06 => Self::Error,
            0x07 => Self::KeyRotation,
            0x08 => Self::KeyRotationAck,
            other => Self::Unknown(other),
        }
    }

    pub fn to_byte(self) -> u8 {
        match self {
            Self::Data => 0x01,
            Self::Heartbeat => 0x02,
            Self::Control => 0x03,
            Self::Vector => 0x04,
            Self::Handshake => 0x05,
            Self::Error => 0x06,
            Self::KeyRotation => 0x07,
            Self::KeyRotationAck => 0x08,
            Self::Unknown(b) => b,
        }
    }
}

// ─── BIND-19 固定头部 ───────────────────────────────────────

/// BIND-19 固定头部（8 字节，大端序）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindHeader {
    pub frame_type: FrameType,
    pub channel_id: u8,
    pub flags: u8,
    pub sequence_id: u8,
    /// Payload 长度（仅指 Payload Data，不包含 PFP/SAP）
    pub payload_length: u32,
}

impl BindHeader {
    /// 创建新的 BIND-19 头部
    pub fn new(
        frame_type: FrameType,
        channel_id: u8,
        flags: u8,
        sequence_id: u8,
        payload_length: u32,
    ) -> Self {
        Self {
            frame_type,
            channel_id,
            flags,
            sequence_id,
            payload_length,
        }
    }

    /// 编码为 8 字节大端序数组
    pub fn encode(&self) -> [u8; HEADER_SIZE] {
        let mut buf = [0u8; HEADER_SIZE];
        buf[0] = self.frame_type.to_byte();
        buf[1] = self.channel_id;
        buf[2] = self.flags;
        buf[3] = self.sequence_id;
        buf[4..8].copy_from_slice(&self.payload_length.to_be_bytes());
        buf
    }

    /// 从 8 字节大端序数组解码
    pub fn decode(buf: &[u8; HEADER_SIZE]) -> Self {
        Self {
            frame_type: FrameType::from_byte(buf[0]),
            channel_id: buf[1],
            flags: buf[2],
            sequence_id: buf[3],
            payload_length: u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]),
        }
    }

    /// 检查 PFP-Present 标志
    pub fn has_pfp(&self) -> bool {
        self.flags & FLAG_PFP_PRESENT != 0
    }

    /// 检查 SAP-Present 标志
    pub fn has_sap(&self) -> bool {
        self.flags & FLAG_SAP_PRESENT != 0
    }

    /// 检查 FIN 标志
    pub fn is_fin(&self) -> bool {
        self.flags & FLAG_FIN != 0
    }

    /// 检查 CON 标志
    pub fn is_con(&self) -> bool {
        self.flags & FLAG_CON != 0
    }

    /// 检查 SEC 标志
    pub fn is_sec(&self) -> bool {
        self.flags & FLAG_SEC != 0
    }

    /// 计算 PFP+SAP 扩展层总长度（字节）
    pub fn extension_length(&self) -> usize {
        let mut len = 0;
        if self.has_pfp() {
            len += crate::pfp::PFP_SIZE;
        }
        if self.has_sap() {
            len += crate::sap::SAP_SIZE;
        }
        len
    }

    /// 计算帧总长度（头部 + 扩展层 + Payload）
    pub fn total_frame_length(&self) -> usize {
        HEADER_SIZE + self.extension_length() + self.payload_length as usize
    }
}

// ─── 完整帧（含可选 PFP/SAP/Payload） ──────────────────────

/// BIND-19 完整帧（v2.0，含可选 PFP/SAP 扩展层）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindFrame {
    pub header: BindHeader,
    /// PFP 物理特征层（可选，由 FLAG_PFP_PRESENT 指示）
    pub pfp: Option<PfpHeader>,
    /// SAP 安全证明层（可选，由 FLAG_SAP_PRESENT 指示）
    pub sap: Option<SapHeader>,
    /// Payload 数据（可变长）
    pub payload: Vec<u8>,
}

impl BindFrame {
    /// 创建新的 v2.0 帧（含 PFP/SAP）
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        frame_type: FrameType,
        channel_id: u8,
        sequence_id: u8,
        pfp: Option<PfpHeader>,
        sap: Option<SapHeader>,
        payload: Vec<u8>,
    ) -> Result<Self, FrameError> {
        if payload.len() > MAX_PAYLOAD_SIZE {
            return Err(FrameError::BufferOverflow);
        }

        // SAP 依赖 PFP（SAP 是 PFP 的安全证明，不能单独存在）
        if sap.is_some() && pfp.is_none() {
            return Err(FrameError::SapWithoutPfp);
        }

        let mut flags = 0u8;
        if pfp.is_some() {
            flags |= FLAG_PFP_PRESENT;
        }
        if sap.is_some() {
            flags |= FLAG_SAP_PRESENT;
        }

        let header = BindHeader::new(
            frame_type,
            channel_id,
            flags,
            sequence_id,
            payload.len() as u32,
        );

        Ok(Self {
            header,
            pfp,
            sap,
            payload,
        })
    }

    /// 创建 v1.0 兼容帧（无 PFP/SAP）
    pub fn new_v1(
        frame_type: FrameType,
        channel_id: u8,
        sequence_id: u8,
        payload: Vec<u8>,
    ) -> Result<Self, FrameError> {
        Self::new(frame_type, channel_id, sequence_id, None, None, payload)
    }

    /// 编码为字节向量（大端序）
    pub fn encode(&self) -> Vec<u8> {
        let total_len = self.header.total_frame_length();
        let mut buf = Vec::with_capacity(total_len);

        // 8 字节头部
        buf.extend_from_slice(&self.header.encode());

        // PFP（4 字节，可选）
        if let Some(pfp) = &self.pfp {
            buf.extend_from_slice(&pfp.encode());
        }

        // SAP（28 字节，可选）
        if let Some(sap) = &self.sap {
            buf.extend_from_slice(&sap.encode());
        }

        // Payload
        buf.extend_from_slice(&self.payload);

        buf
    }

    /// 从字节切片解码（需要完整帧数据）
    pub fn decode(buf: &[u8]) -> Result<Self, FrameError> {
        if buf.len() < HEADER_SIZE {
            return Err(FrameError::IncompleteHeader);
        }

        let mut header_bytes = [0u8; HEADER_SIZE];
        header_bytes.copy_from_slice(&buf[0..HEADER_SIZE]);
        let header = BindHeader::decode(&header_bytes);

        let ext_len = header.extension_length();
        let total_needed = HEADER_SIZE + ext_len + header.payload_length as usize;

        if buf.len() < total_needed {
            return Err(FrameError::IncompleteFrame);
        }

        if header.payload_length as usize > MAX_PAYLOAD_SIZE {
            return Err(FrameError::BufferOverflow);
        }

        let mut offset = HEADER_SIZE;

        // 解码 PFP（可选）
        let pfp = if header.has_pfp() {
            let mut pfp_bytes = [0u8; crate::pfp::PFP_SIZE];
            pfp_bytes.copy_from_slice(&buf[offset..offset + crate::pfp::PFP_SIZE]);
            // 验证 PFP 魔数
            if !PfpHeader::verify_magic(&pfp_bytes) {
                return Err(FrameError::InvalidPfpMagic);
            }
            offset += crate::pfp::PFP_SIZE;
            Some(PfpHeader::decode(&pfp_bytes))
        } else {
            None
        };

        // 解码 SAP（可选）
        let sap = if header.has_sap() {
            let mut sap_bytes = [0u8; crate::sap::SAP_SIZE];
            sap_bytes.copy_from_slice(&buf[offset..offset + crate::sap::SAP_SIZE]);
            // 验证 SAP 魔数和协议 ID
            if !SapHeader::verify_magic(&sap_bytes) {
                return Err(FrameError::InvalidSapMagic);
            }
            if !SapHeader::verify_protocol_id(&sap_bytes) {
                return Err(FrameError::InvalidSapProtocolId);
            }
            offset += crate::sap::SAP_SIZE;
            Some(SapHeader::decode(&sap_bytes))
        } else {
            None
        };

        // 解码 Payload
        let payload = buf[offset..offset + header.payload_length as usize].to_vec();

        Ok(Self {
            header,
            pfp,
            sap,
            payload,
        })
    }

    /// 是否为 v1.0 兼容帧（无 PFP/SAP）
    pub fn is_v1_compatible(&self) -> bool {
        self.pfp.is_none() && self.sap.is_none()
    }

    /// 计算有效风险等级（考虑规则 6：Replay-Enable=0 强制降级）
    ///
    /// - 无 PFP 的帧（v1.0 兼容）：返回 None（无风险等级）
    /// - 有 PFP 的帧：
    ///   - 生产模式：Replay-Enable=0 时强制降级至 MEDIUM
    ///   - 调试模式（CI144_DEBUG=1）：规则 6 可跳过，返回原始风险等级
    ///
    /// 注意：CATASTROPHIC 硬覆盖（规则 1-3）不受调试模式影响，始终生效。
    pub fn effective_risk_level(&self) -> Option<crate::pfp::RiskLevel> {
        self.effective_risk_level_with_config(crate::config::BindConfig::global())
    }

    /// 使用指定配置计算有效风险等级（用于测试）
    pub fn effective_risk_level_with_config(
        &self,
        config: &crate::config::BindConfig,
    ) -> Option<crate::pfp::RiskLevel> {
        let pfp = self.pfp.as_ref()?;

        if config.rule6_enabled() {
            // 生产模式：应用规则 6 降级
            Some(pfp.effective_risk_level())
        } else {
            // 调试模式：跳过规则 6，返回原始风险等级
            Some(pfp.risk_level)
        }
    }

    /// 检查是否触发 CATASTROPHIC 硬覆盖（规则 1，始终生效）
    ///
    /// 无论是否调试模式，CATASTROPHIC 硬覆盖始终生效。
    pub fn is_catastrophic_override(&self) -> bool {
        match &self.pfp {
            Some(pfp) => pfp.is_catastrophic_override(),
            None => false,
        }
    }

    /// 检查 Replay-Enable 是否为 0（规则 6 触发条件）
    pub fn is_replay_disabled(&self) -> bool {
        match &self.pfp {
            Some(pfp) => !pfp.replay_enable,
            None => false, // v1.0 帧无 Replay-Enable 概念
        }
    }
}

// ─── 错误类型 ───────────────────────────────────────────────

/// BIND-19 帧解析错误
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    /// 头部不完整（不足 8 字节）
    IncompleteHeader,
    /// 帧不完整（数据不足）
    IncompleteFrame,
    /// Payload 超过 64MB 上限
    BufferOverflow,
    /// SAP 不能单独存在（必须有 PFP）
    SapWithoutPfp,
    /// PFP 魔数无效（非 0xCF14）
    InvalidPfpMagic,
    /// SAP 魔数无效（非 0xCF14）
    InvalidSapMagic,
    /// SAP 协议 ID 无效（非 0x01）
    InvalidSapProtocolId,
}

impl core::fmt::Display for FrameError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::IncompleteHeader => write!(f, "incomplete BIND-19 header (need 8 bytes)"),
            Self::IncompleteFrame => write!(f, "incomplete BIND-19 frame"),
            Self::BufferOverflow => write!(f, "payload exceeds 64MB hard limit"),
            Self::SapWithoutPfp => write!(f, "SAP cannot exist without PFP"),
            Self::InvalidPfpMagic => write!(f, "invalid PFP family magic (expected 0xCF14)"),
            Self::InvalidSapMagic => write!(f, "invalid SAP family magic (expected 0xCF14)"),
            Self::InvalidSapProtocolId => write!(f, "invalid SAP protocol ID (expected 0x01)"),
        }
    }
}

impl std::error::Error for FrameError {}

// ─── 单元测试 ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BindConfig;
    use crate::pfp::{Modality, OutputDest, OverrideFlag, PfpHeader, ProximityEdge, RiskLevel, BodyStance};
    use crate::sap::SapHeader;

    #[test]
    fn test_header_encode_decode_roundtrip() {
        let header = BindHeader::new(
            FrameType::Data,
            0x42,
            FLAG_FIN | FLAG_PFP_PRESENT,
            0x07,
            1024,
        );
        let encoded = header.encode();
        assert_eq!(encoded.len(), HEADER_SIZE);
        let decoded = BindHeader::decode(&encoded);
        assert_eq!(header, decoded);
    }

    #[test]
    fn test_header_flags() {
        let header = BindHeader::new(
            FrameType::Data,
            0,
            FLAG_FIN | FLAG_CON | FLAG_SEC | FLAG_PFP_PRESENT | FLAG_SAP_PRESENT,
            0,
            0,
        );
        assert!(header.is_fin());
        assert!(header.is_con());
        assert!(header.is_sec());
        assert!(header.has_pfp());
        assert!(header.has_sap());
    }

    #[test]
    fn test_header_extension_length() {
        // 无扩展
        let h1 = BindHeader::new(FrameType::Data, 0, 0, 0, 0);
        assert_eq!(h1.extension_length(), 0);
        assert_eq!(h1.total_frame_length(), HEADER_SIZE);

        // 仅 PFP
        let h2 = BindHeader::new(FrameType::Data, 0, FLAG_PFP_PRESENT, 0, 0);
        assert_eq!(h2.extension_length(), 4);
        assert_eq!(h2.total_frame_length(), HEADER_SIZE + 4);

        // PFP + SAP
        let h3 = BindHeader::new(FrameType::Data, 0, FLAG_PFP_PRESENT | FLAG_SAP_PRESENT, 0, 0);
        assert_eq!(h3.extension_length(), 4 + 28);
        assert_eq!(h3.total_frame_length(), HEADER_SIZE + 4 + 28);
    }

    #[test]
    fn test_frame_v1_encode_decode_roundtrip() {
        let payload = b"Hello, BIND-19 v1.0!".to_vec();
        let frame = BindFrame::new_v1(FrameType::Data, 0x01, 0x00, payload.clone()).unwrap();
        assert!(frame.is_v1_compatible());
        assert_eq!(frame.header.payload_length, payload.len() as u32);

        let encoded = frame.encode();
        assert_eq!(encoded.len(), HEADER_SIZE + payload.len());

        let decoded = BindFrame::decode(&encoded).unwrap();
        assert_eq!(decoded.header.frame_type, FrameType::Data);
        assert_eq!(decoded.header.channel_id, 0x01);
        assert!(decoded.is_v1_compatible());
        assert_eq!(decoded.payload, payload);
    }

    #[test]
    fn test_frame_v2_with_pfp_encode_decode_roundtrip() {
        let pfp = PfpHeader::new(
            Modality::Executive,
            RiskLevel::Critical,
            BodyStance::Moving,
            ProximityEdge::Danger,
            OutputDest::External,
            OverrideFlag::Normal,
            true,
        );
        let payload = b"v2.0 with PFP".to_vec();
        let frame = BindFrame::new(
            FrameType::Data,
            0x02,
            0x01,
            Some(pfp.clone()),
            None,
            payload.clone(),
        )
        .unwrap();

        assert!(!frame.is_v1_compatible());
        assert!(frame.header.has_pfp());
        assert!(!frame.header.has_sap());
        assert_eq!(frame.header.extension_length(), 4);

        let encoded = frame.encode();
        assert_eq!(encoded.len(), HEADER_SIZE + 4 + payload.len());

        let decoded = BindFrame::decode(&encoded).unwrap();
        assert_eq!(decoded.pfp, Some(pfp));
        assert_eq!(decoded.sap, None);
        assert_eq!(decoded.payload, payload);
    }

    #[test]
    fn test_frame_v2_with_pfp_and_sap_encode_decode_roundtrip() {
        let pfp = PfpHeader::default();
        let sap = SapHeader::new(
            12345,
            [0xAB; 14],
            [0xCD; 8],
        );
        let payload = b"v2.0 full security".to_vec();
        let frame = BindFrame::new(
            FrameType::Data,
            0x03,
            0x02,
            Some(pfp.clone()),
            Some(sap.clone()),
            payload.clone(),
        )
        .unwrap();

        assert!(frame.header.has_pfp());
        assert!(frame.header.has_sap());
        assert_eq!(frame.header.extension_length(), 4 + 28);

        let encoded = frame.encode();
        assert_eq!(encoded.len(), HEADER_SIZE + 4 + 28 + payload.len());

        let decoded = BindFrame::decode(&encoded).unwrap();
        assert_eq!(decoded.pfp, Some(pfp));
        assert_eq!(decoded.sap, Some(sap));
        assert_eq!(decoded.payload, payload);
    }

    #[test]
    fn test_sap_without_pfp_rejected() {
        let sap = SapHeader::default();
        let result = BindFrame::new(
            FrameType::Data,
            0,
            0,
            None,
            Some(sap),
            vec![],
        );
        assert_eq!(result.err(), Some(FrameError::SapWithoutPfp));
    }

    #[test]
    fn test_buffer_overflow_rejected() {
        let payload = vec![0u8; MAX_PAYLOAD_SIZE + 1];
        let result = BindFrame::new_v1(FrameType::Data, 0, 0, payload);
        assert_eq!(result.err(), Some(FrameError::BufferOverflow));
    }

    #[test]
    fn test_incomplete_header_rejected() {
        let buf = [0u8; 4]; // 不足 8 字节
        let result = BindFrame::decode(&buf);
        assert_eq!(result.err(), Some(FrameError::IncompleteHeader));
    }

    #[test]
    fn test_incomplete_frame_rejected() {
        // 头部完整，但 Payload 不足
        let header = BindHeader::new(FrameType::Data, 0, 0, 0, 100);
        let mut buf = header.encode().to_vec();
        buf.extend_from_slice(&[0u8; 50]); // 只有 50 字节，不足 100
        let result = BindFrame::decode(&buf);
        assert_eq!(result.err(), Some(FrameError::IncompleteFrame));
    }

    #[test]
    fn test_invalid_pfp_magic_rejected() {
        // 构造一个 PFP-Present 标志，但魔数错误的帧
        let header = BindHeader::new(FrameType::Data, 0, FLAG_PFP_PRESENT, 0, 0);
        let mut buf = header.encode().to_vec();
        buf.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // 错误的魔数（非 0xCF14）
        let result = BindFrame::decode(&buf);
        assert_eq!(result.err(), Some(FrameError::InvalidPfpMagic));
    }

    #[test]
    fn test_frame_type_unknown() {
        let header = BindHeader::new(FrameType::Unknown(0x0F), 0, 0, 0, 0);
        let encoded = header.encode();
        let decoded = BindHeader::decode(&encoded);
        assert_eq!(decoded.frame_type, FrameType::Unknown(0x0F));
        assert_eq!(decoded.frame_type.to_byte(), 0x0F);
    }

    #[test]
    fn test_all_frame_types_roundtrip() {
        for ft in [
            FrameType::Data,
            FrameType::Heartbeat,
            FrameType::Control,
            FrameType::Vector,
            FrameType::Handshake,
            FrameType::Error,
            FrameType::Unknown(0x0F),
            FrameType::Unknown(0xF0),
        ] {
            let header = BindHeader::new(ft, 0, 0, 0, 0);
            let encoded = header.encode();
            let decoded = BindHeader::decode(&encoded);
            assert_eq!(decoded.frame_type, ft);
        }
    }

    #[test]
    fn test_payload_length_max_boundary() {
        // 恰好 64MB 应该成功
        let payload = vec![0u8; MAX_PAYLOAD_SIZE];
        let frame = BindFrame::new_v1(FrameType::Data, 0, 0, payload).unwrap();
        assert_eq!(frame.header.payload_length, MAX_PAYLOAD_SIZE as u32);

        // 超过 64MB 应该失败
        let payload = vec![0u8; MAX_PAYLOAD_SIZE + 1];
        assert!(BindFrame::new_v1(FrameType::Data, 0, 0, payload).is_err());
    }

    // ─── T4 集成测试：规则 6 + 调试模式 ─────────────────────

    #[test]
    fn test_effective_risk_level_v1_frame() {
        // v1.0 帧无 PFP，有效风险等级为 None
        let frame = BindFrame::new_v1(FrameType::Data, 0, 0, vec![]).unwrap();
        let config = BindConfig::default();
        assert_eq!(frame.effective_risk_level_with_config(&config), None);
    }

    #[test]
    fn test_effective_risk_level_production_replay_enabled() {
        // 生产模式 + Replay-Enable=1 → 保持原始风险等级
        let pfp = PfpHeader::new(
            Modality::Executive,
            RiskLevel::Critical,
            BodyStance::Moving,
            ProximityEdge::Danger,
            OutputDest::External,
            OverrideFlag::Normal,
            true, // replay_enable
        );
        let frame = BindFrame::new(FrameType::Data, 0, 0, Some(pfp), None, vec![]).unwrap();
        let config = BindConfig::default(); // 生产模式
        assert_eq!(
            frame.effective_risk_level_with_config(&config),
            Some(RiskLevel::Critical)
        );
    }

    #[test]
    fn test_effective_risk_level_production_replay_disabled() {
        // 生产模式 + Replay-Enable=0 → 强制降级至 MEDIUM（规则 6）
        let pfp = PfpHeader::new(
            Modality::Executive,
            RiskLevel::Catastrophic,
            BodyStance::Moving,
            ProximityEdge::CriticalEdge,
            OutputDest::External,
            OverrideFlag::HardOverride,
            false, // replay_enable = 0
        );
        let frame = BindFrame::new(FrameType::Data, 0, 0, Some(pfp), None, vec![]).unwrap();
        let config = BindConfig::default(); // 生产模式
        assert_eq!(
            frame.effective_risk_level_with_config(&config),
            Some(RiskLevel::Medium) // 强制降级
        );
        // 原始风险等级仍保留
        assert_eq!(frame.pfp.unwrap().risk_level, RiskLevel::Catastrophic);
    }

    #[test]
    fn test_effective_risk_level_debug_replay_disabled() {
        // 调试模式 + Replay-Enable=0 → 跳过规则 6，返回原始风险等级
        let pfp = PfpHeader::new(
            Modality::Executive,
            RiskLevel::Catastrophic,
            BodyStance::Moving,
            ProximityEdge::CriticalEdge,
            OutputDest::External,
            OverrideFlag::HardOverride,
            false, // replay_enable = 0
        );
        let frame = BindFrame::new(FrameType::Data, 0, 0, Some(pfp), None, vec![]).unwrap();
        let config = BindConfig { debug_mode: true }; // 调试模式
        assert_eq!(
            frame.effective_risk_level_with_config(&config),
            Some(RiskLevel::Catastrophic) // 不降级
        );
    }

    #[test]
    fn test_catastrophic_override_always_enabled() {
        // CATASTROPHIC 硬覆盖（规则 1）始终生效，不受调试模式影响
        let pfp = PfpHeader::new(
            Modality::Executive,
            RiskLevel::Catastrophic,
            BodyStance::Moving,
            ProximityEdge::CriticalEdge,
            OutputDest::External,
            OverrideFlag::HardOverride,
            false, // replay_enable = 0
        );
        let frame = BindFrame::new(FrameType::Data, 0, 0, Some(pfp), None, vec![]).unwrap();

        // 生产模式：CATASTROPHIC 硬覆盖触发
        assert!(frame.is_catastrophic_override());

        // 调试模式：CATASTROPHIC 硬覆盖仍然触发（规则 1-3 不可跳过）
        let config = BindConfig { debug_mode: true };
        assert!(config.catastrophic_rules_enabled());
        assert!(frame.is_catastrophic_override());
    }

    #[test]
    fn test_is_replay_disabled() {
        // Replay-Enable=0 的帧
        let pfp_off = PfpHeader::new(
            Modality::Cognitive,
            RiskLevel::Low,
            BodyStance::Unknown,
            ProximityEdge::Safe,
            OutputDest::Internal,
            OverrideFlag::Normal,
            false,
        );
        let frame_off =
            BindFrame::new(FrameType::Data, 0, 0, Some(pfp_off), None, vec![]).unwrap();
        assert!(frame_off.is_replay_disabled());

        // Replay-Enable=1 的帧
        let pfp_on = PfpHeader::new(
            Modality::Cognitive,
            RiskLevel::Low,
            BodyStance::Unknown,
            ProximityEdge::Safe,
            OutputDest::Internal,
            OverrideFlag::Normal,
            true,
        );
        let frame_on =
            BindFrame::new(FrameType::Data, 0, 0, Some(pfp_on), None, vec![]).unwrap();
        assert!(!frame_on.is_replay_disabled());

        // v1.0 帧无 Replay-Enable 概念
        let frame_v1 = BindFrame::new_v1(FrameType::Data, 0, 0, vec![]).unwrap();
        assert!(!frame_v1.is_replay_disabled());
    }

    #[test]
    fn test_rule6_interaction_with_catastrophic() {
        // 规则 6 降级至 MEDIUM 后，CATASTROPHIC 硬覆盖不再触发
        // （因为有效风险等级是 MEDIUM，不是 CATASTROPHIC）
        let pfp = PfpHeader::new(
            Modality::Executive,
            RiskLevel::Catastrophic,
            BodyStance::Moving,
            ProximityEdge::CriticalEdge,
            OutputDest::External,
            OverrideFlag::HardOverride,
            false, // replay_enable = 0 → 规则 6 降级
        );
        let frame = BindFrame::new(FrameType::Data, 0, 0, Some(pfp), None, vec![]).unwrap();
        let config = BindConfig::default(); // 生产模式

        // 原始 PFP 标记为 CATASTROPHIC 硬覆盖
        assert!(frame.is_catastrophic_override());

        // 但有效风险等级被降级至 MEDIUM
        // 实际决策时应使用有效风险等级，而非原始风险等级
        assert_eq!(
            frame.effective_risk_level_with_config(&config),
            Some(RiskLevel::Medium)
        );

        // 调试模式下不降级，有效风险等级为 CATASTROPHIC
        let debug_config = BindConfig { debug_mode: true };
        assert_eq!(
            frame.effective_risk_level_with_config(&debug_config),
            Some(RiskLevel::Catastrophic)
        );
    }
}
