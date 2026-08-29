//! PAH 第一层 64-bit 签名验证（ed25519 软件实现）
//!
//! CI-144 v2.0 PAH 双层安全架构：
//! - **第一层（快速校验，Tuck 硬实时）**：64-bit ECC 截断签名，放在 SAP 头部
//! - **第二层（完整验证，载荷解密后异步）**：512-bit Ed25519 全量签名，放在 INTENT-7 载荷头部扩展区（ADR-0001）
//!
//! 64-bit 截断算法：完整 Ed25519 签名（64 字节）的 SHA-256 哈希值前 64 位（MSB，8 字节）。
//! 跨实现必须一致。
//!
//! 规范依据：规则 5（PAH 强制验证）
//! ADR：ADR-0001（PAH 第二层签名位置）、ADR-0002（验证失败处理）

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};

/// 64-bit 截断签名长度（字节）
pub const TRUNCATED_SIG_SIZE: usize = 8;

/// 完整 Ed25519 签名长度（字节）
pub const FULL_SIG_SIZE: usize = 64;

// ─── 密钥对 ─────────────────────────────────────────────────

/// Ed25519 密钥对（用于签名生成和验证）
#[derive(Clone)]
pub struct KeyPair {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}

impl KeyPair {
    /// 生成新的密钥对（使用 OS 随机数生成器）
    pub fn generate() -> Self {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        Self {
            signing_key,
            verifying_key,
        }
    }

    /// 从私钥字节创建密钥对（32 字节种子）
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(seed);
        let verifying_key = signing_key.verifying_key();
        Self {
            signing_key,
            verifying_key,
        }
    }

    /// 获取公钥（验证密钥，32 字节）
    pub fn public_key(&self) -> [u8; 32] {
        self.verifying_key.to_bytes()
    }

    /// 获取私钥种子（32 字节，用于持久化）
    pub fn seed(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }
}

// ─── 签名生成 ───────────────────────────────────────────────

/// 对消息生成完整 Ed25519 签名（64 字节）
pub fn sign(keypair: &KeyPair, message: &[u8]) -> [u8; FULL_SIG_SIZE] {
    let signature = keypair.signing_key.sign(message);
    signature.to_bytes()
}

/// 对消息生成 64-bit 截断签名（8 字节）
///
/// 截断算法：完整签名的 SHA-256 哈希值前 64 位（MSB）
pub fn sign_truncated(keypair: &KeyPair, message: &[u8]) -> [u8; TRUNCATED_SIG_SIZE] {
    let full_sig = sign(keypair, message);
    truncate_signature(&full_sig)
}

// ─── 签名验证 ───────────────────────────────────────────────

/// 验证完整 Ed25519 签名（64 字节）
pub fn verify(public_key: &[u8; 32], message: &[u8], signature: &[u8; FULL_SIG_SIZE]) -> bool {
    let verifying_key = match VerifyingKey::from_bytes(public_key) {
        Ok(k) => k,
        Err(_) => return false,
    };
    let sig = Signature::from_bytes(signature);
    verifying_key.verify(message, &sig).is_ok()
}

/// 验证 64-bit 截断签名（8 字节）
///
/// 注意：截断签名只能提供基础防伪，无法抵抗暴力碰撞。
/// 第二层完整验证（512-bit）在载荷解密后异步执行（ADR-0002）。
pub fn verify_truncated(
    public_key: &[u8; 32],
    message: &[u8],
    truncated_sig: &[u8; TRUNCATED_SIG_SIZE],
) -> bool {
    // 重新生成完整签名并截断，然后比较
    // 注意：这需要私钥，不适用于纯验证场景。
    // 正确的做法是：发送方同时发送完整签名（第二层），接收方用完整签名验证。
    // 第一层截断签名仅用于快速拒绝（如果截断签名不匹配，直接拒绝）。
    // 但截断签名匹配不代表完整签名匹配（可能碰撞）。
    //
    // 因此，第一层验证的正确逻辑是：
    // 1. 接收方有完整签名（在载荷中，第二层）
    // 2. 接收方计算完整签名的截断值
    // 3. 比较截断值是否与 SAP 中的 PAH-Signature 匹配
    // 4. 如果不匹配，直接拒绝（快速路径）
    // 5. 如果匹配，继续第二层完整验证（异步）
    //
    // 这个函数实现的是步骤 2-3：计算完整签名的截断值并比较。
    // 但需要完整签名作为输入。
    //
    // 对于纯第一层验证（只有截断签名，没有完整签名），无法验证。
    // 这是设计上的权衡：第一层是快速拒绝，不是完整验证。

    // 此函数保留用于未来扩展（如使用零知识证明验证截断签名）。
    // 当前实现：无法仅用截断签名验证，返回 false 表示需要完整签名。
    let _ = (public_key, message, truncated_sig);
    false
}

