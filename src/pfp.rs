//! PFP-xCF14 — Physical Feature Protocol（物理特征协议，冻结层）
//!
//! PFP 是 CI-144 协议家族的物理特征层，提供明文、固定偏移、可被 Tuck 硬实时读取的物理元数据。
//! 总长度 4 字节（32 bits），Tuck 只读这 4 字节做硬实时决策。
//!
//! **冻结策略**：PFP-xCF14 一旦定稿，永远不变。任何修改必须产生新版本（如 PFP-xCF15）。
//!
//! 规范依据：docs/v2.0-upgrade-plan.md（协议家族架构）
//! ADR：ADR-0001（PAH 第二层签名位置）

/// PFP 总长度（字节）
pub const PFP_SIZE: usize = 4;

/// CI-144 家族魔数（2 字节，大端序 = 0xCF14）
pub const FAMILY_MAGIC: u16 = 0xCF14;

/// PFP 子协议 ID
pub const PFP_PROTOCOL_ID: u8 = 0x00;

/// PFP 当前版本（由魔数隐式标识，xCF14 即版本锚点）
pub const PFP_VERSION: u8 = 0x01;

// ─── 枚举类型 ───────────────────────────────────────────────

/// 操作模态（PFP Byte2 bit 0-1）
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

/// 风险等级（PFP Byte2 bit 2-3）
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

/// 本体姿态（PFP Byte2 bit 4-5）
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

/// 临边/高危环境（PFP Byte2 bit 6-7）
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

/// 输出目的地（PFP Byte3 bit 0）
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

/// 硬覆盖标志（PFP Byte3 bit 1）
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

// ─── PFP 结构体 ─────────────────────────────────────────────

/// PFP-xCF14 — Physical Feature Protocol（4 字节固定偏移头部）
///
/// 内存布局（大端序，网络字节序）：
/// ```text
/// Byte 0-1: Family-Magic（16 bits）= 0xCF14
/// Byte 2:   物理特征数据（8 bits）
///   bit 0-1: Modality
///   bit 2-3: Risk-Level
///   bit 4-5: Body-Stance
///   bit 6-7: Proximity-Edge
/// Byte 3:   控制标志（8 bits）
///   bit 0:   Output-Dest
///   bit 1:   Override-Flag
///   bit 2:   Replay-Enable
///   bit 3-7: Reserved（强制 0）
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PfpHeader {
    pub modality: Modality,
    pub risk_level: RiskLevel,
    pub body_stance: BodyStance,
    pub proximity_edge: ProximityEdge,
    pub output_dest: OutputDest,
    pub override_flag: OverrideFlag,
    pub replay_enable: bool,
    /// Reserved 位（bit 3-7 of Byte3），强制为 0。非零值触发版本协商流程。
    pub reserved: u8,
}

impl PfpHeader {
    /// 创建新的 PFP 头部（Reserved 强制为 0）
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        modality: Modality,
        risk_level: RiskLevel,
        body_stance: BodyStance,
        proximity_edge: ProximityEdge,
        output_dest: OutputDest,
        override_flag: OverrideFlag,
        replay_enable: bool,
    ) -> Self {
        Self {
            modality,
            risk_level,
            body_stance,
            proximity_edge,
            output_dest,
            override_flag,
            replay_enable,
            reserved: 0,
        }
    }

    /// 编码为 4 字节大端序数组（固定偏移，零拷贝友好）
    pub fn encode(&self) -> [u8; PFP_SIZE] {
        let mut buf = [0u8; PFP_SIZE];

        // Byte 0-1: Family-Magic = 0xCF14（大端序）
        buf[0] = (FAMILY_MAGIC >> 8) as u8;
        buf[1] = (FAMILY_MAGIC & 0xFF) as u8;

        // Byte 2: 物理特征数据
        buf[2] = (self.modality.to_bits() & 0b11)
            | ((self.risk_level.to_bits() & 0b11) << 2)
            | ((self.body_stance.to_bits() & 0b11) << 4)
            | ((self.proximity_edge.to_bits() & 0b11) << 6);

        // Byte 3: 控制标志
        let mut byte3: u8 = 0;
        if self.output_dest.to_bit() { byte3 |= 1 << 0; }
        if self.override_flag.to_bit() { byte3 |= 1 << 1; }
        if self.replay_enable { byte3 |= 1 << 2; }
        // bit 3-7: Reserved（强制为 0，不设置）
        buf[3] = byte3;

        buf
    }

    /// 从 4 字节大端序数组解码（固定偏移读取，零分配）
    ///
    /// 注意：调用方应先验证 Family-Magic == 0xCF14，再调用此方法。
    pub fn decode(buf: &[u8; PFP_SIZE]) -> Self {
        let byte2 = buf[2];
        let byte3 = buf[3];

        let modality = Modality::from_bits(byte2 & 0b11);
        let risk_level = RiskLevel::from_bits((byte2 >> 2) & 0b11);
        let body_stance = BodyStance::from_bits((byte2 >> 4) & 0b11);
        let proximity_edge = ProximityEdge::from_bits((byte2 >> 6) & 0b11);

        let output_dest = OutputDest::from_bit((byte3 & (1 << 0)) != 0);
        let override_flag = OverrideFlag::from_bit((byte3 & (1 << 1)) != 0);
        let replay_enable = (byte3 & (1 << 2)) != 0;
        let reserved = (byte3 >> 3) & 0b11111;

        Self {
            modality,
            risk_level,
            body_stance,
            proximity_edge,
            output_dest,
            override_flag,
            replay_enable,
            reserved,
        }
    }

    /// 验证 Family-Magic 是否为 0xCF14
    pub fn verify_magic(buf: &[u8; PFP_SIZE]) -> bool {
        u16::from_be_bytes([buf[0], buf[1]]) == FAMILY_MAGIC
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
        self.reserved != 0
    }
}

