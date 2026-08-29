//! Physical Anchor Layer (PAL) — 24-byte fixed-offset header
//!
//! PAL 是 CI-144 v2.0 的物理锚定层，提供明文、固定偏移、可被 Tuck 硬实时读取的元数据。
//! 总长度 24 字节（192 bits）= 3×64-bit 对齐 SIMD。
//!
//! 规范依据：docs/v2.0-upgrade-plan.md 第四章
//! ADR：ADR-0001（PAH 第二层签名位置）

/// PAL 总长度（字节）
pub const PAL_SIZE: usize = 24;

/// Physical-Context-Hash 长度（字节）= 112 bits
pub const PAH_SIZE: usize = 14;

/// PAH-Signature 长度（字节）= 64 bits
pub const SIG_SIZE: usize = 8;

/// PAL 当前版本（v2.0 = 0001）
pub const PAL_VERSION: u8 = 0b0001;

// ─── 枚举类型 ───────────────────────────────────────────────

/// 操作模态（bit 0-1）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Modality {
    Cognitive = 0,
    Render = 1,
    Executive = 2,
    SensorFeed = 3,
}

impl Modality {
    pub fn from_bits(bits: u8) -> Self {
        match bits & 0b11 {
            0 => Self::Cognitive,
            1 => Self::Render,
            2 => Self::Executive,
            _ => Self::SensorFeed,
        }
    }

    pub fn to_bits(self) -> u8 {
        self as u8
    }
}

/// 风险等级（bit 2-3）
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum RiskLevel {
    Low = 0,
    Medium = 1,
    Critical = 2,
    Catastrophic = 3,
}

impl RiskLevel {
    pub fn from_bits(bits: u8) -> Self {
        match bits & 0b11 {
            0 => Self::Low,
            1 => Self::Medium,
            2 => Self::Critical,
            _ => Self::Catastrophic,
        }
    }

    pub fn to_bits(self) -> u8 {
        self as u8
    }
}

/// 本体姿态（bit 4-5）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BodyStance {
    Seated = 0,
    Standing = 1,
    Moving = 2,
    Unknown = 3,
}

impl BodyStance {
    pub fn from_bits(bits: u8) -> Self {
        match bits & 0b11 {
            0 => Self::Seated,
            1 => Self::Standing,
            2 => Self::Moving,
            _ => Self::Unknown,
        }
    }

    pub fn to_bits(self) -> u8 {
        self as u8
    }
}

/// 临边/高危环境（bit 6-7）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ProximityEdge {
    Safe = 0,
    Warning = 1,
    Danger = 2,
    CriticalEdge = 3,
}

impl ProximityEdge {
    pub fn from_bits(bits: u8) -> Self {
        match bits & 0b11 {
            0 => Self::Safe,
            1 => Self::Warning,
            2 => Self::Danger,
            _ => Self::CriticalEdge,
        }
    }

    pub fn to_bits(self) -> u8 {
        self as u8
    }
}

/// 输出目的地（bit 8）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OutputDest {
    Internal = 0,
    External = 1,
}

impl OutputDest {
    pub fn from_bit(bit: bool) -> Self {
        if bit { Self::External } else { Self::Internal }
    }

    pub fn to_bit(self) -> bool {
        self == Self::External
    }
}

/// 硬覆盖标志（bit 9）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OverrideFlag {
    Normal = 0,
    HardOverride = 1,
}

impl OverrideFlag {
    pub fn from_bit(bit: bool) -> Self {
        if bit { Self::HardOverride } else { Self::Normal }
    }

    pub fn to_bit(self) -> bool {
        self == Self::HardOverride
    }
}

// ─── PAL 结构体 ─────────────────────────────────────────────

