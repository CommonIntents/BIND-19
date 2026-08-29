//! SAP-xCF14 — Security Attestation Protocol（安全证明协议，演进层）
//!
//! SAP 是 CI-144 协议家族的安全证明层，提供防重放、完整性校验和身份认证。
//! 总长度 28 字节，按需加载（低安全场景可跳过 SAP，仅发送 PFP）。
//!
//! **演进策略**：SAP-xCF14 独立演进，v1、v2 可并行存在。PFP 冻结，SAP 升级。
//!
//! 规范依据：docs/v2.0-upgrade-plan.md（协议家族架构）
//! ADR：ADR-0004（KEY_ROTATION 帧格式）、ADR-0005（ACK 超时机制）

/// SAP 总长度（字节）
pub const SAP_SIZE: usize = 28;

/// SAP 子协议 ID
pub const SAP_PROTOCOL_ID: u8 = 0x01;

/// SAP 当前版本（v1 = 0001）
pub const SAP_VERSION: u8 = 0b0001;

/// Physical-Context-Hash 长度（字节）= 112 bits
pub const PAH_SIZE: usize = 14;

/// PAH-Signature 长度（字节）= 64 bits（第一层快速校验）
pub const SIG_SIZE: usize = 8;

/// Seq-Counter 回绕阈值（≥ 此值触发密钥轮换）
pub const SEQ_ROTATION_THRESHOLD: u16 = 65534;

// ─── SAP 结构体 ─────────────────────────────────────────────

/// SAP-xCF14 — Security Attestation Protocol（28 字节固定偏移结构）
///
/// 内存布局（大端序，网络字节序）：
/// ```text
/// Byte 0-1:   Family-Magic（16 bits）= 0xCF14
/// Byte 2:     Protocol-ID（8 bits）= 0x01（SAP-xCF14）
/// Byte 3:     版本与保留（8 bits）
///   bit 0-3:   SAP-Version（当前 v1 = 0001）
///   bit 4-7:   Reserved（全 0）
/// Byte 4-5:   Seq-Counter（16 bits，大端序）
///   防重放，单调递增，回绕阈值 65534 触发密钥轮换
/// Byte 6-19:  PAH-Hash（112 bits = 14 bytes）
///   SHA-256 截断（高 112 位），物理上下文哈希锁定
/// Byte 20-27: PAH-Signature（64 bits = 8 bytes）
///   ECC 签名截断（第一层快速校验）= SHA-256(完整签名) 前 64 位
/// ```
///
/// **注意**：第二层 512-bit 完整签名放在 INTENT-7 载荷头部扩展区（ADR-0001），不在 SAP 中。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SapHeader {
    pub sap_version: u8,
    /// Seq-Counter（16 bits，防重放）
    pub seq_counter: u16,
    /// Physical-Context-Hash（14 bytes，SHA-256 截断高 112 位）
    pub pah_hash: [u8; PAH_SIZE],
    /// PAH-Signature（8 bytes，第一层快速校验）
    pub pah_signature: [u8; SIG_SIZE],
}

