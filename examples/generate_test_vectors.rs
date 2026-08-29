//! CI-144 v2.0 测试向量生成器
//!
//! 生成 PFP/SAP/帧编解码、防重放、规则6降级、密钥轮换、CATASTROPHIC检测、PAH签名
//! 等测试向量，输出为 JSON 格式，供其他兼容 CI-144 协议家族的项目使用。
//!
//! 运行方式：`cargo run --example generate_test_vectors`
//! 输出目录：`tests/test_vectors/`

use bind19::crypto::{KeyPair, sign, sign_truncated, truncate_signature};
use bind19::frame::{BindFrame, FrameType};
use bind19::pfp::{
    BodyStance, Modality, OutputDest, OverrideFlag, PfpHeader, ProximityEdge, RiskLevel,
};
use bind19::replay_cache::{ReplayCache, ReplayCheckResult, ReplayKey};
use bind19::rotation::{KeyRotationStateMachine, ROTATION_THRESHOLD};
use bind19::sap::SapHeader;
use std::fs;
use std::path::Path;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn main() {
    let output_dir = Path::new("tests/test_vectors");
    fs::create_dir_all(output_dir).unwrap();

    let mut all_vectors = serde_json::Map::new();

    // 1. PFP 编解码测试向量（5 组）
    all_vectors.insert("pfp_codec".into(), generate_pfp_vectors());

    // 2. SAP 编解码测试向量（5 组）
    all_vectors.insert("sap_codec".into(), generate_sap_vectors());

    // 3. 完整帧编解码测试向量（5 组）
    all_vectors.insert("frame_codec".into(), generate_frame_vectors());

    // 4. 防重放测试向量（5 组）
    all_vectors.insert("replay_protection".into(), generate_replay_vectors());

    // 5. 规则6降级测试向量（3 组）
    all_vectors.insert("rule6_downgrade".into(), generate_rule6_vectors());

    // 6. 密钥轮换测试向量（4 组）
    all_vectors.insert("key_rotation".into(), generate_key_rotation_vectors());

    // 7. CATASTROPHIC 检测测试向量（3 组）
    all_vectors.insert("catastrophic_detection".into(), generate_catastrophic_vectors());

    // 8. PAH 签名测试向量（3 组）
    all_vectors.insert("pah_signature".into(), generate_pah_signature_vectors());

    // 统计
    let total: usize = all_vectors
        .values()
        .map(|v| v.as_array().map(|a| a.len()).unwrap_or(0))
        .sum();

    let metadata = serde_json::json!({
        "protocol": "CI-144 v2.0",
        "generated_at": "2026-08-29",
        "total_vectors": total,
        "categories": all_vectors.keys().collect::<Vec<_>>(),
        "description": "CI-144 v2.0 协议家族测试向量，供其他兼容实现验证使用",
        "usage": "每个测试向量包含 input（输入）和 expected（期望输出），实现者应验证 input 经过处理后等于 expected"
    });

    let output = serde_json::json!({
        "metadata": metadata,
        "vectors": all_vectors
    });

    let json = serde_json::to_string_pretty(&output).unwrap();
    let output_path = output_dir.join("ci-144-v2.0-test-vectors.json");
    fs::write(&output_path, &json).unwrap();

    println!("✅ 测试向量生成完成！");
    println!("   总数: {} 组", total);
    println!("   输出: {}", output_path.display());
    println!();
    println!("   分类:");
    for (key, value) in &all_vectors {
        let count = value.as_array().map(|a| a.len()).unwrap_or(0);
        println!("     - {}: {} 组", key, count);
    }
}

// ─── 1. PFP 编解码测试向量 ──────────────────────────────────

