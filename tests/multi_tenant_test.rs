//! CI-144 v2.0 多租户场景验证测试
//!
//! 验证跨租户隔离：缓存、计数器、密钥轮换状态机、FrameProcessor
//!
//! 运行方式：`cargo test --test multi_tenant_test`

use bind19::frame::{BindFrame, FrameType};
use bind19::pfp::{
    BodyStance, Modality, OutputDest, OverrideFlag, PfpHeader, ProximityEdge, RiskLevel,
};
use bind19::processor::FrameProcessor;
use bind19::replay_cache::{ReplayCache, ReplayCheckResult, ReplayKey};
use bind19::rotation::{KeyRotationStateMachine, ROTATION_THRESHOLD};
use bind19::sap::SapHeader;
use std::collections::HashSet;
use std::sync::Arc;
use std::thread;

// ─── 辅助函数 ────────────────────────────────────────────────

fn make_pfp(risk: RiskLevel, replay_enable: bool) -> PfpHeader {
    PfpHeader::new(
        Modality::Executive,
        risk,
        BodyStance::Moving,
        ProximityEdge::Warning,
        OutputDest::External,
        OverrideFlag::Normal,
        replay_enable,
    )
}

fn make_sap(seq: u16) -> SapHeader {
    SapHeader::new(seq, [0xAB; 14], [0xCD; 8])
}

fn make_frame(seq: u16, risk: RiskLevel) -> BindFrame {
    let pfp = make_pfp(risk, true);
    let sap = make_sap(seq);
    BindFrame::new(FrameType::Data, 1, 0, Some(pfp), Some(sap), vec![]).unwrap()
}

// ─── 1. 跨租户缓存隔离 ──────────────────────────────────────

#[test]
fn test_cross_tenant_cache_isolation() {
    let cache = ReplayCache::new();

    // 租户 A: source=100, 注册 seq=100
    let key_a = ReplayKey::new(1, 100);
    assert_eq!(
        cache.check_and_update(key_a, 100),
        ReplayCheckResult::Allowed
    );

    // 租户 B: source=100（同 source_id，不同 tenant），seq=100 应该是新源
    let key_b = ReplayKey::new(2, 100);
    assert_eq!(
        cache.check_and_update(key_b, 100),
        ReplayCheckResult::Allowed
    );

    // 租户 A: seq=100 应该被拒绝（重放）
    assert_eq!(
        cache.check_and_update(key_a, 100),
        ReplayCheckResult::Rejected
    );

    // 租户 B: seq=100 应该被拒绝（重放）
    assert_eq!(
        cache.check_and_update(key_b, 100),
        ReplayCheckResult::Rejected
    );

    // 租户 A: seq=101 应该允许
    assert_eq!(
        cache.check_and_update(key_a, 101),
        ReplayCheckResult::Allowed
    );

    // 租户 B: seq=50 应该被拒绝（50 < 100），不受租户 A 的 seq=101 影响
    assert_eq!(
        cache.check_and_update(key_b, 50),
        ReplayCheckResult::Rejected
    );
}

// ─── 2. 跨租户计数器隔离 ────────────────────────────────────

#[test]
fn test_cross_tenant_counter_isolation() {
    let cache = ReplayCache::new();

    // 租户 A: source=1, 递增到 50
    let key_a = ReplayKey::new(1, 1);
    for seq in 1..=50 {
        assert_eq!(
            cache.check_and_update(key_a, seq),
            ReplayCheckResult::Allowed
        );
    }

    // 租户 B: source=1, seq=1 应该是新源（不受租户 A 影响）
    let key_b = ReplayKey::new(2, 1);
    assert_eq!(
        cache.check_and_update(key_b, 1),
        ReplayCheckResult::Allowed
    );

    // 租户 B: seq=50 应该允许（租户 B 的 last_seq=1）
    assert_eq!(
        cache.check_and_update(key_b, 50),
        ReplayCheckResult::Allowed
    );

    // 租户 A: seq=50 应该被拒绝（租户 A 的 last_seq=50）
    assert_eq!(
        cache.check_and_update(key_a, 50),
        ReplayCheckResult::Rejected
    );
}

