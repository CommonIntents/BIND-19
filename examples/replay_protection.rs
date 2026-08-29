//! CI-144 v2.0 防重放保护使用示例
//!
//! 展示如何使用 ReplayCache 和 FrameProcessor 进行防重放检查
//!
//! 运行方式：`cargo run --example replay_protection`

use bind19::frame::{BindFrame, FrameType};
use bind19::pfp::{
    BodyStance, Modality, OutputDest, OverrideFlag, PfpHeader, ProximityEdge, RiskLevel,
};
use bind19::processor::FrameProcessor;
use bind19::replay_cache::{ReplayCache, ReplayCheckResult, ReplayKey};
use bind19::sap::SapHeader;

fn make_frame(seq: u16) -> BindFrame {
    let pfp = PfpHeader::new(
        Modality::Executive,
        RiskLevel::Low,
        BodyStance::Moving,
        ProximityEdge::Safe,
        OutputDest::Internal,
        OverrideFlag::Normal,
        true,
    );
    let sap = SapHeader::new(seq, [0xAB; 14], [0xCD; 8]);
    BindFrame::new(FrameType::Data, 1, 0, Some(pfp), Some(sap), vec![]).unwrap()
}

fn main() {
    println!("=== CI-144 v2.0 防重放保护使用示例 ===\n");

    // ─── 1. 直接使用 ReplayCache ──────────────────────────────
    println!("1. 直接使用 ReplayCache（低级 API）");
    println!("   按 (tenant_id, source_id) 分片缓存 last_seq");

    let cache = ReplayCache::new();
    let key = ReplayKey::new(1, 100); // tenant=1, source=100

    // 第一帧: seq=100（新源，允许）
    let result = cache.check_and_update(key, 100);
    println!("   seq=100 (新源): {:?}", result);
    assert_eq!(result, ReplayCheckResult::Allowed);

    // 第二帧: seq=101（递增，允许）
    let result = cache.check_and_update(key, 101);
    println!("   seq=101 (递增): {:?}", result);
    assert_eq!(result, ReplayCheckResult::Allowed);

    // 第三帧: seq=100（重放，拒绝）
    let result = cache.check_and_update(key, 100);
    println!("   seq=100 (重放): {:?}", result);
    assert_eq!(result, ReplayCheckResult::Rejected);

    // 第四帧: seq=50（旧seq，拒绝）
    let result = cache.check_and_update(key, 50);
    println!("   seq=50  (旧seq): {:?}", result);
    assert_eq!(result, ReplayCheckResult::Rejected);

    // 新源: seq=1（不同 source_id，允许）
    let new_key = ReplayKey::new(1, 200);
    let result = cache.check_and_update(new_key, 1);
    println!("   seq=1   (新源 source=200): {:?}", result);
    assert_eq!(result, ReplayCheckResult::Allowed);

    println!();

    // ─── 2. 使用 FrameProcessor（高级 API）────────────────────
    println!("2. 使用 FrameProcessor（高级 API，整合防重放）");
    println!("   自动处理: 无SAP→NoSap, Replay-Enable=0→跳过, 正常→防重放");

    let processor = FrameProcessor::new(1); // tenant_id = 1
    let source_id = 100u64;

    // 正常帧序列
    for seq in 1..=5 {
        let frame = make_frame(seq);
        let result = processor.process_frame(&frame, source_id);
        println!("   seq={}: {:?} (allowed={})", seq, result, result.is_allowed());
        assert!(result.is_allowed());
    }

    // 重放帧
    let replay_frame = make_frame(3);
    let result = processor.process_frame(&replay_frame, source_id);
    println!("   seq=3 (重放): {:?} (rejected={})", result, result.is_rejected());
    assert!(result.is_rejected());

    println!();

    // ─── 3. 多租户隔离 ────────────────────────────────────────
    println!("3. 多租户隔离（每个租户独立缓存）");

    let processor_t1 = FrameProcessor::new(1);
    let processor_t2 = FrameProcessor::new(2);

    // 租户1: seq=100
    let frame = make_frame(100);
    let result = processor_t1.process_frame(&frame, 100);
    println!("   租户1 seq=100: {:?}", result);

    // 租户2: seq=100（不受租户1影响，新源允许）
    let result = processor_t2.process_frame(&frame, 100);
    println!("   租户2 seq=100: {:?} (独立缓存，允许)", result);
    assert!(result.is_allowed());

    // 租户1: seq=100（重放，拒绝）
    let result = processor_t1.process_frame(&frame, 100);
    println!("   租户1 seq=100 (重放): {:?}", result);
    assert!(result.is_rejected());

    println!();

    // ─── 4. 规则6：Replay-Enable=0 跳过防重放 ────────────────
    println!("4. 规则6: Replay-Enable=0 跳过防重放检查（节能模式）");

    let pfp_eco = PfpHeader::new(
        Modality::Executive,
        RiskLevel::Catastrophic, // 原始高风险
        BodyStance::Moving,
        ProximityEdge::CriticalEdge,
        OutputDest::External,
        OverrideFlag::Normal,
        false, // Replay-Enable = 0
    );
    let sap_eco = SapHeader::new(1, [0xAB; 14], [0xCD; 8]);
    let eco_frame = BindFrame::new(
        FrameType::Data,
        1,
        0,
        Some(pfp_eco.clone()),
        Some(sap_eco),
        vec![],
    )
    .unwrap();

    // Replay-Enable=0 的帧跳过防重放检查
    let processor_eco = FrameProcessor::new(1);
    let result = processor_eco.process_frame(&eco_frame, 100);
    println!("   Replay-Enable=0 帧: {:?} (跳过检查，允许)", result);
    assert!(result.is_allowed());

    // 但有效风险等级被强制降级为 MEDIUM（规则6）
    let effective_risk = pfp_eco.effective_risk_level();
    println!("   原始风险: Catastrophic");
    println!("   有效风险: {:?} (规则6强制降级)", effective_risk);
    assert_eq!(effective_risk, RiskLevel::Medium);

    // 重复发送同一帧仍然允许（因为跳过了防重放检查）
    let result = processor_eco.process_frame(&eco_frame, 100);
    println!("   重复发送: {:?} (仍然允许，因为跳过检查)", result);
    assert!(result.is_allowed());

    println!();

    println!("=== 示例完成 ===");
    println!();
    println!("关键要点:");
    println!("  1. ReplayCache 按 (tenant_id, source_id) 分片，容量10万，TTL 60秒");
    println!("  2. 防重放规则: seq > last_seq → Allowed, seq ≤ last_seq → Rejected");
    println!("  3. FrameProcessor 自动处理无SAP帧和Replay-Enable=0帧");
    println!("  4. Replay-Enable=0 时跳过防重放，但规则6强制降级风险至 MEDIUM");
    println!("  5. 多租户完全隔离，每个租户有独立的缓存");
}