fn generate_pfp_vectors() -> serde_json::Value {
    let cases = vec![
        ("all_zero", Modality::Cognitive, RiskLevel::Low, BodyStance::Unknown, ProximityEdge::Safe, OutputDest::Internal, OverrideFlag::Normal, false),
        ("all_one", Modality::SensorFeed, RiskLevel::Catastrophic, BodyStance::Moving, ProximityEdge::CriticalEdge, OutputDest::External, OverrideFlag::HardOverride, true),
        ("typical_executive", Modality::Executive, RiskLevel::Medium, BodyStance::Standing, ProximityEdge::Warning, OutputDest::External, OverrideFlag::Normal, true),
        ("cognitive_low", Modality::Cognitive, RiskLevel::Low, BodyStance::Seated, ProximityEdge::Safe, OutputDest::Internal, OverrideFlag::Normal, true),
        ("render_critical", Modality::Render, RiskLevel::Critical, BodyStance::Moving, ProximityEdge::Danger, OutputDest::External, OverrideFlag::Normal, true),
    ];

    let vectors: Vec<_> = cases.into_iter().map(|(name, modality, risk, stance, edge, dest, override_flag, replay_enable)| {
        let pfp = PfpHeader::new(modality, risk, stance, edge, dest, override_flag, replay_enable);
        let encoded = pfp.encode();
        serde_json::json!({
            "id": format!("pfp-{}", name),
            "description": format!("PFP 编码: {:?}/{:?}/{:?}/{:?}", modality, risk, stance, edge),
            "input": {
                "modality": format!("{:?}", modality),
                "risk_level": format!("{:?}", risk),
                "body_stance": format!("{:?}", stance),
                "proximity_edge": format!("{:?}", edge),
                "output_dest": format!("{:?}", dest),
                "override_flag": format!("{:?}", override_flag),
                "replay_enable": replay_enable
            },
            "expected": {
                "encoded_hex": hex(&encoded),
                "encoded_bytes": encoded.to_vec(),
                "size": encoded.len(),
                "family_magic": "0xCF14"
            }
        })
    }).collect();

    serde_json::Value::Array(vectors)
}

// ─── 2. SAP 编解码测试向量 ──────────────────────────────────

fn generate_sap_vectors() -> serde_json::Value {
    let cases = vec![
        ("seq_zero", 0u16, [0u8; 14], [0u8; 8]),
        ("seq_max", 65535, [0xFF; 14], [0xFF; 8]),
        ("seq_42", 42, [0xAB; 14], [0xCD; 8]),
        ("seq_rotation_threshold", ROTATION_THRESHOLD, [0x11; 14], [0x22; 8]),
        ("seq_1000", 1000, [0xDE; 14], [0xAD; 8]),
    ];

    let vectors: Vec<_> = cases.into_iter().map(|(name, seq, pah_hash, pah_sig)| {
        let sap = SapHeader::new(seq, pah_hash, pah_sig);
        let encoded = sap.encode();
        serde_json::json!({
            "id": format!("sap-{}", name),
            "description": format!("SAP 编码: seq={}", seq),
            "input": {
                "seq_counter": seq,
                "pah_hash_hex": hex(&pah_hash),
                "pah_signature_hex": hex(&pah_sig)
            },
            "expected": {
                "encoded_hex": hex(&encoded),
                "encoded_bytes": encoded.to_vec(),
                "size": encoded.len(),
                "protocol_id": "0x01",
                "family_magic": "0xCF14"
            }
        })
    }).collect();

    serde_json::Value::Array(vectors)
}

// ─── 3. 完整帧编解码测试向量 ────────────────────────────────

fn generate_frame_vectors() -> serde_json::Value {
    let pfp = PfpHeader::new(
        Modality::Executive,
        RiskLevel::Medium,
        BodyStance::Moving,
        ProximityEdge::Warning,
        OutputDest::External,
        OverrideFlag::Normal,
        true,
    );
    let sap = SapHeader::new(42, [0xAB; 14], [0xCD; 8]);

    let cases = vec![
        ("v1_compat", None, None, vec![]),
        ("pfp_only", Some(pfp.clone()), None, vec![]),
        ("pfp_sap", Some(pfp.clone()), Some(sap.clone()), vec![]),
        ("pfp_sap_payload", Some(pfp.clone()), Some(sap.clone()), vec![0xEE; 64]),
        ("max_payload", Some(pfp.clone()), Some(sap.clone()), vec![0xFF; 1024]),
    ];

    let vectors: Vec<_> = cases.into_iter().map(|(name, p, s, payload)| {
        let frame = BindFrame::new(FrameType::Data, 1, 0, p.clone(), s.clone(), payload.clone()).unwrap();
        let encoded = frame.encode();
        serde_json::json!({
            "id": format!("frame-{}", name),
            "description": format!("完整帧编码: {}", name),
            "input": {
                "frame_type": "Data(0x01)",
                "channel_id": 1,
                "has_pfp": p.is_some(),
                "has_sap": s.is_some(),
                "payload_size": payload.len()
            },
            "expected": {
                "encoded_hex": hex(&encoded),
                "total_size": encoded.len(),
                "header_size": 8,
                "pfp_size": if p.is_some() { 4 } else { 0 },
                "sap_size": if s.is_some() { 28 } else { 0 },
                "payload_size": payload.len()
            }
        })
    }).collect();

    serde_json::Value::Array(vectors)
}

