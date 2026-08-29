//! BIND-19 v2.0 端到端集成测试
//!
//! 覆盖完整帧编解码 + PFP/SAP + 防重放 + 密钥轮换 + CATASTROPHIC + 规则6 + 多租户
//!
//! 这些测试模拟真实使用场景，验证各模块之间的集成正确性。

use bind19::catastrophic::CatastrophicManager;
use bind19::frame::{BindFrame, FrameType};
use bind19::pfp::{
    BodyStance, Modality, OutputDest, OverrideFlag, PfpHeader, ProximityEdge, RiskLevel,
};
use bind19::processor::{FrameProcessResult, FrameProcessor};
use bind19::rotation::{
    KeyRotationPayload, KeyRotationStateMachine, RotationState, TimeoutResult,
    ROTATION_THRESHOLD,
};
use bind19::sap::SapHeader;

// ─── 辅助函数 ───────────────────────────────────────────────

fn make_data_frame(seq: u16, replay_enable: bool, risk: RiskLevel) -> BindFrame {
    let pfp = PfpHeader::new(
        Modality::Executive,
        risk,
        BodyStance::Moving,
        ProximityEdge::Warning,
        OutputDest::External,
        OverrideFlag::Normal,
        replay_enable,
    );
    let sap = SapHeader::new(seq, [0u8; 14], [0u8; 8]);
    BindFrame::new(FrameType::Data, 1, 0, Some(pfp), Some(sap), vec![0xAA, 0xBB]).unwrap()
}

fn make_catastrophic_frame(seq: u16) -> BindFrame {
    let pfp = PfpHeader::new(
        Modality::Executive,
        RiskLevel::Catastrophic,
        BodyStance::Moving,
        ProximityEdge::CriticalEdge,
        OutputDest::External,
        OverrideFlag::HardOverride,
        true,
    );
    let sap = SapHeader::new(seq, [0u8; 14], [0u8; 8]);
    BindFrame::new(FrameType::Data, 1, 0, Some(pfp), Some(sap), vec![]).unwrap()
}

// ─── 测试 1：完整帧编解码 roundtrip ─────────────────────────

#[test]
fn test_e2e_frame_encode_decode_roundtrip() {
    // 模拟发送端：创建帧并编码
    let frame = make_data_frame(42, true, RiskLevel::Medium);
    let encoded = frame.encode();

    // 模拟网络传输（字节流）
    assert!(!encoded.is_empty());

    // 模拟接收端：解码帧
    let decoded = BindFrame::decode(&encoded).unwrap();

    // 验证帧内容完整
    assert_eq!(decoded.header.frame_type, FrameType::Data);
    assert_eq!(decoded.header.channel_id, 1);
    assert!(decoded.pfp.is_some());
    assert!(decoded.sap.is_some());
    assert_eq!(decoded.sap.unwrap().seq_counter, 42);
    assert_eq!(decoded.payload, vec![0xAA, 0xBB]);

    // 验证 PFP 字段
    let pfp = decoded.pfp.unwrap();
    assert_eq!(pfp.modality, Modality::Executive);
    assert_eq!(pfp.risk_level, RiskLevel::Medium);
    assert!(pfp.replay_enable);
}

// ─── 测试 2：防重放端到端 ───────────────────────────────────

#[test]
fn test_e2e_replay_protection() {
    let processor = FrameProcessor::new(1);
    let source_id = 100;

    // 模拟发送端发送 seq=1, 2, 3
    for seq in 1..=3 {
        let frame = make_data_frame(seq, true, RiskLevel::Low);
        let encoded = frame.encode();
        let decoded = BindFrame::decode(&encoded).unwrap();

        let result = processor.process_frame(&decoded, source_id);
        assert_eq!(result, FrameProcessResult::Allowed, "seq={} should be allowed", seq);
    }

    // 模拟攻击者重放 seq=2
    let replay_frame = make_data_frame(2, true, RiskLevel::Low);
    let encoded = replay_frame.encode();
    let decoded = BindFrame::decode(&encoded).unwrap();
    let result = processor.process_frame(&decoded, source_id);
    assert_eq!(result, FrameProcessResult::RejectedReplay, "replayed seq=2 should be rejected");

    // 正常 seq=4 应该允许
    let frame4 = make_data_frame(4, true, RiskLevel::Low);
    let encoded = frame4.encode();
    let decoded = BindFrame::decode(&encoded).unwrap();
    let result = processor.process_frame(&decoded, source_id);
    assert_eq!(result, FrameProcessResult::Allowed);
}