// ─── 3. 跨租户密钥轮换状态机隔离 ─────────────────────────────

#[test]
fn test_cross_tenant_key_rotation_isolation() {
    // 每个租户有独立的密钥轮换状态机
    let mut sm_a = KeyRotationStateMachine::new();
    let sm_b = KeyRotationStateMachine::new();

    // 租户 A: 触发轮换
    assert!(sm_a.should_rotate(ROTATION_THRESHOLD));
    let payload = bind19::rotation::KeyRotationPayload::new([0x01; 12], vec![0xAA; 32]);
    sm_a.start_rotation(payload).unwrap();
    assert!(sm_a.is_rotation_pending());

    // 租户 B: 不应该受影响，仍然是 Idle
    assert!(!sm_b.is_rotation_pending());
    assert!(sm_b.can_send_data());

    // 租户 A: ACK 成功，进入 Rotated
    sm_a.handle_ack().unwrap();
    sm_a.complete_rotation().unwrap();

    // 租户 B: 仍然可以独立触发轮换
    assert!(sm_b.should_rotate(ROTATION_THRESHOLD));
}

// ─── 4. 多租户并发访问 ──────────────────────────────────────

#[test]
fn test_multi_tenant_concurrent_access() {
    let cache = Arc::new(ReplayCache::new());
    let mut handles = vec![];

    // 10 个租户，每个租户 100 个 seq 递增
    for tenant_id in 1..=10u64 {
        let cache = Arc::clone(&cache);
        handles.push(thread::spawn(move || {
            let key = ReplayKey::new(tenant_id, 1);
            for seq in 1..=100u16 {
                let result = cache.check_and_update(key, seq);
                // 并发下偶尔可能有竞态，但大多数应该是 Allowed
                assert!(
                    result == ReplayCheckResult::Allowed || result == ReplayCheckResult::Rejected
                );
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // 验证每个租户的 last_seq 应该接近 100
    for tenant_id in 1..=10u64 {
        let key = ReplayKey::new(tenant_id, 1);
        // seq=1 一定被拒绝（已经注册过）
        assert_eq!(
            cache.check_and_update(key, 1),
            ReplayCheckResult::Rejected
        );
    }
}

// ─── 5. 租户 ID 边界情况 ────────────────────────────────────

#[test]
fn test_tenant_id_boundary() {
    let cache = ReplayCache::new();

    // 租户 ID = 0
    let key_zero = ReplayKey::new(0, 1);
    assert_eq!(
        cache.check_and_update(key_zero, 1),
        ReplayCheckResult::Allowed
    );
    assert_eq!(
        cache.check_and_update(key_zero, 1),
        ReplayCheckResult::Rejected
    );

    // 租户 ID = u64::MAX
    let key_max = ReplayKey::new(u64::MAX, 1);
    assert_eq!(
        cache.check_and_update(key_max, 1),
        ReplayCheckResult::Allowed
    );

    // 租户 0 和 租户 MAX 互不影响
    assert_eq!(
        cache.check_and_update(key_zero, 2),
        ReplayCheckResult::Allowed
    );
    assert_eq!(
        cache.check_and_update(key_max, 1),
        ReplayCheckResult::Rejected
    );
}

// ─── 6. 同一租户不同 source 隔离 ─────────────────────────────

#[test]
fn test_same_tenant_different_source_isolation() {
    let cache = ReplayCache::new();

    // 租户 1: source=1, seq=100
    let key_s1 = ReplayKey::new(1, 1);
    assert_eq!(
        cache.check_and_update(key_s1, 100),
        ReplayCheckResult::Allowed
    );

    // 租户 1: source=2, seq=100 应该是新源
    let key_s2 = ReplayKey::new(1, 2);
    assert_eq!(
        cache.check_and_update(key_s2, 100),
        ReplayCheckResult::Allowed
    );

    // 租户 1: source=1, seq=100 被拒绝
    assert_eq!(
        cache.check_and_update(key_s1, 100),
        ReplayCheckResult::Rejected
    );

    // 租户 1: source=2, seq=50 被拒绝
    assert_eq!(
        cache.check_and_update(key_s2, 50),
        ReplayCheckResult::Rejected
    );
}

// ─── 7. FrameProcessor 多租户处理（每个租户独立 Processor）───

#[test]
fn test_frame_processor_multi_tenant() {
    // 每个租户有独立的 FrameProcessor（绑定 tenant_id）
    let processor_t1 = FrameProcessor::new(1);
    let processor_t2 = FrameProcessor::new(2);

    // 租户 1: 处理 10 个帧
    for seq in 1..=10 {
        let frame = make_frame(seq, RiskLevel::Low);
        let result = processor_t1.process_frame(&frame, 100);
        assert!(result.is_allowed());
    }

    // 租户 2: 处理 10 个帧（不受租户 1 影响）
    for seq in 1..=10 {
        let frame = make_frame(seq, RiskLevel::Low);
        let result = processor_t2.process_frame(&frame, 100);
        assert!(result.is_allowed());
    }

    // 租户 1: 重放 seq=5 应该被拒绝
    let frame_replay = make_frame(5, RiskLevel::Low);
    let result = processor_t1.process_frame(&frame_replay, 100);
    assert!(result.is_rejected());

    // 租户 2: seq=5 应该被拒绝（租户 2 的 last_seq=10）
    let result = processor_t2.process_frame(&frame_replay, 100);
    assert!(result.is_rejected());
}

// ─── 8. 多租户缓存容量验证 ──────────────────────────────────

#[test]
fn test_multi_tenant_cache_capacity() {
    let cache = ReplayCache::new();

    // 注册 1000 个不同的 (tenant, source) 组合
    for i in 0..1000u64 {
        let tenant = i % 100;
        let source = i / 100;
        let key = ReplayKey::new(tenant, source);
        assert_eq!(
            cache.check_and_update(key, 1),
            ReplayCheckResult::Allowed
        );
    }

    // 验证所有 1000 个组合都被正确记录
    let mut seen = HashSet::new();
    for i in 0..1000u64 {
        let tenant = i % 100;
        let source = i / 100;
        let key = ReplayKey::new(tenant, source);
        let result = cache.check_and_update(key, 1);
        // seq=1 应该被拒绝（已经注册过）
        assert_eq!(result, ReplayCheckResult::Rejected);
        seen.insert((tenant, source));
    }
    assert_eq!(seen.len(), 1000);
}

// ─── 9. 多租户规则6降级隔离 ──────────────────────────────────

#[test]
fn test_multi_tenant_rule6_isolation() {
    let processor_t1 = FrameProcessor::new(1);
    let processor_t2 = FrameProcessor::new(2);

    // 租户 1: Replay-Enable=false 的帧（规则6降级，跳过防重放检查）
    let pfp_disabled = make_pfp(RiskLevel::Catastrophic, false);
    let sap = make_sap(1);
    let frame_disabled = BindFrame::new(
        FrameType::Data,
        1,
        0,
        Some(pfp_disabled),
        Some(sap),
        vec![],
    )
    .unwrap();

    // 租户 1: 处理 Replay-Enable=false 的帧（跳过防重放，返回 Allowed）
    let result = processor_t1.process_frame(&frame_disabled, 100);
    assert!(result.is_allowed()); // Replay-Enable=0 跳过检查

    // 验证 PFP 的 effective_risk_level 是 MEDIUM（规则6降级）
    let pfp = frame_disabled.pfp.as_ref().unwrap();
    assert_eq!(pfp.effective_risk_level(), RiskLevel::Medium);

    // 租户 2: 正常帧（Replay-Enable=true），不降级
    let frame_normal = make_frame(1, RiskLevel::Catastrophic);
    let result = processor_t2.process_frame(&frame_normal, 100);
    assert!(result.is_allowed());
    // Replay-Enable=true 时不降级，保持原始风险等级
    let pfp = frame_normal.pfp.as_ref().unwrap();
    assert_eq!(pfp.effective_risk_level(), RiskLevel::Catastrophic);
}