// ─── 4. 防重放测试向量 ──────────────────────────────────────

fn generate_replay_vectors() -> serde_json::Value {
    let cache = ReplayCache::new();
    let key = ReplayKey::new(1, 100);

    // 预填充: seq=100
    cache.check_and_update(key, 100);

    let cases = vec![
        ("normal_increment", 101, ReplayCheckResult::Allowed),
        ("exact_replay", 100, ReplayCheckResult::Rejected),
        ("old_seq", 50, ReplayCheckResult::Rejected),
        ("large_jump", 65000, ReplayCheckResult::Allowed),
        ("new_source", 1, ReplayCheckResult::Allowed), // 不同 source_id
    ];

    let vectors: Vec<_> = cases.into_iter().map(|(name, seq, expected)| {
        let test_key = if name == "new_source" {
            ReplayKey::new(1, 200) // 不同 source
        } else {
            key
        };
        // 注意: check_and_update 会修改缓存, 但这里只是为了验证我们的实现正确
        // 测试向量的期望结果是独立计算的: seq > last_seq(100) → Allowed, 否则 Rejected
        let result = cache.check_and_update(test_key, seq);
        serde_json::json!({
            "id": format!("replay-{}", name),
            "description": format!("防重放检查: seq={}, 预填充last_seq=100", seq),
            "input": {
                "tenant_id": 1,
                "source_id": if name == "new_source" { 200 } else { 100 },
                "seq_counter": seq,
                "last_seen_seq": 100
            },
            "expected": {
                "result": format!("{:?}", expected),
                "actual_result": format!("{:?}", result),
                "allowed": expected == ReplayCheckResult::Allowed,
                "rule": "seq > last_seen_seq → Allowed; seq <= last_seen_seq → Rejected; new source → Allowed"
            }
        })
    }).collect();

    serde_json::Value::Array(vectors)
}

// ─── 5. 规则6降级测试向量 ────────────────────────────────────

fn generate_rule6_vectors() -> serde_json::Value {
    let cases = vec![
        ("catastrophic_replay_disabled", RiskLevel::Catastrophic, false, RiskLevel::Medium),
        ("critical_replay_disabled", RiskLevel::Critical, false, RiskLevel::Medium),
        ("low_replay_enabled", RiskLevel::Low, true, RiskLevel::Low),
    ];

    let vectors: Vec<_> = cases.into_iter().map(|(name, original_risk, replay_enable, effective_risk)| {
        let pfp = PfpHeader::new(
            Modality::Executive,
            original_risk,
            BodyStance::Moving,
            ProximityEdge::CriticalEdge,
            OutputDest::External,
            OverrideFlag::HardOverride,
            replay_enable,
        );
        let effective = pfp.effective_risk_level();
        serde_json::json!({
            "id": format!("rule6-{}", name),
            "description": format!("规则6降级: 原始风险={:?}, Replay-Enable={}", original_risk, replay_enable),
            "input": {
                "original_risk_level": format!("{:?}", original_risk),
                "replay_enable": replay_enable
            },
            "expected": {
                "effective_risk_level": format!("{:?}", effective_risk),
                "actual_effective": format!("{:?}", effective),
                "downgraded": effective_risk != original_risk
            }
        })
    }).collect();

    serde_json::Value::Array(vectors)
}

// ─── 6. 密钥轮换测试向量 ────────────────────────────────────

fn generate_key_rotation_vectors() -> serde_json::Value {
    let mut sm = KeyRotationStateMachine::new();
    let nonce = [0x01; 12];
    let new_key = vec![0xCD; 32];
    let payload = bind19::rotation::KeyRotationPayload::new(nonce, new_key.clone());

    let vectors = vec![
        serde_json::json!({
            "id": "rotation-threshold-detection",
            "description": "密钥轮换阈值检测: Seq-Counter >= 65534 触发",
            "input": { "seq_counter": ROTATION_THRESHOLD },
            "expected": { "should_rotate": true, "threshold": ROTATION_THRESHOLD }
        }),
        {
            // 开始轮换
            sm.start_rotation(payload.clone()).unwrap();
            let state = sm.state();
            serde_json::json!({
                "id": "rotation-start-pending",
                "description": "开始轮换后状态: Pending (retries=0)",
                "input": { "action": "start_rotation" },
                "expected": { "state": format!("{:?}", state), "is_pending": true }
            })
        },
        {
            // ACK 成功
            sm.handle_ack().unwrap();
            let state = sm.state();
            serde_json::json!({
                "id": "rotation-ack-success",
                "description": "收到 ACK 后状态: Rotated",
                "input": { "action": "handle_ack" },
                "expected": { "state": format!("{:?}", state), "is_rotated": true }
            })
        },
        {
            // 完成轮换
            sm.complete_rotation().unwrap();
            let state = sm.state();
            serde_json::json!({
                "id": "rotation-complete-idle",
                "description": "完成轮换后状态: Idle (可开始下一次)",
                "input": { "action": "complete_rotation" },
                "expected": { "state": format!("{:?}", state), "is_idle": true }
            })
        },
    ];

    serde_json::Value::Array(vectors)
}