// ─── 测试 3：密钥轮换端到端 ─────────────────────────────────

#[test]
fn test_e2e_key_rotation_lifecycle() {
    let mut rotation_sm = KeyRotationStateMachine::new();
    let processor = FrameProcessor::new(1);
    let source_id = 100;

    // 1. 正常发送帧直到接近轮换阈值
    for seq in 0..5 {
        let frame = make_data_frame(seq, true, RiskLevel::Low);
        let result = processor.process_frame(&frame, source_id);
        assert_eq!(result, FrameProcessResult::Allowed);
    }

    // 2. 检测到需要轮换（seq >= 65534）
    assert!(rotation_sm.should_rotate(ROTATION_THRESHOLD));

    // 3. 开始轮换
    let nonce = [0x01; 12];
    let new_key = vec![0xCD; 32];
    let payload = KeyRotationPayload::new(nonce, new_key.clone());
    let send_payload = rotation_sm.start_rotation(payload).unwrap().clone();
    assert_eq!(rotation_sm.state(), RotationState::Pending { retries: 0 });

    // 4. 模拟发送 KEY_ROTATION 帧
    let rotation_frame = BindFrame::new(
        FrameType::KeyRotation,
        1,
        0,
        None,
        None,
        send_payload.encode(),
    )
    .unwrap();
    let encoded = rotation_frame.encode();
    let decoded = BindFrame::decode(&encoded).unwrap();
    assert_eq!(decoded.header.frame_type, FrameType::KeyRotation);

    // 5. 接收端解析轮换载荷
    let received_payload = KeyRotationPayload::decode(&decoded.payload).unwrap();
    assert_eq!(received_payload.nonce, nonce);
    assert_eq!(received_payload.new_key_encrypted, new_key);

    // 6. 接收端发送 ACK
    let ack_frame = BindFrame::new_v1(FrameType::KeyRotationAck, 1, 0, vec![]).unwrap();
    let _ack_encoded = ack_frame.encode();

    // 7. 发送端收到 ACK
    rotation_sm.handle_ack().unwrap();
    assert_eq!(rotation_sm.state(), RotationState::Rotated);

    // 8. 清除防重放缓存（新密钥，新会话）
    assert!(processor.remove_source(source_id));

    // 9. 新会话从 seq=0 开始（应该允许，因为缓存已清除）
    let new_frame = make_data_frame(0, true, RiskLevel::Low);
    let result = processor.process_frame(&new_frame, source_id);
    assert_eq!(result, FrameProcessResult::Allowed);

    // 10. 完成轮换
    rotation_sm.complete_rotation().unwrap();
    assert_eq!(rotation_sm.state(), RotationState::Idle);
}

// ─── 测试 4：CATASTROPHIC 端到端 ───────────────────────────

