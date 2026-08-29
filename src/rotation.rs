//! 密钥轮换状态机（KEY_ROTATION 控制帧 + ACK 超时 fail-closed）
//!
//! CI-144 v2.0 规则 7：Seq-Counter 回绕时触发密钥轮换。
//! - 触发条件：Seq-Counter ≥ 65534（SEQ_ROTATION_THRESHOLD）
//! - 轮换帧：BIND-19 帧类型 0x07（KEY_ROTATION，ADR-0008 确认未被占用）
//! - 确认帧：BIND-19 帧类型 0x08（KEY_ROTATION_ACK）
//! - ACK 超时：100ms 重试，最多 3 次
//! - fail-closed：3 次失败后停止发送数据帧，进入安全状态，等待人工复位
//!
//! 规范依据：规则 7（密钥轮换流程）
//! ADR：ADR-0004（KEY_ROTATION 帧格式）、ADR-0005（ACK 超时机制）

use std::time::Duration;

/// Seq-Counter 回绕阈值（≥ 此值触发密钥轮换）
pub const ROTATION_THRESHOLD: u16 = 65534;

/// ACK 超时时间（100ms）
pub const ACK_TIMEOUT: Duration = Duration::from_millis(100);

/// 最大重试次数（3 次）
pub const MAX_RETRIES: u8 = 3;

/// KEY_ROTATION 帧载荷中的 nonce 长度（12 字节，AES-GCM nonce）
pub const NONCE_SIZE: usize = 12;

// ─── KEY_ROTATION 帧载荷 ────────────────────────────────────

/// KEY_ROTATION 控制帧载荷
///
/// 格式：`[nonce (12 bytes)] + [new_key_encrypted (variable)]`
///
/// - nonce：AES-GCM 随机数，防重放
/// - new_key_encrypted：新会话密钥，由主密钥 AES-256-GCM 加密保护
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyRotationPayload {
    /// AES-GCM nonce（12 字节）
    pub nonce: [u8; NONCE_SIZE],
    /// 新会话密钥（由主密钥加密，可变长）
    pub new_key_encrypted: Vec<u8>,
}

impl KeyRotationPayload {
    /// 创建新的 KEY_ROTATION 载荷
    pub fn new(nonce: [u8; NONCE_SIZE], new_key_encrypted: Vec<u8>) -> Self {
        Self {
            nonce,
            new_key_encrypted,
        }
    }

    /// 编码为字节向量
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(NONCE_SIZE + self.new_key_encrypted.len());
        buf.extend_from_slice(&self.nonce);
        buf.extend_from_slice(&self.new_key_encrypted);
        buf
    }

    /// 从字节切片解码
    pub fn decode(buf: &[u8]) -> Result<Self, RotationError> {
        if buf.len() < NONCE_SIZE {
            return Err(RotationError::PayloadTooShort);
        }
        let mut nonce = [0u8; NONCE_SIZE];
        nonce.copy_from_slice(&buf[..NONCE_SIZE]);
        let new_key_encrypted = buf[NONCE_SIZE..].to_vec();
        Ok(Self {
            nonce,
            new_key_encrypted,
        })
    }
}

// ─── 密钥轮换状态机 ─────────────────────────────────────────

/// 密钥轮换状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotationState {
    /// 正常状态（无轮换进行中）
    Idle,
    /// 已发送 KEY_ROTATION，等待 ACK
    Pending {
        /// 已重试次数（0 = 首次发送，1 = 第一次重试，...）
        retries: u8,
    },
    /// 密钥已轮换成功（收到 ACK）
    Rotated,
    /// 轮换失败（3 次重试失败，fail-closed）
    Failed,
}

/// 密钥轮换状态机
#[derive(Debug, Clone)]
pub struct KeyRotationStateMachine {
    /// 当前状态
    state: RotationState,
    /// 当前轮换的载荷（Pending 状态下用于重试）
    pending_payload: Option<KeyRotationPayload>,
}

impl KeyRotationStateMachine {
    /// 创建新的状态机（初始 Idle）
    pub fn new() -> Self {
        Self {
            state: RotationState::Idle,
            pending_payload: None,
        }
    }

    /// 获取当前状态
    pub fn state(&self) -> RotationState {
        self.state
    }

    /// 检查是否需要触发密钥轮换（基于 Seq-Counter）
    pub fn should_rotate(&self, seq_counter: u16) -> bool {
        // 仅在 Idle 状态下检查，避免重复触发
        self.state == RotationState::Idle && seq_counter >= ROTATION_THRESHOLD
    }