/// Physical Anchor Layer — 24 字节固定偏移头部
///
/// 内存布局（大端序，网络字节序）：
/// ```text
/// Byte 0-1:   控制字段（16 bits）
///   bit 0-1:   Modality
///   bit 2-3:   Risk-Level
///   bit 4-5:   Body-Stance
///   bit 6-7:   Proximity-Edge
///   bit 8:     Output-Dest
///   bit 9:     Override-Flag
///   bit 10-13: PAL-Version（4 bits，当前 v2.0 = 0001）
///   bit 14:    Replay-Enable
///   bit 15:    Reserved（强制为 0）
/// Byte 2-15:  Physical-Context-Hash（112 bits = 14 bytes，SHA-256 截断高 112 位）
/// Byte 16-23: PAH-Signature（64 bits = 8 bytes，ECC 签名截断 = SHA-256(完整签名) 前 64 位）
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PalHeader {
    pub modality: Modality,
    pub risk_level: RiskLevel,
    pub body_stance: BodyStance,
    pub proximity_edge: ProximityEdge,
    pub output_dest: OutputDest,
    pub override_flag: OverrideFlag,
    pub pal_version: u8,
    pub replay_enable: bool,
    /// Reserved 位（bit 15），强制为 0。非零值触发版本协商流程。
    pub reserved: bool,
    /// Physical-Context-Hash（14 bytes，SHA-256 截断高 112 位）
    pub physical_context_hash: [u8; PAH_SIZE],
    /// PAH-Signature（8 bytes，ECC 签名截断 = SHA-256(完整签名) 前 64 位）
    pub pah_signature: [u8; SIG_SIZE],
}

impl PalHeader {
    /// 创建新的 PAL 头部（默认 v2.0，Reserved=0）
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        modality: Modality,
        risk_level: RiskLevel,
        body_stance: BodyStance,
        proximity_edge: ProximityEdge,
        output_dest: OutputDest,
        override_flag: OverrideFlag,
        replay_enable: bool,
        physical_context_hash: [u8; PAH_SIZE],
        pah_signature: [u8; SIG_SIZE],
    ) -> Self {
        Self {
            modality,
            risk_level,
            body_stance,
            proximity_edge,
            output_dest,
            override_flag,
            pal_version: PAL_VERSION,
            replay_enable,
            reserved: false,
            physical_context_hash,
            pah_signature,
        }
    }

    /// 编码为 24 字节大端序数组（固定偏移，零拷贝友好）
    pub fn encode(&self) -> [u8; PAL_SIZE] {
        let mut buf = [0u8; PAL_SIZE];

        // Byte 0: 低 8 位控制字段
        buf[0] = (self.modality.to_bits() & 0b11)
            | ((self.risk_level.to_bits() & 0b11) << 2)
            | ((self.body_stance.to_bits() & 0b11) << 4)
            | ((self.proximity_edge.to_bits() & 0b11) << 6);

        // Byte 1: 高 8 位控制字段
        let mut byte1: u8 = 0;
        if self.output_dest.to_bit() { byte1 |= 1 << 0; }
        if self.override_flag.to_bit() { byte1 |= 1 << 1; }
        byte1 |= (self.pal_version & 0b1111) << 2;
        if self.replay_enable { byte1 |= 1 << 6; }
        // bit 7 (Reserved) 强制为 0，不设置
        buf[1] = byte1;

        // Byte 2-15: Physical-Context-Hash
        buf[2..2 + PAH_SIZE].copy_from_slice(&self.physical_context_hash);

        // Byte 16-23: PAH-Signature
        buf[2 + PAH_SIZE..2 + PAH_SIZE + SIG_SIZE].copy_from_slice(&self.pah_signature);

        buf
    }

    /// 从 24 字节大端序数组解码（固定偏移读取，零分配）
    pub fn decode(buf: &[u8; PAL_SIZE]) -> Self {
        let byte0 = buf[0];
        let byte1 = buf[1];

        let modality = Modality::from_bits(byte0 & 0b11);
        let risk_level = RiskLevel::from_bits((byte0 >> 2) & 0b11);
        let body_stance = BodyStance::from_bits((byte0 >> 4) & 0b11);
        let proximity_edge = ProximityEdge::from_bits((byte0 >> 6) & 0b11);

        let output_dest = OutputDest::from_bit((byte1 & (1 << 0)) != 0);
        let override_flag = OverrideFlag::from_bit((byte1 & (1 << 1)) != 0);
        let pal_version = (byte1 >> 2) & 0b1111;
        let replay_enable = (byte1 & (1 << 6)) != 0;
        let reserved = (byte1 & (1 << 7)) != 0;

        let mut physical_context_hash = [0u8; PAH_SIZE];
        physical_context_hash.copy_from_slice(&buf[2..2 + PAH_SIZE]);

        let mut pah_signature = [0u8; SIG_SIZE];
        pah_signature.copy_from_slice(&buf[2 + PAH_SIZE..2 + PAH_SIZE + SIG_SIZE]);

        Self {
            modality,
            risk_level,
            body_stance,
            proximity_edge,
            output_dest,
            override_flag,
            pal_version,
            replay_enable,
            reserved,
            physical_context_hash,
            pah_signature,
        }
    }

    /// 检查是否触发 CATASTROPHIC 硬覆盖（规则 1）
    pub fn is_catastrophic_override(&self) -> bool {
        self.risk_level == RiskLevel::Catastrophic
            && self.override_flag == OverrideFlag::HardOverride
    }

    /// 检查 Replay-Enable=0 时的有效风险等级（规则 6：强制降级至 MEDIUM）
    ///
    /// 注意：调试模式（CI144_DEBUG=1）下可跳过降级，由调用方决定。
    pub fn effective_risk_level(&self) -> RiskLevel {
        if !self.replay_enable {
            // 规则 6：Replay-Enable=0 时强制降级至 MEDIUM
            RiskLevel::Medium
        } else {
            self.risk_level
        }
    }

    /// 检查 Reserved 位是否非零（触发版本协商流程）
    pub fn has_unknown_reserved(&self) -> bool {
        self.reserved
    }

    /// 检查 PAL 版本是否匹配当前版本
    pub fn is_version_current(&self) -> bool {
        self.pal_version == PAL_VERSION
    }
}

