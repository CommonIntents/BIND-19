# BIND-19 协议白皮书

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0) [![Version](https://img.shields.io/badge/Version-0.1.0--draft-orange.svg)]() [![Status](https://img.shields.io/badge/Status-RFC%20Draft-yellow.svg)]() [![Org](https://img.shields.io/badge/Org-CommonIntents-144-darkgray.svg)](https://github.com/CommonIntents)

## INTENT-7/传输绑定协议

**版本**: v0.1.0 草案
**日期**: 2026-05-21
**状态**: 工作组内部草案
**许可证**：Apache 2.0

---

## 一、核心定位

BIND-19（INTENT-7 Binding）是**CIS与具体传输实现之间的适配层**。

它的唯一职责是：定义CIS意图如何安全、高效、完整地承载于具体的传输协议之上。

CIB是协议栈的**韧带**——灵活、轻薄、可替换。

---

## 二、为何需要CIB

CIS是传输无关的语义标准。加密算法会演进，传输协议会更替，序列化格式会更新。如果这些变化需要修改CIS，CIS的寿命就绑定在传输层上。

CIB将所有传输相关的决策隔离在一层。加密技术演进时，仅需更新CIB的绑定目标。CIS本身不受任何影响。

---

## 三、核心职责

### 3.1 传输格式协商

在握手阶段，双方通过`Content-Type`和`Accept`头协商传输格式。

```
客户端请求:
  Content-Type: application/cic13+msgpack
  Accept: application/cic13+msgpack, application/cic13+json

服务端响应:
  Content-Type: application/cic13+msgpack
```

**默认二进制，兼容JSON。** 任一方不支持二进制则自动降级为JSON。人类需审查时可按需请求JSON端点，不影响生产环境二进制极速路径。

二进制转JSON为O(n)格式化操作，微秒级完成。

### 3.2 完整性校验协商

CIB在帧层引入可选的完整性校验。

```
CIB帧 = 帧头 (类型+长度) + 载荷 + 帧尾 (CRC32)
```

**协商机制：**

```
客户端请求:
  X-BIND-19-Integrity: crc32

服务端响应:
  X-BIND-19-Integrity: crc32
```

双方协商一致则启用。任一方不支持或不同意则跳过。**按需开启，默认关闭。**

当底层使用CISS（mTLS）时，TLS已在传输层提供完整性保护，应用层校验可协商关闭，避免冗余计算。非加密传输场景（如本地进程间通信）可协商开启。

### 3.3 版本兼容性声明

CIB在握手阶段声明自身版本及绑定目标。

```
X-BIND-19-Version: 0.1.0
X-BIND-19-Binding: INTENT-7-SECURE/1.0
```

---

## 四、当前绑定目标

CIB当前将CIS绑定到CISS（mTLS over HTTPS）。未来可能出现的绑定目标包括：

- **INTENT-7-SECURE-QUIC**：基于QUIC的安全传输
- **INTENT-7-SECURE-PQC**：基于后量子密码的安全传输
- **INTENT-7-SECURE-Local**：本地进程间通信（零网络开销）

所有绑定目标共享相同的CIB协商机制。更换绑定目标时，上层CIS和CAP不受任何影响。

---

## 五、协议边界

BIND-19 **负责**：
- 定义传输格式的协商机制
- 定义完整性校验的协商机制
- 定义版本兼容性的声明格式
- 定义CIB帧结构

BIND-19 **不负责**：
- 规定必须使用哪种传输协议（由应用选择）
- 规定必须使用哪种加密算法（由传输实现决定）
- 规定必须使用哪种二进制格式（协商决定）
- 传输层本身的安全保证（由CISS或替代实现提供）

---

## 六、与CISS的关系

CIB是**规范**，CISS是**实现**。

CIB定义：“意图数据通过协商后的格式，承载于安全的传输信道之上。”

CISS实现：“这条安全信道是mTLS over HTTPS。”

未来CISS-PQC实现：“这条安全信道是mTLS with post-quantum cryptography over HTTPS。”

CIB不变，CIS不变，CAP不变。只有传输实现的版本号在变。

---

## 七、帧结构定义

### 7.1 无完整性校验

```
帧头 (1字节类型 + 4字节长度) + 载荷
```

### 7.2 有完整性校验（协商开启）

```
帧头 (1字节类型 + 4字节长度) + 载荷 + 帧尾 (4字节CRC32)
```

CRC32覆盖帧头+载荷的全部字节。

---

## 八、协议边界再确认

CIB是协议栈中最薄的一层。它的存在不是为了增加新功能，而是为了隔离变化。它不定义任何新的交互语义，不引入任何新的安全机制，不绑定任何具体的传输实现。

**CIB的存在，是为了让CIS永远不需要知道自己跑在什么传输层上。**

---

*本白皮书由CIS/CAP协议工作组维护。*