#[test]
fn test_e2e_catastrophic_override() {
    let (manager, receiver) = CatastrophicManager::new();

    // 模拟发送端创建 CATASTROPHIC 帧
    let frame = make_catastrophic_frame(100);
    let encoded = frame.encode();
    let decoded = BindFrame::decode(&encoded).unwrap();

    // 验证帧确实是 CATASTROPHIC
    assert!(decoded.is_catastrophic_override());

    // 模拟接收端检测并触发 CATASTROPHIC 事件
    let pfp_bytes = decoded.pfp.unwrap().encode();
    let triggered = manager.handle_frame(
        &pfp_bytes,
        Some(100),
        Some(vec![0xDE, 0xAD]),
        "tuck",
    );
    assert!(triggered);

    // 验证事件总线收到事件
    let event = receiver.recv().unwrap();
    assert_eq!(event.seq_counter, Some(100));
    assert_eq!(event.source, "tuck");
    assert_eq!(event.sensor_context, Some(vec![0xDE, 0xAD]));

    // 验证审计日志记录
    assert_eq!(manager.total_triggers(), 1);
    let entries = manager.audit_entries();
    assert_eq!(entries.len(), 1);
    assert!(entries[0].verify_hash());

    // 验证审计链完整性
    assert!(manager.verify_audit_chain());
}

// ─── 测试 5：规则 6 降级端到端 ──────────────────────────────

#[test]
fn test_e2e_rule6_replay_disabled_downgrade() {
    use bind19::config::BindConfig;

    // 创建 Replay-Enable=0 的帧（原始风险等级 CATASTROPHIC）
    let pfp = PfpHeader::new(
        Modality::Executive,
        RiskLevel::Catastrophic,
        BodyStance::Moving,
        ProximityEdge::CriticalEdge,
        OutputDest::External,
        OverrideFlag::HardOverride,
        false, // Replay-Enable = 0
    );
    let sap = SapHeader::new(42, [0u8; 14], [0u8; 8]);
    let frame = BindFrame::new(FrameType::Data, 1, 0, Some(pfp), Some(sap), vec![]).unwrap();

    // 生产模式：有效风险等级强制降级至 MEDIUM
    let config = BindConfig::default();
    let effective_risk = frame.effective_risk_level_with_config(&config);
    assert_eq!(effective_risk, Some(RiskLevel::Medium));

    // 原始风险等级仍然是 CATASTROPHIC
    assert_eq!(frame.pfp.as_ref().unwrap().risk_level, RiskLevel::Catastrophic);

    // 帧仍然标记为 CATASTROPHIC 硬覆盖（原始标志）
    assert!(frame.is_catastrophic_override());

    // 但实际决策应使用有效风险等级（MEDIUM），不触发 CATASTROPHIC 硬覆盖
    // （这是规则 6 的核心：用降级补偿防重放缺失）

    // 调试模式：不降级
    let debug_config = BindConfig { debug_mode: true };
    let effective_risk_debug = frame.effective_risk_level_with_config(&debug_config);
    assert_eq!(effective_risk_debug, Some(RiskLevel::Catastrophic));
}

// ─── 测试 6：多租户端到端 ───────────────────────────────────

#[test]
fn test_e2e_multi_tenant_isolation() {
    let processor_a = FrameProcessor::new(1); // 租户 A
    let processor_b = FrameProcessor::new(2); // 租户 B
    let source_id = 100; // 相同 source_id

    // 租户 A 发送 seq=42
    let frame_a = make_data_frame(42, true, RiskLevel::Low);
    assert_eq!(
        processor_a.process_frame(&frame_a, source_id),
        FrameProcessResult::Allowed
    );

    // 租户 B 发送相同 seq=42（应该允许，因为租户隔离）
    let frame_b = make_data_frame(42, true, RiskLevel::Low);
    assert_eq!(
        processor_b.process_frame(&frame_b, source_id),
        FrameProcessResult::Allowed
    );

    // 租户 A 重放 seq=42（应该拒绝）
    assert_eq!(
        processor_a.process_frame(&frame_a, source_id),
        FrameProcessResult::RejectedReplay
    );

    // 租户 B 重放 seq=42（应该拒绝）
    assert_eq!(
        processor_b.process_frame(&frame_b, source_id),
        FrameProcessResult::RejectedReplay
    );

    // 验证缓存隔离
    assert_eq!(processor_a.replay_cache().len(), 1);
    assert_eq!(processor_b.replay_cache().len(), 1);
    assert_eq!(processor_a.tenant_id(), 1);
    assert_eq!(processor_b.tenant_id(), 2);
}