impl SapHeader {
    /// 创建新的 SAP 头部（默认 v1）
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        seq_counter: u16,
        pah_hash: [u8; PAH_SIZE],
        pah_signature: [u8; SIG_SIZE],
    ) -> Self {
        Self {
            sap_version: SAP_VERSION,
            seq_counter,
            pah_hash,
            pah_signature,
        }
    }

    /// 编码为 28 字节大端序数组（固定偏移，零拷贝友好）
    pub fn encode(&self) -> [u8; SAP_SIZE] {
        let mut buf = [0u8; SAP_SIZE];

        // Byte 0-1: Family-Magic = 0xCF14（大端序）
        buf[0] = (crate::pfp::FAMILY_MAGIC >> 8) as u8;
        buf[1] = (crate::pfp::FAMILY_MAGIC & 0xFF) as u8;

        // Byte 2: Protocol-ID = 0x01
        buf[2] = SAP_PROTOCOL_ID;

        // Byte 3: SAP-Version（bit 0-3）+ Reserved（bit 4-7，强制 0）
        buf[3] = self.sap_version & 0b1111;

        // Byte 4-5: Seq-Counter（大端序）
        buf[4] = (self.seq_counter >> 8) as u8;
        buf[5] = (self.seq_counter & 0xFF) as u8;

        // Byte 6-19: PAH-Hash（14 bytes）
        buf[6..6 + PAH_SIZE].copy_from_slice(&self.pah_hash);

        // Byte 20-27: PAH-Signature（8 bytes）
        buf[6 + PAH_SIZE..6 + PAH_SIZE + SIG_SIZE].copy_from_slice(&self.pah_signature);

        buf
    }

    /// 从 28 字节大端序数组解码（固定偏移读取，零分配）
    ///
    /// 注意：调用方应先验证 Family-Magic == 0xCF14 和 Protocol-ID == 0x01，再调用此方法。
    pub fn decode(buf: &[u8; SAP_SIZE]) -> Self {
        let sap_version = buf[3] & 0b1111;
        let seq_counter = u16::from_be_bytes([buf[4], buf[5]]);

        let mut pah_hash = [0u8; PAH_SIZE];
        pah_hash.copy_from_slice(&buf[6..6 + PAH_SIZE]);

        let mut pah_signature = [0u8; SIG_SIZE];
        pah_signature.copy_from_slice(&buf[6 + PAH_SIZE..6 + PAH_SIZE + SIG_SIZE]);

        Self {
            sap_version,
            seq_counter,
            pah_hash,
            pah_signature,
        }
    }

    /// 验证 Family-Magic 是否为 0xCF14
    pub fn verify_magic(buf: &[u8; SAP_SIZE]) -> bool {
        u16::from_be_bytes([buf[0], buf[1]]) == crate::pfp::FAMILY_MAGIC
    }

    /// 验证 Protocol-ID 是否为 0x01（SAP）
    pub fn verify_protocol_id(buf: &[u8; SAP_SIZE]) -> bool {
        buf[2] == SAP_PROTOCOL_ID
    }

    /// 检查 SAP 版本是否匹配当前版本
    pub fn is_version_current(&self) -> bool {
        self.sap_version == SAP_VERSION
    }

    /// 检查 Seq-Counter 是否达到回绕阈值（需要触发密钥轮换）
    pub fn needs_key_rotation(&self) -> bool {
        self.seq_counter >= SEQ_ROTATION_THRESHOLD
    }

    /// 原子递增 Seq-Counter（返回新值）
    ///
    /// 注意：多线程环境下应使用 AtomicU16 + fetch_add(1, Ordering::SeqCst)。
    /// 此方法仅用于单线程场景或测试。
    pub fn increment_seq(&mut self) -> u16 {
        self.seq_counter = self.seq_counter.wrapping_add(1);
        self.seq_counter
    }
}

impl Default for SapHeader {
    fn default() -> Self {
        Self::new(0, [0u8; PAH_SIZE], [0u8; SIG_SIZE])
    }
}

