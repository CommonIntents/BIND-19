//! 帧处理器（整合防重放缓存 + 帧解码 + 规则检查）
//!
//! CI-144 v2.0 规则 4（防重放检查）：
//! - 若 SAP.Replay-Enable == 1，必须校验 Seq-Counter
//! - 拒绝条件：Seq-Counter ≤ Last-Seen-Seq[Source-ID]
//! - 拒绝动作：拉高 ERROR 电平，写入审计日志 REJECTED_REPLAY，严禁放行
//!
//! 处理器整合：
//! - ReplayCache：高并发防重放缓存（DashMap + TTL）
//! - BindFrame：帧解码（PFP/SAP/Payload）
//! - 规则 4：防重放检查
//! - 规则 6：Replay-Enable=0 强制降级（通过 effective_risk_level）
//!
//! 规范依据：规则 4（防重放检查）
//! ADR：ADR-0007（Seq-Counter 冷启动攻击窗口）

use crate::frame::BindFrame;
use crate::replay_cache::{
    ReplayCache, ReplayCheckResult, ReplayKey, SourceId, TenantId,
};

/// 帧处理结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameProcessResult {
    /// 帧允许通过（防重放检查通过，或 Replay-Enable=0 跳过检查）
    Allowed,
    /// 重放拒绝（Seq-Counter ≤ Last-Seen-Seq）
    RejectedReplay,
    /// 缓存已满，无法注册新源（拒绝该帧）
    RejectedCacheFull,
    /// 帧无 SAP（无法进行防重放检查，v1.0 兼容帧或节能模式）
    NoSap,
}

impl FrameProcessResult {
    /// 是否允许通过
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed | Self::NoSap)
    }

    /// 是否被拒绝
    pub fn is_rejected(&self) -> bool {
        matches!(self, Self::RejectedReplay | Self::RejectedCacheFull)
    }
}

/// 帧处理器（整合防重放缓存 + 帧处理）
///
/// # 示例
///
/// ```
/// use bind19::processor::FrameProcessor;
/// use bind19::frame::BindFrame;
///
/// let processor = FrameProcessor::new(1); // tenant_id = 1
/// // 假设 frame 是已解码的帧，source_id = 100
/// // let result = processor.process_frame(&frame, 100);
/// ```
#[derive(Debug)]
pub struct FrameProcessor {
    /// 防重放缓存
    replay_cache: ReplayCache,
    /// 租户 ID（多租户隔离）
    tenant_id: TenantId,
}

impl FrameProcessor {
    /// 创建新的帧处理器
    pub fn new(tenant_id: TenantId) -> Self {
        Self {
            replay_cache: ReplayCache::new(),
            tenant_id,
        }
    }

    /// 创建新的帧处理器（自定义 TTL）
    pub fn with_ttl(tenant_id: TenantId, ttl: std::time::Duration) -> Self {
        Self {
            replay_cache: ReplayCache::with_ttl(ttl),
            tenant_id,
        }
    }

    /// 处理帧：检查防重放，返回处理结果
    ///
    /// 处理逻辑：
    /// 1. 如果帧无 SAP → 返回 NoSap（不进行防重放检查）
    /// 2. 如果 SAP.Replay-Enable == 0 → 返回 Allowed（跳过防重放检查，规则 6 降级由 effective_risk_level 处理）
    /// 3. 如果 SAP.Replay-Enable == 1 → 调用 ReplayCache.check_and_update()
    ///    - Allowed → 返回 Allowed
    ///    - Rejected → 返回 RejectedReplay
    ///    - CacheFull → 返回 RejectedCacheFull
    pub fn process_frame(&self, frame: &BindFrame, source_id: SourceId) -> FrameProcessResult {
        // 1. 检查是否有 SAP
        let sap = match &frame.sap {
            Some(sap) => sap,
            None => return FrameProcessResult::NoSap,
        };

        // 2. 检查 Replay-Enable
        // 注意：Replay-Enable 在 PFP 第 3 字节第 2 位，不在 SAP 中
        // 如果帧无 PFP，则默认 Replay-Enable=1（严格检查）
        let replay_enable = frame
            .pfp
            .as_ref()
            .map(|p| p.replay_enable)
            .unwrap_or(true);

        if !replay_enable {
            // Replay-Enable=0：跳过防重放检查
            // 规则 6 降级由 frame.effective_risk_level() 处理
            return FrameProcessResult::Allowed;
        }

        // 3. 防重放检查
        let key = ReplayKey::new(self.tenant_id, source_id);
        match self.replay_cache.check_and_update(key, sap.seq_counter) {
            ReplayCheckResult::Allowed => FrameProcessResult::Allowed,
            ReplayCheckResult::Rejected => FrameProcessResult::RejectedReplay,
            ReplayCheckResult::CacheFull => FrameProcessResult::RejectedCacheFull,
        }
    }