// ─── 测试 7：完整数据流端到端（编码→传输→解码→防重放→处理） ─

#[test]
fn test_e2e_full_data_flow() {
    let processor = FrameProcessor::new(1);
    let source_id = 200;

    // 模拟 10 个帧的完整数据流
    for seq in 0..10 {
        // 发送端：创建帧
        let risk = if seq < 5 {
            RiskLevel::Low
        } else {
            RiskLevel::Medium
        };
        let frame = make_data_frame(seq, true, risk);

        // 编码
        let encoded = frame.encode();

        // 模拟网络传输（这里只是字节复制）
        let received = encoded.clone();

        // 接收端：解码
        let decoded = BindFrame::decode(&received).unwrap();

        // 防重放检查
        let result = processor.process_frame(&decoded, source_id);
        assert_eq!(result, FrameProcessResult::Allowed, "seq={} should pass", seq);

        // 验证有效风险等级
        let effective_risk = decoded.effective_risk_level().unwrap();
        assert_eq!(effective_risk, risk);
    }

    // 验证最后 seq
    assert_eq!(processor.last_seq(source_id), Some(9));

    // 验证缓存条目数
    assert_eq!(processor.replay_cache().len(), 1);
}

// ─── 测试 8：密钥轮换失败 fail-closed 端到端 ───────────────

#[test]
fn test_e2e_key_rotation_fail_closed() {
    let mut rotation_sm = KeyRotationStateMachine::new();

    // 开始轮换
    let payload = KeyRotationPayload::new([0x01; 12], vec![0xCD; 32]);
    rotation_sm.start_rotation(payload).unwrap();

    // 模拟 4 次 ACK 超时（首次 + 3 次重试）
    for i in 0..4 {
        let result = rotation_sm.handle_timeout();
        if i < 3 {
            assert!(matches!(result, TimeoutResult::Retry(_)));
        } else {
            assert!(matches!(result, TimeoutResult::Failed));
        }
    }

    // 验证进入 Failed 状态
    assert_eq!(rotation_sm.state(), RotationState::Failed);

    // fail-closed：禁止发送数据帧
    assert!(!rotation_sm.can_send_data());

    // 人工复位
    rotation_sm.manual_reset().unwrap();
    assert_eq!(rotation_sm.state(), RotationState::Idle);
    assert!(rotation_sm.can_send_data());
}

// ─── 测试 9：节能模式（无 SAP）端到端 ───────────────────────

#[test]
fn test_e2e_low_power_mode_no_sap() {
    let processor = FrameProcessor::new(1);
    let source_id = 100;

    // 创建无 SAP 的帧（节能模式，最小帧开销 11 字节）
    let pfp = PfpHeader::new(
        Modality::Cognitive,
        RiskLevel::Low,
        BodyStance::Unknown,
        ProximityEdge::Safe,
        OutputDest::Internal,
        OverrideFlag::Normal,
        true,
    );
    let frame = BindFrame::new(FrameType::Data, 1, 0, Some(pfp), None, vec![]).unwrap();

    // 编码
    let encoded = frame.encode();
    // 验证最小帧开销：8（BIND-19头）+ 4（PFP）= 12 字节（无 payload）
    // 注意：BIND-19 头部 8 字节 + PFP 4 字节 = 12 字节
    assert_eq!(encoded.len(), 12);

    // 解码
    let decoded = BindFrame::decode(&encoded).unwrap();
    assert!(decoded.sap.is_none());
    assert!(decoded.pfp.is_some());

    // 防重放检查：无 SAP → NoSap（允许通过）
    let result = processor.process_frame(&decoded, source_id);
    assert_eq!(result, FrameProcessResult::NoSap);
    assert!(result.is_allowed());

    // 缓存不应该更新（无 SAP 不进行防重放检查）
    assert!(processor.replay_cache().is_empty());
}