// ─── 单元测试 ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sap_size_constants() {
        assert_eq!(SAP_SIZE, 28);
        assert_eq!(SAP_PROTOCOL_ID, 0x01);
        assert_eq!(SAP_VERSION, 0b0001);
        assert_eq!(PAH_SIZE, 14);
        assert_eq!(SIG_SIZE, 8);
        assert_eq!(2 + 1 + 1 + 2 + PAH_SIZE + SIG_SIZE, SAP_SIZE);
        assert_eq!(SEQ_ROTATION_THRESHOLD, 65534);
    }

    #[test]
    fn test_family_magic_and_protocol_id_encoding() {
        let sap = SapHeader::default();
        let encoded = sap.encode();
        // Family-Magic = 0xCF14
        assert_eq!(encoded[0], 0xCF);
        assert_eq!(encoded[1], 0x14);
        // Protocol-ID = 0x01
        assert_eq!(encoded[2], 0x01);
        assert!(SapHeader::verify_magic(&encoded));
        assert!(SapHeader::verify_protocol_id(&encoded));
    }

    #[test]
    fn test_encode_decode_roundtrip_default() {
        let sap = SapHeader::default();
        let encoded = sap.encode();
        assert_eq!(encoded.len(), SAP_SIZE);
        let decoded = SapHeader::decode(&encoded);
        assert_eq!(sap, decoded);
    }

    #[test]
    fn test_encode_decode_roundtrip_full() {
        let mut pah = [0u8; PAH_SIZE];
        pah[0] = 0xAA;
        pah[PAH_SIZE - 1] = 0xFF;
        let mut sig = [0u8; SIG_SIZE];
        sig[0] = 0xDE;
        sig[SIG_SIZE - 1] = 0xAD;

        let sap = SapHeader::new(12345, pah, sig);
        let encoded = sap.encode();
        let decoded = SapHeader::decode(&encoded);

        assert_eq!(decoded.sap_version, SAP_VERSION);
        assert!(decoded.is_version_current());
        assert_eq!(decoded.seq_counter, 12345);
        assert_eq!(decoded.pah_hash, pah);
        assert_eq!(decoded.pah_signature, sig);
    }

    #[test]
    fn test_fixed_offsets() {
        let mut pah = [0u8; PAH_SIZE];
        pah[0] = 0x11;
        pah[PAH_SIZE - 1] = 0x22;
        let mut sig = [0u8; SIG_SIZE];
        sig[0] = 0x33;
        sig[SIG_SIZE - 1] = 0x44;

        let sap = SapHeader::new(0x1234, pah, sig);
        let encoded = sap.encode();

        // Byte 0-1: Magic
        assert_eq!(&encoded[0..2], &[0xCF, 0x14]);
        // Byte 2: Protocol-ID
        assert_eq!(encoded[2], 0x01);
        // Byte 3: SAP-Version
        assert_eq!(encoded[3], SAP_VERSION);
        // Byte 4-5: Seq-Counter（大端序）
        assert_eq!(encoded[4], 0x12);
        assert_eq!(encoded[5], 0x34);
        // Byte 6-19: PAH-Hash
        assert_eq!(encoded[6], 0x11);
        assert_eq!(encoded[6 + PAH_SIZE - 1], 0x22);
        // Byte 20-27: PAH-Signature
        assert_eq!(encoded[6 + PAH_SIZE], 0x33);
        assert_eq!(encoded[SAP_SIZE - 1], 0x44);
    }

    #[test]
    fn test_seq_counter_increment() {
        let mut sap = SapHeader::default();
        assert_eq!(sap.seq_counter, 0);
        assert_eq!(sap.increment_seq(), 1);
        assert_eq!(sap.increment_seq(), 2);
        assert_eq!(sap.seq_counter, 2);
    }

    #[test]
    fn test_seq_counter_wrapping() {
        let mut sap = SapHeader::new(65535, [0u8; PAH_SIZE], [0u8; SIG_SIZE]);
        assert_eq!(sap.increment_seq(), 0); // wrapping add
    }

    #[test]
    fn test_key_rotation_threshold() {
        let sap_low = SapHeader::new(100, [0u8; PAH_SIZE], [0u8; SIG_SIZE]);
        assert!(!sap_low.needs_key_rotation());

        let sap_threshold = SapHeader::new(65534, [0u8; PAH_SIZE], [0u8; SIG_SIZE]);
        assert!(sap_threshold.needs_key_rotation());

        let sap_high = SapHeader::new(65535, [0u8; PAH_SIZE], [0u8; SIG_SIZE]);
        assert!(sap_high.needs_key_rotation());
    }

    #[test]
    fn test_version_field() {
        let sap = SapHeader::default();
        assert_eq!(sap.sap_version, SAP_VERSION);
        assert!(sap.is_version_current());

        // 模拟旧版本
        let mut old_sap = sap.clone();
        old_sap.sap_version = 0;
        assert!(!old_sap.is_version_current());
    }

    #[test]
    fn test_magic_and_protocol_id_verification() {
        let sap = SapHeader::default();
        let encoded = sap.encode();
        assert!(SapHeader::verify_magic(&encoded));
        assert!(SapHeader::verify_protocol_id(&encoded));

        // 篡改魔数
        let mut bad_magic = encoded;
        bad_magic[0] = 0x00;
        assert!(!SapHeader::verify_magic(&bad_magic));

        // 篡改协议 ID
        let mut bad_id = encoded;
        bad_id[2] = 0x00;
        assert!(!SapHeader::verify_protocol_id(&bad_id));
    }

    #[test]
    fn test_reserved_bits_forced_zero() {
        let sap = SapHeader::default();
        let encoded = sap.encode();
        // Byte 3 bit 4-7 (Reserved) 强制为 0
        assert_eq!(encoded[3] & 0b11110000, 0);
    }
}