    /// 开始密钥轮换，返回需要发送的 KEY_ROTATION 载荷
    ///
    /// 调用方应将此载荷封装为 BIND-19 帧（FrameType::KeyRotation = 0x07）发送。
    pub fn start_rotation(
        &mut self,
        payload: KeyRotationPayload,
    ) -> Result<&KeyRotationPayload, RotationError> {
        if self.state != RotationState::Idle {
            return Err(RotationError::NotInIdleState);
        }
        self.pending_payload = Some(payload);
        self.state = RotationState::Pending { retries: 0 };
        Ok(self.pending_payload.as_ref().unwrap())
    }

    /// 处理 ACK 确认帧
    ///
    /// 收到 KEY_ROTATION_ACK（0x08）后调用，转换到 Rotated 状态。
    pub fn handle_ack(&mut self) -> Result<(), RotationError> {
        match self.state {
            RotationState::Pending { .. } => {
                self.state = RotationState::Rotated;
                self.pending_payload = None;
                Ok(())
            }
            _ => Err(RotationError::UnexpectedAck),
        }
    }

    /// 处理 ACK 超时
    ///
    /// ACK 超时（100ms）后调用。如果重试次数 < MAX_RETRIES，返回需要重发的载荷；
    /// 否则进入 Failed 状态（fail-closed）。
    pub fn handle_timeout(&mut self) -> TimeoutResult {
        match self.state {
            RotationState::Pending { retries } => {
                if retries < MAX_RETRIES {
                    // 重试
                    self.state = RotationState::Pending { retries: retries + 1 };
                    TimeoutResult::Retry(self.pending_payload.as_ref().unwrap().clone())
                } else {
                    // 3 次重试失败，fail-closed
                    self.state = RotationState::Failed;
                    self.pending_payload = None;
                    TimeoutResult::Failed
                }
            }
            _ => TimeoutResult::Ignored,
        }
    }

    /// 检查是否可以发送数据帧
    ///
    /// Failed 状态下禁止发送数据帧（fail-closed），等待人工复位。
    pub fn can_send_data(&self) -> bool {
        self.state != RotationState::Failed
    }

    /// 检查是否处于轮换进行中（Pending 状态）
    pub fn is_rotation_pending(&self) -> bool {
        matches!(self.state, RotationState::Pending { .. })
    }

    /// 获取当前重试次数（Pending 状态下）
    pub fn retry_count(&self) -> u8 {
        match self.state {
            RotationState::Pending { retries } => retries,
            _ => 0,
        }
    }

    /// 人工复位（从 Failed 状态恢复到 Idle）
    ///
    /// 仅在 Failed 状态下有效，需要人工物理复位或带外管理干预。
    pub fn manual_reset(&mut self) -> Result<(), RotationError> {
        match self.state {
            RotationState::Failed => {
                self.state = RotationState::Idle;
                self.pending_payload = None;
                Ok(())
            }
            _ => Err(RotationError::NotInFailedState),
        }
    }

    /// 完成轮换后重置到 Idle（Rotated → Idle）
    ///
    /// 新密钥已生效后调用，准备下一次轮换。
    pub fn complete_rotation(&mut self) -> Result<(), RotationError> {
        match self.state {
            RotationState::Rotated => {
                self.state = RotationState::Idle;
                Ok(())
            }
            _ => Err(RotationError::NotInRotatedState),
        }
    }
}

impl Default for KeyRotationStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

// ─── 超时处理结果 ───────────────────────────────────────────

/// ACK 超时处理结果
#[derive(Debug, Clone)]
pub enum TimeoutResult {
    /// 需要重试（返回重发的载荷）
    Retry(KeyRotationPayload),
    /// 轮换失败（3 次重试失败，fail-closed）
    Failed,
    /// 忽略（非 Pending 状态下的超时）
    Ignored,
}

// ─── 错误类型 ───────────────────────────────────────────────

/// 密钥轮换错误
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotationError {
    /// 载荷太短（不足 nonce 长度）
    PayloadTooShort,
    /// 不在 Idle 状态（无法开始轮换）
    NotInIdleState,
    /// 意外的 ACK（非 Pending 状态）
    UnexpectedAck,
    /// 不在 Failed 状态（无法复位）
    NotInFailedState,
    /// 不在 Rotated 状态（无法完成轮换）
    NotInRotatedState,
}