/// 计算完整签名的 64-bit 截断值（用于第一层快速校验）
///
/// 截断算法：完整 Ed25519 签名（64 字节）的 SHA-256 哈希值前 64 位（MSB，8 字节）
pub fn truncate_signature(full_signature: &[u8; FULL_SIG_SIZE]) -> [u8; TRUNCATED_SIG_SIZE] {
    let mut hasher = Sha256::new();
    hasher.update(full_signature);
    let hash = hasher.finalize();
    let mut truncated = [0u8; TRUNCATED_SIG_SIZE];
    truncated.copy_from_slice(&hash[..TRUNCATED_SIG_SIZE]);
    truncated
}

/// 验证完整签名的截断值是否与预期匹配（第一层快速拒绝）
///
/// 流程：
/// 1. 接收方从载荷中获取完整签名（第二层）
/// 2. 计算完整签名的截断值
/// 3. 与 SAP 中的 PAH-Signature 比较
/// 4. 不匹配 → 直接拒绝（快速路径，无需完整验证）
/// 5. 匹配 → 继续第二层完整验证
pub fn verify_truncated_match(
    full_signature: &[u8; FULL_SIG_SIZE],
    expected_truncated: &[u8; TRUNCATED_SIG_SIZE],
) -> bool {
    let actual = truncate_signature(full_signature);
    actual == *expected_truncated
}

// ─── PAH 哈希计算 ───────────────────────────────────────────

/// 计算物理上下文哈希（PAH-Hash，112 bits = 14 字节）
///
/// PAH-Hash = SHA-256(物理上下文数据) 的高 112 位（前 14 字节）
///
/// 物理上下文数据包括：传感器读数、姿态、临边等物理事实。
/// 由发送方的硬件/传感器层生成，AI 无权修改。
pub fn compute_pah_hash(physical_context: &[u8]) -> [u8; 14] {
    let mut hasher = Sha256::new();
    hasher.update(physical_context);
    let hash = hasher.finalize();
    let mut pah = [0u8; 14];
    pah.copy_from_slice(&hash[..14]);
    pah
}