// ─── 7. CATASTROPHIC 检测测试向量 ───────────────────────────

fn generate_catastrophic_vectors() -> serde_json::Value {
    let cases = vec![
        ("catastrophic_override", RiskLevel::Catastrophic, OverrideFlag::HardOverride, true),
        ("catastrophic_no_override", RiskLevel::Catastrophic, OverrideFlag::Normal, false),
        ("critical_override", RiskLevel::Critical, OverrideFlag::HardOverride, false),
    ];

    let vectors: Vec<_> = cases.into_iter().map(|(name, risk, override_flag, expected)| {
        let pfp = PfpHeader::new(
            Modality::Executive,
            risk,
            BodyStance::Moving,
            ProximityEdge::CriticalEdge,
            OutputDest::External,
            override_flag,
            true,
        );
        let encoded = pfp.encode();
        let actual = catastrophic_manager_local(&encoded);
        serde_json::json!({
            "id": format!("catastrophic-{}", name),
            "description": format!("CATASTROPHIC检测: risk={:?}, override={:?}", risk, override_flag),
            "input": {
                "risk_level": format!("{:?}", risk),
                "override_flag": format!("{:?}", override_flag),
                "pfp_bytes_hex": hex(&encoded)
            },
            "expected": {
                "is_catastrophic_override": expected,
                "actual": actual
            }
        })
    }).collect();

    serde_json::Value::Array(vectors)
}

// 本地辅助函数（避免循环依赖）
fn catastrophic_manager_local(pfp_bytes: &[u8; 4]) -> bool {
    let risk_level = (pfp_bytes[2] >> 2) & 0b11;
    let override_flag = (pfp_bytes[3] >> 1) & 0b1;
    risk_level == 3 && override_flag == 1
}

// ─── 8. PAH 签名测试向量 ────────────────────────────────────

fn generate_pah_signature_vectors() -> serde_json::Value {
    let keypair = KeyPair::from_seed(&[0x42; 32]); // 固定种子，可复现
    let message = b"CI-144 v2.0 Physical Anchor Layer test message";

    let full_sig = sign(&keypair, message);
    let truncated = truncate_signature(&full_sig);
    let truncated_direct = sign_truncated(&keypair, message);

    let wrong_message = b"wrong message";
    let wrong_sig = sign(&keypair, wrong_message);

    let vectors = vec![
        serde_json::json!({
            "id": "pah-full-signature",
            "description": "完整 Ed25519 签名（64字节），固定种子可复现",
            "input": {
                "seed_hex": hex(&[0x42; 32]),
                "message": String::from_utf8_lossy(message),
                "message_hex": hex(message)
            },
            "expected": {
                "full_signature_hex": hex(&full_sig),
                "signature_size": full_sig.len(),
                "algorithm": "Ed25519"
            }
        }),
        serde_json::json!({
            "id": "pah-truncated-signature",
            "description": "64-bit 截断签名（8字节）= SHA-256(完整签名) 前64位(MSB)",
            "input": {
                "full_signature_hex": hex(&full_sig)
            },
            "expected": {
                "truncated_signature_hex": hex(&truncated),
                "truncated_size": truncated.len(),
                "truncation_algorithm": "SHA-256(full_sig)[0..8] (MSB, high 64 bits)",
                "matches_direct_sign_truncated": truncated == truncated_direct
            }
        }),
        serde_json::json!({
            "id": "pah-wrong-signature-rejected",
            "description": "错误消息的签名截断值不匹配（第一层快速拒绝）",
            "input": {
                "expected_truncated_hex": hex(&truncated),
                "actual_message": String::from_utf8_lossy(wrong_message),
                "actual_full_signature_hex": hex(&wrong_sig)
            },
            "expected": {
                "actual_truncated_hex": hex(&truncate_signature(&wrong_sig)),
                "matches": false,
                "should_reject": true
            }
        }),
    ];

    serde_json::Value::Array(vectors)
}
