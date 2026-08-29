//! CI-144 v2.0 基本帧使用示例
//!
//! 展示如何创建、编码、解码 BIND-19 v2.0 帧
//!
//! 运行方式：`cargo run --example basic_frame`

use bind19::frame::{BindFrame, FrameType};
use bind19::pfp::{
    BodyStance, Modality, OutputDest, OverrideFlag, PfpHeader, ProximityEdge, RiskLevel,
};
use bind19::sap::SapHeader;

fn main() {
    println!("=== CI-144 v2.0 基本帧使用示例 ===\n");

    // 1. 创建 PFP 头部（4 字节物理特征协议）
    let pfp = PfpHeader::new(
        Modality::Executive,
        RiskLevel::Medium,
        BodyStance::Standing,
        ProximityEdge::Warning,
        OutputDest::External,
        OverrideFlag::Normal,
        true, // Replay-Enable
    );
    let pfp_bytes = pfp.encode();
    println!("1. PFP 头部（{} 字节）:", pfp_bytes.len());
    println!("   编码: {:02x?}", pfp_bytes);
    println!("   Modality: {:?}", pfp.modality);
    println!("   Risk-Level: {:?}", pfp.risk_level);
    println!("   Family-Magic: 0x{:04x}", u16::from_be_bytes([pfp_bytes[0], pfp_bytes[1]]));
    println!();

    // 2. 创建 SAP 头部（28 字节安全证明协议）
    let sap = SapHeader::new(
        42,                    // Seq-Counter
        [0xAB; 14],            // PAH-Hash（SHA-256 截断 112 位）
        [0xCD; 8],             // PAH-Signature（64-bit 截断签名）
    );
    let sap_bytes = sap.encode();
    println!("2. SAP 头部（{} 字节）:", sap_bytes.len());
    println!("   编码: {:02x?}", &sap_bytes[..8]);
    println!("   Seq-Counter: {}", sap.seq_counter);
    println!();

    // 3. 创建完整帧（BIND-19 头 + PFP + SAP + Payload）
    let payload = b"Hello, CI-144 v2.0!".to_vec();
    let frame = BindFrame::new(
        FrameType::Data,
        1,                     // channel_id
        0,                     // sequence_id
        Some(pfp.clone()),
        Some(sap.clone()),
        payload.clone(),
    )
    .expect("帧创建失败");

    let encoded = frame.encode();
    println!("3. 完整帧（{} 字节）:", encoded.len());
    println!("   头部: 8 字节");
    println!("   PFP: 4 字节");
    println!("   SAP: 28 字节");
    println!("   Payload: {} 字节", payload.len());
    println!("   总大小: {} 字节", encoded.len());
    println!("   前16字节: {:02x?}", &encoded[..16]);
    println!();

    // 4. 解码帧
    let decoded = BindFrame::decode(&encoded).expect("帧解码失败");
    println!("4. 解码帧:");
    println!("   Frame-Type: {:?}", decoded.header.frame_type);
    println!("   Channel-ID: {}", decoded.header.channel_id);
    println!("   有 PFP: {}", decoded.pfp.is_some());
    println!("   有 SAP: {}", decoded.sap.is_some());
    println!("   Payload 长度: {}", decoded.payload.len());
    println!("   Payload: {:?}", String::from_utf8_lossy(&decoded.payload));
    println!();

    // 5. v1.0 兼容帧（无 PFP/SAP）
    let v1_frame = BindFrame::new(
        FrameType::Data,
        1,
        0,
        None,  // 无 PFP
        None,  // 无 SAP
        b"v1.0 compatible frame".to_vec(),
    )
    .expect("v1.0 帧创建失败");
    let v1_encoded = v1_frame.encode();
    println!("5. v1.0 兼容帧（{} 字节）:", v1_encoded.len());
    println!("   无 PFP/SAP，仅 8 字节头部 + Payload");
    println!("   向后兼容: v1.0 接收端可正常解析");
    println!();

    // 6. 节能模式帧（PFP-only，无 SAP）
    let eco_frame = BindFrame::new(
        FrameType::Data,
        1,
        0,
        Some(pfp.clone()),
        None,  // 无 SAP，节能模式
        b"eco mode frame".to_vec(),
    )
    .expect("节能帧创建失败");
    let eco_encoded = eco_frame.encode();
    println!("6. 节能模式帧（{} 字节）:", eco_encoded.len());
    println!("   PFP-only，无 SAP（节省 28 字节）");
    println!("   Replay-Enable 自动设为 0，规则6降级至 MEDIUM");
    println!();

    println!("=== 示例完成 ===");
    println!();
    println!("关键常量:");
    println!("  Family-Magic: 0xCF14");
    println!("  PFP-Size: 4 字节（冻结层）");
    println!("  SAP-Size: 28 字节（演进层）");
    println!("  BIND-19-Header: 8 字节");
}
