//! BIND-19 基准测试（v2.0-beta B4 完善）
//!
//! 当前为基本框架，B4 任务将完善以下基准：
//! - 防重放缓存 QPS（check_and_update）
//! - 帧编解码延迟（PFP/SAP/完整帧）
//! - PAH 签名验证延迟（ed25519 + SHA-256 截断）
//! - CATASTROPHIC 事件总线延迟

use criterion::{criterion_group, criterion_main, Criterion};
use bind19::replay_cache::{ReplayCache, ReplayKey, ReplayCheckResult};

fn bench_replay_cache_check_and_update(c: &mut Criterion) {
    let cache = ReplayCache::new();
    let key = ReplayKey::new(1, 100);
    let mut seq: u16 = 0;

    c.bench_function("replay_cache check_and_update", |b| {
        b.iter(|| {
            seq = seq.wrapping_add(1);
            let result = cache.check_and_update(key, seq);
            // 确保结果被使用，避免编译器优化
            assert!(matches!(result, ReplayCheckResult::Allowed | ReplayCheckResult::Rejected));
        })
    });
}

criterion_group!(benches, bench_replay_cache_check_and_update);
criterion_main!(benches);