    /// 仅检查防重放（不更新缓存），用于预览
    pub fn check_frame(&self, frame: &BindFrame, source_id: SourceId) -> FrameProcessResult {
        let sap = match &frame.sap {
            Some(sap) => sap,
            None => return FrameProcessResult::NoSap,
        };

        let replay_enable = frame
            .pfp
            .as_ref()
            .map(|p| p.replay_enable)
            .unwrap_or(true);

        if !replay_enable {
            return FrameProcessResult::Allowed;
        }

        let key = ReplayKey::new(self.tenant_id, source_id);
        match self.replay_cache.check_only(key, sap.seq_counter) {
            ReplayCheckResult::Allowed => FrameProcessResult::Allowed,
            ReplayCheckResult::Rejected => FrameProcessResult::RejectedReplay,
            ReplayCheckResult::CacheFull => FrameProcessResult::RejectedCacheFull,
        }
    }

    /// 获取防重放缓存（只读访问）
    pub fn replay_cache(&self) -> &ReplayCache {
        &self.replay_cache
    }

    /// 获取租户 ID
    pub fn tenant_id(&self) -> TenantId {
        self.tenant_id
    }

    /// 清理过期缓存条目
    pub fn cleanup_expired(&self) -> usize {
        self.replay_cache.cleanup_expired()
    }

    /// 获取某源的最后 Seq-Counter
    pub fn last_seq(&self, source_id: SourceId) -> Option<u16> {
        let key = ReplayKey::new(self.tenant_id, source_id);
        self.replay_cache.last_seq(key)
    }

    /// 手动移除某源的缓存条目（密钥轮换后调用）
    pub fn remove_source(&self, source_id: SourceId) -> bool {
        let key = ReplayKey::new(self.tenant_id, source_id);
        self.replay_cache.remove(key)
    }
}

