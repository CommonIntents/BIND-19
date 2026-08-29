//! BIND-19 v2.0 基准测试
//!
//! 覆盖：
//! 1. 防重放缓存 check_and_update QPS
//! 2. 帧编解码延迟（PFP+SAP 完整帧）
//! 3. PAH 签名验证延迟（ed25519 + SHA-256 截断）
//! 4. CATASTROPHIC 事件检测延迟

use criterion::{criterion_group, criterion_main, Criterion, black_box};
use bind19::catastrophic::CatastrophicManager;
use bind19::crypto::{KeyPair, sign, verify, sign_truncated, truncate_signature, verify_truncated_match};
use bind19::frame::{BindFrame, FrameType};
use bind19::pfp::{
    BodyStance, Modality, OutputDest, OverrideFlag, PfpHeader, ProximityEdge, RiskLevel,
};
use bind19::replay_cache::{ReplayCache, ReplayKey};
use bind19::sap::SapHeader;

// ─── 基准 1：防重放缓存 check_and_update ────────────────────

fn bench_replay_cache_check_and_update(c: &mut Criterion) {
    let cache = ReplayCache::new();
    let key = ReplayKey::new(1, 100);
    // 预填充缓存
    cache.check_and_update(key, 0);

    c.bench_function("replay_cache check_and_update (hit)", |b| {
        let mut seq: u16 = 1;
        b.iter(|| {
            seq = seq.wrapping_add(1);
            let result = cache.check_and_update(key, seq);
            black_box(result);
        })
    });

    // 新源（缓存未命中）
    c.bench_function("replay_cache check_and_update (miss/new)", |b| {
        let mut source_id: u64 = 1000;
        b.iter(|| {
            source_id += 1;
            let key = ReplayKey::new(1, source_id);
            let result = cache.check_and_update(key, 1);
            black_box(result);
        })
    });

    // 重放拒绝（seq ≤ last_seq）
    c.bench_function("replay_cache check_and_update (replay reject)", |b| {
        let key = ReplayKey::new(1, 9999);
        cache.check_and_update(key, 100);
        b.iter(|| {
            let result = cache.check_and_update(key, 50);
            black_box(result);
        })
    });
}

// ─── 基准 2：帧编解码 ────────────────────────────────────────

fn make_test_frame() -> BindFrame {
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
    BindFrame::new(FrameType::Data, 1, 0, Some(pfp), Some(sap), vec![0xEE; 64]).unwrap()
}

fn bench_frame_encode(c: &mut Criterion) {
    let frame = make_test_frame();
    c.bench_function("frame encode (PFP+SAP+64B payload)", |b| {
        b.iter(|| {
            let encoded = frame.encode();
            black_box(encoded);
        })
    });
}

fn bench_frame_decode(c: &mut Criterion) {
    let frame = make_test_frame();
    let encoded = frame.encode();

    c.bench_function("frame decode (PFP+SAP+64B payload)", |b| {
        b.iter(|| {
            let decoded = BindFrame::decode(&encoded).unwrap();
            black_box(decoded);
        })
    });
}

fn bench_frame_encode_decode_roundtrip(c: &mut Criterion) {
    let frame = make_test_frame();
    c.bench_function("frame encode+decode roundtrip", |b| {
        b.iter(|| {
            let encoded = frame.encode();
            let decoded = BindFrame::decode(&encoded).unwrap();
            black_box(decoded);
        })
    });
}

// ─── 基准 3：PAH 签名验证 ───────────────────────────────────

fn bench_pah_signature(c: &mut Criterion) {
    let keypair = KeyPair::generate();
    let message = b"CI-144 v2.0 physical anchor layer test message for benchmarking";

    // 完整签名生成
    c.bench_function("ed25519 full sign (64B)", |b| {
        b.iter(|| {
            let sig = sign(&keypair, message);
            black_box(sig);
        })
    });

    // 完整签名验证
    let full_sig = sign(&keypair, message);
    let public_key = keypair.public_key();
    c.bench_function("ed25519 full verify (64B)", |b| {
        b.iter(|| {
            let result = verify(&public_key, message, &full_sig);
            black_box(result);
        })
    });

    // 64-bit 截断签名生成
    c.bench_function("PAH 64-bit truncated sign", |b| {
        b.iter(|| {
            let truncated = sign_truncated(&keypair, message);
            black_box(truncated);
        })
    });

    // 64-bit 截断匹配验证（第一层快速拒绝）
    let truncated = truncate_signature(&full_sig);
    c.bench_function("PAH 64-bit truncated verify (match)", |b| {
        b.iter(|| {
            let result = verify_truncated_match(&full_sig, &truncated);
            black_box(result);
        })
    });
}

// ─── 基准 4：CATASTROPHIC 事件检测 ──────────────────────────

fn bench_catastrophic_detection(c: &mut Criterion) {
    let (manager, _receiver) = CatastrophicManager::new();

    // CATASTROPHIC 帧（触发事件）
    let pfp_catastrophic = PfpHeader::new(
        Modality::Executive,
        RiskLevel::Catastrophic,
        BodyStance::Moving,
        ProximityEdge::CriticalEdge,
        OutputDest::External,
        OverrideFlag::HardOverride,
        true,
    );
    let catastrophic_bytes = pfp_catastrophic.encode();

    // 正常帧（不触发事件）
    let pfp_normal = PfpHeader::new(
        Modality::Cognitive,
        RiskLevel::Low,
        BodyStance::Unknown,
        ProximityEdge::Safe,
        OutputDest::Internal,
        OverrideFlag::Normal,
        true,
    );
    let normal_bytes = pfp_normal.encode();

    // 纯检测（is_catastrophic_override，不触发事件）
    c.bench_function("catastrophic detect (pure check)", |b| {
        b.iter(|| {
            let result = CatastrophicManager::is_catastrophic_override(&catastrophic_bytes);
            black_box(result);
        })
    });

    // 正常帧检测（不触发）
    c.bench_function("catastrophic detect (normal frame, no trigger)", |b| {
        b.iter(|| {
            let result = manager.handle_frame(&normal_bytes, Some(1), None, "bench");
            black_box(result);
        })
    });

    // CATASTROPHIC 帧处理（触发事件 + 审计日志）
    // 注意：这个基准会不断发送事件，接收端需要及时消费
    c.bench_function("catastrophic handle (trigger event + audit)", |b| {
        b.iter(|| {
            let result = manager.handle_frame(&catastrophic_bytes, Some(1), None, "bench");
            black_box(result);
        })
    });
}

// ─── 基准组 ─────────────────────────────────────────────────

criterion_group!(
    benches,
    bench_replay_cache_check_and_update,
    bench_frame_encode,
    bench_frame_decode,
    bench_frame_encode_decode_roundtrip,
    bench_pah_signature,
    bench_catastrophic_detection,
);
criterion_main!(benches);