impl Default for PalHeader {
    fn default() -> Self {
        Self::new(
            Modality::Cognitive,
            RiskLevel::Low,
            BodyStance::Unknown,
            ProximityEdge::Safe,
            OutputDest::Internal,
            OverrideFlag::Normal,
            true, // replay_enable 默认开启
            [0u8; PAH_SIZE],
            [0u8; SIG_SIZE],
        )
    }
}

// ─── 单元测试 ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pal_size_constants() {
        assert_eq!(PAL_SIZE, 24);
        assert_eq!(PAH_SIZE, 14);
        assert_eq!(SIG_SIZE, 8);
        assert_eq!(2 + PAH_SIZE + SIG_SIZE, PAL_SIZE);
    }

    #[test]
    fn test_encode_decode_roundtrip_default() {
        let pal = PalHeader::default();
        let encoded = pal.encode();
        assert_eq!(encoded.len(), PAL_SIZE);
        let decoded = PalHeader::decode(&encoded);
        assert_eq!(pal, decoded);
    }

    #[test]
    fn test_encode_decode_roundtrip_catastrophic() {
        let mut pah = [0u8; PAH_SIZE];
        pah[0] = 0xAA;
        pah[13] = 0xFF;
        let mut sig = [0u8; SIG_SIZE];
        sig[0] = 0xDE;
        sig[7] = 0xAD;

        let pal = PalHeader::new(
            Modality::Executive,
            RiskLevel::Catastrophic,
            BodyStance::Moving,
            ProximityEdge::CriticalEdge,
            OutputDest::External,
            OverrideFlag::HardOverride,
            true,
            pah,
            sig,
        );

        let encoded = pal.encode();
        let decoded = PalHeader::decode(&encoded);

        assert_eq!(decoded.modality, Modality::Executive);
        assert_eq!(decoded.risk_level, RiskLevel::Catastrophic);
        assert_eq!(decoded.body_stance, BodyStance::Moving);
        assert_eq!(decoded.proximity_edge, ProximityEdge::CriticalEdge);
        assert_eq!(decoded.output_dest, OutputDest::External);
        assert_eq!(decoded.override_flag, OverrideFlag::HardOverride);
        assert_eq!(decoded.pal_version, PAL_VERSION);
        assert!(decoded.replay_enable);
        assert!(!decoded.reserved);
        assert_eq!(decoded.physical_context_hash, pah);
        assert_eq!(decoded.pah_signature, sig);
    }

    #[test]
    fn test_fixed_offset_byte0() {
        let pal = PalHeader::new(
            Modality::Render,       // bits 0-1 = 01
            RiskLevel::Critical,    // bits 2-3 = 10
            BodyStance::Standing,   // bits 4-5 = 01
            ProximityEdge::Warning, // bits 6-7 = 01
            OutputDest::Internal,
            OverrideFlag::Normal,
            true,
            [0u8; PAH_SIZE],
            [0u8; SIG_SIZE],
        );

        let encoded = pal.encode();
        // byte0 = Render(01) | Critical(10)<<2 | Standing(01)<<4 | Warning(01)<<6
        //       = 01 | 1000 | 010000 | 01000000
        //       = 01011001 = 0x59 = 89
        assert_eq!(encoded[0], 0b01011001);
        assert_eq!(encoded[0], 0x59);
    }

    #[test]
    fn test_fixed_offset_byte1() {
        let pal = PalHeader::new(
            Modality::Cognitive,
            RiskLevel::Low,
            BodyStance::Unknown,
            ProximityEdge::Safe,
            OutputDest::External,   // bit 0 = 1
            OverrideFlag::HardOverride, // bit 1 = 1
            false,                  // bit 6 = 0 (replay_enable)
            [0u8; PAH_SIZE],
            [0u8; SIG_SIZE],
        );

        let encoded = pal.encode();
        // byte1: bit0=1 (External), bit1=1 (HardOverride), bits2-5=PAL_VERSION=0001 (bit2=1), bit6=0, bit7=0
        // = 1 | 2 | 4 | 0 | 0 = 7 = 0x07 = 00000111
        assert_eq!(encoded[1], 0b00000111);
        assert_eq!(encoded[1], 0x07);
    }

    #[test]
    fn test_pah_and_signature_offsets() {
        let mut pah = [0u8; PAH_SIZE];
        pah[0] = 0x11;
        pah[PAH_SIZE - 1] = 0x22;
        let mut sig = [0u8; SIG_SIZE];
        sig[0] = 0x33;
        sig[SIG_SIZE - 1] = 0x44;

        let pal = PalHeader::new(
            Modality::Cognitive,
            RiskLevel::Low,
            BodyStance::Unknown,
            ProximityEdge::Safe,
            OutputDest::Internal,
            OverrideFlag::Normal,
            true,
            pah,
            sig,
        );

        let encoded = pal.encode();
        // PAH starts at byte 2
        assert_eq!(encoded[2], 0x11);
        assert_eq!(encoded[2 + PAH_SIZE - 1], 0x22);
        // Signature starts at byte 2 + PAH_SIZE = 16
        assert_eq!(encoded[2 + PAH_SIZE], 0x33);
        assert_eq!(encoded[PAL_SIZE - 1], 0x44);
    }

    #[test]
    fn test_catastrophic_override_detection() {
        let pal_normal = PalHeader::default();
        assert!(!pal_normal.is_catastrophic_override());

        let pal_cat = PalHeader::new(
            Modality::Executive,
            RiskLevel::Catastrophic,
            BodyStance::Moving,
            ProximityEdge::CriticalEdge,
            OutputDest::External,
            OverrideFlag::HardOverride,
            true,
            [0u8; PAH_SIZE],
            [0u8; SIG_SIZE],
        );
        assert!(pal_cat.is_catastrophic_override());

        // Catastrophic risk but no override flag → not hard override
        let pal_cat_no_override = PalHeader::new(
            Modality::Executive,
            RiskLevel::Catastrophic,
            BodyStance::Moving,
            ProximityEdge::CriticalEdge,
            OutputDest::External,
            OverrideFlag::Normal,
            true,
            [0u8; PAH_SIZE],
            [0u8; SIG_SIZE],
        );
        assert!(!pal_cat_no_override.is_catastrophic_override());
    }

    #[test]
    fn test_replay_disabled_forces_medium_risk() {
        // Replay-Enable=1 → 保持原始风险等级
        let pal_replay_on = PalHeader::new(
            Modality::Executive,
            RiskLevel::Catastrophic,
            BodyStance::Moving,
            ProximityEdge::CriticalEdge,
            OutputDest::External,
            OverrideFlag::HardOverride,
            true, // replay_enable
            [0u8; PAH_SIZE],
            [0u8; SIG_SIZE],
        );
        assert_eq!(pal_replay_on.effective_risk_level(), RiskLevel::Catastrophic);

        // Replay-Enable=0 → 强制降级至 MEDIUM（规则 6）
        let pal_replay_off = PalHeader::new(
            Modality::Executive,
            RiskLevel::Catastrophic,
            BodyStance::Moving,
            ProximityEdge::CriticalEdge,
            OutputDest::External,
            OverrideFlag::HardOverride,
            false, // replay_enable = 0
            [0u8; PAH_SIZE],
            [0u8; SIG_SIZE],
        );
        assert_eq!(pal_replay_off.effective_risk_level(), RiskLevel::Medium);
        // 原始风险等级仍保留在 risk_level 字段
        assert_eq!(pal_replay_off.risk_level, RiskLevel::Catastrophic);
    }

    #[test]
    fn test_reserved_bit_forced_zero_on_encode() {
        let pal = PalHeader { reserved: true, ..Default::default() };
        let encoded = pal.encode();
        // 编码时 Reserved 位（bit 7 of byte1）强制为 0
        assert_eq!(encoded[1] & (1 << 7), 0);

        // 解码时如果输入有 Reserved=1，应该能检测到
        let mut buf = pal.encode();
        buf[1] |= 1 << 7; // 手动设置 Reserved=1
        let decoded = PalHeader::decode(&buf);
        assert!(decoded.has_unknown_reserved());
    }

    #[test]
    fn test_pal_version_field() {
        let pal = PalHeader::default();
        assert_eq!(pal.pal_version, PAL_VERSION);
        assert!(pal.is_version_current());

        let encoded = pal.encode();
        let decoded = PalHeader::decode(&encoded);
        assert_eq!(decoded.pal_version, PAL_VERSION);
        assert!(decoded.is_version_current());
    }

    #[test]
    fn test_all_enum_values_roundtrip() {
        for modality in [Modality::Cognitive, Modality::Render, Modality::Executive, Modality::SensorFeed] {
            for risk in [RiskLevel::Low, RiskLevel::Medium, RiskLevel::Critical, RiskLevel::Catastrophic] {
                for stance in [BodyStance::Seated, BodyStance::Standing, BodyStance::Moving, BodyStance::Unknown] {
                    for edge in [ProximityEdge::Safe, ProximityEdge::Warning, ProximityEdge::Danger, ProximityEdge::CriticalEdge] {
                        let pal = PalHeader::new(
                            modality, risk, stance, edge,
                            OutputDest::External, OverrideFlag::HardOverride,
                            true, [0xAB; PAH_SIZE], [0xCD; SIG_SIZE],
                        );
                        let encoded = pal.encode();
                        let decoded = PalHeader::decode(&encoded);
                        assert_eq!(decoded.modality, modality);
                        assert_eq!(decoded.risk_level, risk);
                        assert_eq!(decoded.body_stance, stance);
                        assert_eq!(decoded.proximity_edge, edge);
                    }
                }
            }
        }
    }
}