impl Default for PfpHeader {
    fn default() -> Self {
        Self::new(
            Modality::Cognitive,
            RiskLevel::Low,
            BodyStance::Unknown,
            ProximityEdge::Safe,
            OutputDest::Internal,
            OverrideFlag::Normal,
            true, // replay_enable 默认开启
        )
    }
}

// ─── 单元测试 ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pfp_size_constants() {
        assert_eq!(PFP_SIZE, 4);
        assert_eq!(FAMILY_MAGIC, 0xCF14);
        assert_eq!(PFP_PROTOCOL_ID, 0x00);
    }

    #[test]
    fn test_family_magic_encoding() {
        let pfp = PfpHeader::default();
        let encoded = pfp.encode();
        assert_eq!(encoded[0], 0xCF);
        assert_eq!(encoded[1], 0x14);
        assert!(PfpHeader::verify_magic(&encoded));
    }

    #[test]
    fn test_encode_decode_roundtrip_default() {
        let pfp = PfpHeader::default();
        let encoded = pfp.encode();
        assert_eq!(encoded.len(), PFP_SIZE);
        let decoded = PfpHeader::decode(&encoded);
        assert_eq!(pfp, decoded);
    }

    #[test]
    fn test_encode_decode_roundtrip_catastrophic() {
        let pfp = PfpHeader::new(
            Modality::Executive,
            RiskLevel::Catastrophic,
            BodyStance::Moving,
            ProximityEdge::CriticalEdge,
            OutputDest::External,
            OverrideFlag::HardOverride,
            true,
        );

        let encoded = pfp.encode();
        let decoded = PfpHeader::decode(&encoded);

        assert_eq!(decoded.modality, Modality::Executive);
        assert_eq!(decoded.risk_level, RiskLevel::Catastrophic);
        assert_eq!(decoded.body_stance, BodyStance::Moving);
        assert_eq!(decoded.proximity_edge, ProximityEdge::CriticalEdge);
        assert_eq!(decoded.output_dest, OutputDest::External);
        assert_eq!(decoded.override_flag, OverrideFlag::HardOverride);
        assert!(decoded.replay_enable);
        assert_eq!(decoded.reserved, 0);
    }

    #[test]
    fn test_fixed_offset_byte2() {
        let pfp = PfpHeader::new(
            Modality::Render,       // bits 0-1 = 01
            RiskLevel::Critical,    // bits 2-3 = 10
            BodyStance::Standing,   // bits 4-5 = 01
            ProximityEdge::Warning, // bits 6-7 = 01
            OutputDest::Internal,
            OverrideFlag::Normal,
            true,
        );

        let encoded = pfp.encode();
        // byte2 = 01 | 10<<2 | 01<<4 | 01<<6
        //       = 01 | 1000 | 010000 | 01000000
        //       = 01011001 = 0x59 = 89
        assert_eq!(encoded[2], 0b01011001);
        assert_eq!(encoded[2], 0x59);
    }

    #[test]
    fn test_fixed_offset_byte3() {
        let pfp = PfpHeader::new(
            Modality::Cognitive,
            RiskLevel::Low,
            BodyStance::Unknown,
            ProximityEdge::Safe,
            OutputDest::External,    // bit 0 = 1
            OverrideFlag::HardOverride, // bit 1 = 1
            false,                   // bit 2 = 0 (replay_enable)
        );

        let encoded = pfp.encode();
        // byte3: bit0=1, bit1=1, bit2=0, bit3-7=0
        // = 00000011 = 0x03 = 3
        assert_eq!(encoded[3], 0b00000011);
        assert_eq!(encoded[3], 0x03);
    }

    #[test]
    fn test_catastrophic_override_detection() {
        let pfp_normal = PfpHeader::default();
        assert!(!pfp_normal.is_catastrophic_override());

        let pfp_cat = PfpHeader::new(
            Modality::Executive,
            RiskLevel::Catastrophic,
            BodyStance::Moving,
            ProximityEdge::CriticalEdge,
            OutputDest::External,
            OverrideFlag::HardOverride,
            true,
        );
        assert!(pfp_cat.is_catastrophic_override());

        // Catastrophic risk but no override flag → not hard override
        // (see test_catastrophic_no_override)
    }

    #[test]
    fn test_catastrophic_no_override() {
        let pfp_cat_no_override = PfpHeader::new(
            Modality::Executive,
            RiskLevel::Catastrophic,
            BodyStance::Moving,
            ProximityEdge::CriticalEdge,
            OutputDest::External,
            OverrideFlag::Normal,
            true,
        );
        assert!(!pfp_cat_no_override.is_catastrophic_override());
    }

    #[test]
    fn test_replay_disabled_forces_medium_risk() {
        // Replay-Enable=1 → 保持原始风险等级
        let pfp_replay_on = PfpHeader::new(
            Modality::Executive,
            RiskLevel::Catastrophic,
            BodyStance::Moving,
            ProximityEdge::CriticalEdge,
            OutputDest::External,
            OverrideFlag::HardOverride,
            true,
        );
        assert_eq!(pfp_replay_on.effective_risk_level(), RiskLevel::Catastrophic);

        // Replay-Enable=0 → 强制降级至 MEDIUM（规则 6）
        let pfp_replay_off = PfpHeader::new(
            Modality::Executive,
            RiskLevel::Catastrophic,
            BodyStance::Moving,
            ProximityEdge::CriticalEdge,
            OutputDest::External,
            OverrideFlag::HardOverride,
            false,
        );
        assert_eq!(pfp_replay_off.effective_risk_level(), RiskLevel::Medium);
        // 原始风险等级仍保留在 risk_level 字段
        assert_eq!(pfp_replay_off.risk_level, RiskLevel::Catastrophic);
    }

    #[test]
    fn test_reserved_bits_forced_zero_on_encode() {
        let pfp = PfpHeader { reserved: 0b11111, ..Default::default() };
        let encoded = pfp.encode();
        // 编码时 Reserved 位（bit 3-7 of byte3）强制为 0
        assert_eq!(encoded[3] & 0b11111000, 0);

        // 解码时如果输入有 Reserved 非零，应该能检测到
        let mut buf = pfp.encode();
        buf[3] |= 0b11111000; // 手动设置 Reserved=非零
        let decoded = PfpHeader::decode(&buf);
        assert!(decoded.has_unknown_reserved());
        assert_eq!(decoded.reserved, 0b11111);
    }

    #[test]
    fn test_all_enum_values_roundtrip() {
        for modality in [Modality::Cognitive, Modality::Render, Modality::Executive, Modality::SensorFeed] {
            for risk in [RiskLevel::Low, RiskLevel::Medium, RiskLevel::Critical, RiskLevel::Catastrophic] {
                for stance in [BodyStance::Seated, BodyStance::Standing, BodyStance::Moving, BodyStance::Unknown] {
                    for edge in [ProximityEdge::Safe, ProximityEdge::Warning, ProximityEdge::Danger, ProximityEdge::CriticalEdge] {
                        let pfp = PfpHeader::new(
                            modality, risk, stance, edge,
                            OutputDest::External, OverrideFlag::HardOverride,
                            true,
                        );
                        let encoded = pfp.encode();
                        let decoded = PfpHeader::decode(&encoded);
                        assert_eq!(decoded.modality, modality);
                        assert_eq!(decoded.risk_level, risk);
                        assert_eq!(decoded.body_stance, stance);
                        assert_eq!(decoded.proximity_edge, edge);
                    }
                }
            }
        }
    }

    #[test]
    fn test_magic_verification() {
        let pfp = PfpHeader::default();
        let encoded = pfp.encode();
        assert!(PfpHeader::verify_magic(&encoded));

        // 篡改魔数
        let mut bad = encoded;
        bad[0] = 0x00;
        assert!(!PfpHeader::verify_magic(&bad));
    }
}