// ─── 单元测试 ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let kp1 = KeyPair::generate();
        let kp2 = KeyPair::generate();
        assert_ne!(kp1.public_key(), kp2.public_key());
        assert_eq!(kp1.public_key().len(), 32);
        assert_eq!(kp1.seed().len(), 32);
    }

    #[test]
    fn test_keypair_from_seed() {
        let seed = [42u8; 32];
        let kp1 = KeyPair::from_seed(&seed);
        let kp2 = KeyPair::from_seed(&seed);
        assert_eq!(kp1.public_key(), kp2.public_key());
        assert_eq!(kp1.seed(), seed);
    }

    #[test]
    fn test_sign_and_verify_full() {
        let kp = KeyPair::generate();
        let message = b"Hello, CI-144 v2.0!";
        let signature = sign(&kp, message);

        assert_eq!(signature.len(), FULL_SIG_SIZE);
        assert!(verify(&kp.public_key(), message, &signature));

        // 篡改消息 → 验证失败
        let tampered = b"Hello, CI-144 v2.0?!";
        assert!(!verify(&kp.public_key(), tampered, &signature));
    }

    #[test]
    fn test_sign_and_verify_wrong_key() {
        let kp1 = KeyPair::generate();
        let kp2 = KeyPair::generate();
        let message = b"test message";
        let signature = sign(&kp1, message);

        assert!(verify(&kp1.public_key(), message, &signature));
        assert!(!verify(&kp2.public_key(), message, &signature));
    }

    #[test]
    fn test_truncate_signature_deterministic() {
        let sig = [0xABu8; FULL_SIG_SIZE];
        let t1 = truncate_signature(&sig);
        let t2 = truncate_signature(&sig);
        assert_eq!(t1, t2);
        assert_eq!(t1.len(), TRUNCATED_SIG_SIZE);
    }

    #[test]
    fn test_truncate_signature_different_inputs() {
        let sig1 = [0xAAu8; FULL_SIG_SIZE];
        let sig2 = [0xBBu8; FULL_SIG_SIZE];
        assert_ne!(truncate_signature(&sig1), truncate_signature(&sig2));
    }

    #[test]
    fn test_sign_truncated() {
        let kp = KeyPair::generate();
        let message = b"truncated test";
        let full_sig = sign(&kp, message);
        let truncated = sign_truncated(&kp, message);

        assert_eq!(truncated.len(), TRUNCATED_SIG_SIZE);
        assert_eq!(truncated, truncate_signature(&full_sig));
    }

    #[test]
    fn test_verify_truncated_match() {
        let kp = KeyPair::generate();
        let message = b"match test";
        let full_sig = sign(&kp, message);
        let truncated = truncate_signature(&full_sig);

        assert!(verify_truncated_match(&full_sig, &truncated));

        // 错误的截断值 → 不匹配
        let wrong = [0xFFu8; TRUNCATED_SIG_SIZE];
        assert!(!verify_truncated_match(&full_sig, &wrong));
    }

    #[test]
    fn test_compute_pah_hash() {
        let context1 = b"sensor data: temperature=25.5, humidity=60%";
        let context2 = b"sensor data: temperature=25.6, humidity=60%";

        let hash1 = compute_pah_hash(context1);
        let hash2 = compute_pah_hash(context2);

        assert_eq!(hash1.len(), 14);
        assert_ne!(hash1, hash2); // 微小差异 → 不同哈希

        // 确定性
        let hash1_again = compute_pah_hash(context1);
        assert_eq!(hash1, hash1_again);
    }

    #[test]
    fn test_full_pipeline() {
        // 模拟完整的 PAH 签名流程
        let kp = KeyPair::generate();

        // 1. 传感器生成物理上下文
        let physical_context = b"modality=executive, risk=critical, stance=moving, edge=danger";

        // 2. 计算 PAH-Hash（112 bits）
        let pah_hash = compute_pah_hash(physical_context);
        assert_eq!(pah_hash.len(), 14);

        // 3. 对 PAH-Hash 签名（完整签名）
        let full_sig = sign(&kp, &pah_hash);
        assert_eq!(full_sig.len(), FULL_SIG_SIZE);

        // 4. 计算截断签名（64 bits，第一层快速校验）
        let truncated_sig = truncate_signature(&full_sig);
        assert_eq!(truncated_sig.len(), TRUNCATED_SIG_SIZE);

        // 5. 验证完整签名（第二层）
        assert!(verify(&kp.public_key(), &pah_hash, &full_sig));

        // 6. 验证截断匹配（第一层快速拒绝）
        assert!(verify_truncated_match(&full_sig, &truncated_sig));

        // 7. 篡改 PAH-Hash → 完整签名验证失败
        let tampered_hash = [0xFFu8; 14];
        assert!(!verify(&kp.public_key(), &tampered_hash, &full_sig));
    }

    #[test]
    fn test_constants() {
        assert_eq!(TRUNCATED_SIG_SIZE, 8);
        assert_eq!(FULL_SIG_SIZE, 64);
    }
}