impl core::fmt::Display for RotationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::PayloadTooShort => write!(f, "KEY_ROTATION payload too short (need {} bytes nonce)", NONCE_SIZE),
            Self::NotInIdleState => write!(f, "cannot start rotation: not in Idle state"),
            Self::UnexpectedAck => write!(f, "unexpected ACK: not in Pending state"),
            Self::NotInFailedState => write!(f, "cannot manual reset: not in Failed state"),
            Self::NotInRotatedState => write!(f, "cannot complete rotation: not in Rotated state"),
        }
    }
}

impl std::error::Error for RotationError {}

// ─── 单元测试 ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(ROTATION_THRESHOLD, 65534);
        assert_eq!(ACK_TIMEOUT, Duration::from_millis(100));
        assert_eq!(MAX_RETRIES, 3);
        assert_eq!(NONCE_SIZE, 12);
    }

    #[test]
    fn test_payload_encode_decode_roundtrip() {
        let nonce = [0xAB; NONCE_SIZE];
        let key = vec![0xCD; 32];
        let payload = KeyRotationPayload::new(nonce, key.clone());
        let encoded = payload.encode();
        assert_eq!(encoded.len(), NONCE_SIZE + key.len());
        let decoded = KeyRotationPayload::decode(&encoded).unwrap();
        assert_eq!(decoded.nonce, nonce);
        assert_eq!(decoded.new_key_encrypted, key);
    }

    #[test]
    fn test_payload_decode_too_short() {
        let buf = [0u8; 10]; // 不足 12 字节
        assert_eq!(
            KeyRotationPayload::decode(&buf),
            Err(RotationError::PayloadTooShort)
        );
    }

    #[test]
    fn test_should_rotate() {
        let sm = KeyRotationStateMachine::new();
        assert!(!sm.should_rotate(0));
        assert!(!sm.should_rotate(100));
        assert!(!sm.should_rotate(65533));
        assert!(sm.should_rotate(65534));
        assert!(sm.should_rotate(65535));
    }

    #[test]
    fn test_start_rotation() {
        let mut sm = KeyRotationStateMachine::new();
        let payload = KeyRotationPayload::new([0x01; NONCE_SIZE], vec![0x02; 32]);
        let result = sm.start_rotation(payload.clone()).unwrap();
        assert_eq!(result, &payload);
        assert_eq!(sm.state(), RotationState::Pending { retries: 0 });
        assert!(sm.is_rotation_pending());
        assert_eq!(sm.retry_count(), 0);
    }

    #[test]
    fn test_start_rotation_not_idle() {
        let mut sm = KeyRotationStateMachine::new();
        let payload = KeyRotationPayload::new([0x01; NONCE_SIZE], vec![0x02; 32]);
        sm.start_rotation(payload.clone()).unwrap();
        // 再次开始轮换应该失败
        assert_eq!(
            sm.start_rotation(payload),
            Err(RotationError::NotInIdleState)
        );
    }

    #[test]
    fn test_handle_ack() {
        let mut sm = KeyRotationStateMachine::new();
        let payload = KeyRotationPayload::new([0x01; NONCE_SIZE], vec![0x02; 32]);
        sm.start_rotation(payload).unwrap();
        sm.handle_ack().unwrap();
        assert_eq!(sm.state(), RotationState::Rotated);
        assert!(sm.can_send_data());
    }

    #[test]
    fn test_handle_ack_unexpected() {
        let mut sm = KeyRotationStateMachine::new();
        assert_eq!(sm.handle_ack(), Err(RotationError::UnexpectedAck));
    }

    #[test]
    fn test_handle_timeout_retry() {
        let mut sm = KeyRotationStateMachine::new();
        let payload = KeyRotationPayload::new([0x01; NONCE_SIZE], vec![0x02; 32]);
        sm.start_rotation(payload.clone()).unwrap();

        // 第一次超时 → 重试（retries 0 → 1）
        match sm.handle_timeout() {
            TimeoutResult::Retry(p) => {
                assert_eq!(p, payload);
            }
            _ => panic!("expected Retry"),
        }
        assert_eq!(sm.state(), RotationState::Pending { retries: 1 });
        assert_eq!(sm.retry_count(), 1);
    }

    #[test]
    fn test_handle_timeout_fail_closed() {
        let mut sm = KeyRotationStateMachine::new();
        let payload = KeyRotationPayload::new([0x01; NONCE_SIZE], vec![0x02; 32]);
        sm.start_rotation(payload).unwrap();

        // 3 次超时（首次 + 3 次重试 = 总共 4 次发送尝试）
        // 第 1 次超时：retries 0 → 1
        assert!(matches!(sm.handle_timeout(), TimeoutResult::Retry(_)));
        // 第 2 次超时：retries 1 → 2
        assert!(matches!(sm.handle_timeout(), TimeoutResult::Retry(_)));
        // 第 3 次超时：retries 2 → 3
        assert!(matches!(sm.handle_timeout(), TimeoutResult::Retry(_)));
        // 第 4 次超时：retries 3 → Failed（retries == MAX_RETRIES）
        assert!(matches!(sm.handle_timeout(), TimeoutResult::Failed));

        assert_eq!(sm.state(), RotationState::Failed);
        // fail-closed：禁止发送数据帧
        assert!(!sm.can_send_data());
    }

    #[test]
    fn test_manual_reset() {
        let mut sm = KeyRotationStateMachine::new();
        let payload = KeyRotationPayload::new([0x01; NONCE_SIZE], vec![0x02; 32]);
        sm.start_rotation(payload).unwrap();
        // 触发 fail-closed
        for _ in 0..4 {
            sm.handle_timeout();
        }
        assert_eq!(sm.state(), RotationState::Failed);
        assert!(!sm.can_send_data());

        // 人工复位
        sm.manual_reset().unwrap();
        assert_eq!(sm.state(), RotationState::Idle);
        assert!(sm.can_send_data());
    }

    #[test]
    fn test_manual_reset_not_failed() {
        let mut sm = KeyRotationStateMachine::new();
        assert_eq!(sm.manual_reset(), Err(RotationError::NotInFailedState));
    }

    #[test]
    fn test_complete_rotation() {
        let mut sm = KeyRotationStateMachine::new();
        let payload = KeyRotationPayload::new([0x01; NONCE_SIZE], vec![0x02; 32]);
        sm.start_rotation(payload).unwrap();
        sm.handle_ack().unwrap();
        assert_eq!(sm.state(), RotationState::Rotated);

        // 完成轮换，回到 Idle
        sm.complete_rotation().unwrap();
        assert_eq!(sm.state(), RotationState::Idle);
    }

    #[test]
    fn test_complete_rotation_not_rotated() {
        let mut sm = KeyRotationStateMachine::new();
        assert_eq!(
            sm.complete_rotation(),
            Err(RotationError::NotInRotatedState)
        );
    }

    #[test]
    fn test_can_send_data_in_all_states() {
        let mut sm = KeyRotationStateMachine::new();
        assert!(sm.can_send_data()); // Idle

        let payload = KeyRotationPayload::new([0x01; NONCE_SIZE], vec![0x02; 32]);
        sm.start_rotation(payload).unwrap();
        assert!(sm.can_send_data()); // Pending（轮换期间仍可发送数据，用旧密钥）

        sm.handle_ack().unwrap();
        assert!(sm.can_send_data()); // Rotated

        // 触发 fail-closed
        sm.complete_rotation().unwrap();
        let payload2 = KeyRotationPayload::new([0x03; NONCE_SIZE], vec![0x04; 32]);
        sm.start_rotation(payload2).unwrap();
        for _ in 0..4 {
            sm.handle_timeout();
        }
        assert!(!sm.can_send_data()); // Failed（fail-closed）
    }

    #[test]
    fn test_full_rotation_lifecycle() {
        let mut sm = KeyRotationStateMachine::new();

        // 1. 检测到需要轮换
        assert!(sm.should_rotate(65534));

        // 2. 开始轮换
        let payload = KeyRotationPayload::new([0xAA; NONCE_SIZE], vec![0xBB; 32]);
        let send_payload = sm.start_rotation(payload.clone()).unwrap().clone();
        assert_eq!(send_payload, payload);

        // 3. 模拟发送 KEY_ROTATION 帧（FrameType::KeyRotation = 0x07）
        // （实际发送由调用方完成）

        // 4. 收到 ACK
        sm.handle_ack().unwrap();
        assert_eq!(sm.state(), RotationState::Rotated);

        // 5. 新密钥生效，完成轮换
        sm.complete_rotation().unwrap();
        assert_eq!(sm.state(), RotationState::Idle);

        // 6. 可以开始下一次轮换
        assert!(sm.should_rotate(65534));
    }
}