// ─── 单元测试 ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::FrameType;
    use crate::pfp::{
        BodyStance, Modality, OutputDest, OverrideFlag, PfpHeader, ProximityEdge, RiskLevel,
    };
    use crate::sap::SapHeader;

    fn make_frame_with_sap(seq: u16, replay_enable: bool) -> BindFrame {
        let pfp = PfpHeader::new(
            Modality::Cognitive,
            RiskLevel::Low,
            BodyStance::Unknown,
            ProximityEdge::Safe,
            OutputDest::Internal,
            OverrideFlag::Normal,
            replay_enable,
        );
        let pah_hash = [0u8; 14];
        let pah_signature = [0u8; 8];
        let sap = SapHeader::new(seq, pah_hash, pah_signature);
        BindFrame::new(FrameType::Data, 0, 0, Some(pfp), Some(sap), vec![]).unwrap()
    }

    fn make_frame_without_sap() -> BindFrame {
        let pfp = PfpHeader::new(
            Modality::Cognitive,
            RiskLevel::Low,
            BodyStance::Unknown,
            ProximityEdge::Safe,
            OutputDest::Internal,
            OverrideFlag::Normal,
            true,
        );
        BindFrame::new(FrameType::Data, 0, 0, Some(pfp), None, vec![]).unwrap()
    }

    #[test]
    fn test_processor_new() {
        let processor = FrameProcessor::new(1);
        assert_eq!(processor.tenant_id(), 1);
        assert!(processor.replay_cache().is_empty());
    }

    #[test]
    fn test_process_frame_first_seen_allowed() {
        let processor = FrameProcessor::new(1);
        let frame = make_frame_with_sap(42, true);
        let result = processor.process_frame(&frame, 100);
        assert_eq!(result, FrameProcessResult::Allowed);
        assert!(result.is_allowed());
        assert_eq!(processor.last_seq(100), Some(42));
    }

    #[test]
    fn test_process_frame_replay_rejected() {
        let processor = FrameProcessor::new(1);
        let frame1 = make_frame_with_sap(42, true);
        let frame2 = make_frame_with_sap(42, true); // 重放

        assert_eq!(processor.process_frame(&frame1, 100), FrameProcessResult::Allowed);
        assert_eq!(processor.process_frame(&frame2, 100), FrameProcessResult::RejectedReplay);
        assert!(processor.process_frame(&frame2, 100).is_rejected());
    }

    #[test]
    fn test_process_frame_old_seq_rejected() {
        let processor = FrameProcessor::new(1);
        let frame1 = make_frame_with_sap(50, true);
        let frame2 = make_frame_with_sap(40, true); // 旧 seq

        assert_eq!(processor.process_frame(&frame1, 100), FrameProcessResult::Allowed);
        assert_eq!(processor.process_frame(&frame2, 100), FrameProcessResult::RejectedReplay);
    }

    #[test]
    fn test_process_frame_newer_seq_allowed() {
        let processor = FrameProcessor::new(1);
        let frame1 = make_frame_with_sap(42, true);
        let frame2 = make_frame_with_sap(43, true);
        let frame3 = make_frame_with_sap(100, true);

        assert_eq!(processor.process_frame(&frame1, 100), FrameProcessResult::Allowed);
        assert_eq!(processor.process_frame(&frame2, 100), FrameProcessResult::Allowed);
        assert_eq!(processor.process_frame(&frame3, 100), FrameProcessResult::Allowed);
        assert_eq!(processor.last_seq(100), Some(100));
    }

    #[test]
    fn test_process_frame_replay_disabled_skips_check() {
        let processor = FrameProcessor::new(1);
        // Replay-Enable=0，跳过防重放检查
        let frame1 = make_frame_with_sap(42, false);
        let frame2 = make_frame_with_sap(42, false); // 相同 seq，但跳过检查

        assert_eq!(processor.process_frame(&frame1, 100), FrameProcessResult::Allowed);
        assert_eq!(processor.process_frame(&frame2, 100), FrameProcessResult::Allowed);
        // 缓存不应该更新（因为跳过了检查）
        assert_eq!(processor.last_seq(100), None);
    }

    #[test]
    fn test_process_frame_no_sap() {
        let processor = FrameProcessor::new(1);
        let frame = make_frame_without_sap();
        let result = processor.process_frame(&frame, 100);
        assert_eq!(result, FrameProcessResult::NoSap);
        assert!(result.is_allowed());
    }

    #[test]
    fn test_process_frame_multiple_sources_isolated() {
        let processor = FrameProcessor::new(1);
        let frame1 = make_frame_with_sap(42, true);
        let frame2 = make_frame_with_sap(42, true); // 相同 seq，不同 source

        assert_eq!(processor.process_frame(&frame1, 100), FrameProcessResult::Allowed);
        assert_eq!(processor.process_frame(&frame2, 200), FrameProcessResult::Allowed);
        assert_eq!(processor.replay_cache().len(), 2);
    }

    #[test]
    fn test_process_frame_multiple_tenants_isolated() {
        let processor1 = FrameProcessor::new(1);
        let processor2 = FrameProcessor::new(2);
        let frame = make_frame_with_sap(42, true);

        assert_eq!(processor1.process_frame(&frame, 100), FrameProcessResult::Allowed);
        // 不同租户，相同 source_id 和 seq，应该允许
        assert_eq!(processor2.process_frame(&frame, 100), FrameProcessResult::Allowed);
    }

    #[test]
    fn test_check_frame_does_not_update() {
        let processor = FrameProcessor::new(1);
        let frame = make_frame_with_sap(42, true);

        // check_only 不更新缓存
        assert_eq!(processor.check_frame(&frame, 100), FrameProcessResult::Allowed);
        assert!(processor.replay_cache().is_empty());

        // process_frame 才更新
        assert_eq!(processor.process_frame(&frame, 100), FrameProcessResult::Allowed);
        assert_eq!(processor.replay_cache().len(), 1);
    }

    #[test]
    fn test_remove_source_after_key_rotation() {
        let processor = FrameProcessor::new(1);
        let frame = make_frame_with_sap(42, true);

        processor.process_frame(&frame, 100);
        assert_eq!(processor.last_seq(100), Some(42));

        // 密钥轮换后移除源缓存
        assert!(processor.remove_source(100));
        assert_eq!(processor.last_seq(100), None);

        // 移除后，相同 seq 应该允许（新会话）
        assert_eq!(processor.process_frame(&frame, 100), FrameProcessResult::Allowed);
    }

    #[test]
    fn test_cleanup_expired() {
        use std::time::Duration;
        let processor = FrameProcessor::with_ttl(1, Duration::from_millis(10));
        let frame1 = make_frame_with_sap(42, true);
        let frame2 = make_frame_with_sap(43, true);

        processor.process_frame(&frame1, 100);
        processor.process_frame(&frame2, 200);
        assert_eq!(processor.replay_cache().len(), 2);

        // 等待过期
        std::thread::sleep(Duration::from_millis(20));

        // 清理过期
        let cleaned = processor.cleanup_expired();
        assert_eq!(cleaned, 2);
        assert!(processor.replay_cache().is_empty());
    }

    #[test]
    fn test_frame_process_result_is_allowed() {
        assert!(FrameProcessResult::Allowed.is_allowed());
        assert!(FrameProcessResult::NoSap.is_allowed());
        assert!(!FrameProcessResult::RejectedReplay.is_allowed());
        assert!(!FrameProcessResult::RejectedCacheFull.is_allowed());
    }

    #[test]
    fn test_frame_process_result_is_rejected() {
        assert!(!FrameProcessResult::Allowed.is_rejected());
        assert!(!FrameProcessResult::NoSap.is_rejected());
        assert!(FrameProcessResult::RejectedReplay.is_rejected());
        assert!(FrameProcessResult::RejectedCacheFull.is_rejected());
    }
}
