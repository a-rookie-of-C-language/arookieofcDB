# arookieofcDB

一个基于 Rust 构建的、高性能、可落盘的存储引擎。本项目旨在深度探索分布式缓存与数据库数据强一致性问题，并提供了一套从内存缓存到磁盘 B+ 树的完整解决方案。

> [!NOTE]
> 本项目目前正处于从“全内存存储”向“基于页式管理的物理存储引擎（类 InnoDB 架构）”转型的核心阶段。

## 🌟 核心愿景
在传统的后端架构中，缓存（Redis）与数据库（MySQL）的不一致性始终是痛点。`arookieofcDB` 通过**混合存储架构 (Hybrid Store)** 与 **WAL (预写日志)** 机制，尝试在应用层解决这一问题。

## 🏗️ 架构设计

### 1. Java 风格的工程组织
为了让习惯 Java 生态（如 Spring Boot）的开发者能够无缝阅读 Rust 代码，本项目采用了严格的 **"One Struct/Trait per File"** 原则。
*   每个 Struct 或 Trait 都有其独立的 `.rs` 文件。
*   通过 `mod.rs` 门面模式进行导包管理。
*   目录结构分层清晰：`storage` (存储层)、`engine` (引擎层)、`commands` (命令层)。

### 2. 类 InnoDB 的物理存储引擎
本项目正在实现一套完整的物理页存储管理系统：
*   **Buffer Pool Manager**: 采用 LRU 算法管理 4KB 物理页，在有限内存下支持海量数据读取。
*   **Page-Oriented B+Tree**: 所有的索引和数据跳转均基于 `PageId` 而非内存指针，支持真正的物理落盘（`.data` 文件）。
*   **二进制协议**: 严格定义了 `LeafPage` (叶子页) 和 `InternalPage` (内部页) 的二进制布局，确保跨平台的数据一致性。

### 3. 多层次存储模型
*   **Memory Store**: 基于精简 Hash 表的高速内存缓存。
*   **Wal Store**: 具备数据回放能力的持久化存储，支持 Redo Log 和 Snapshot。
*   **Hybrid Store**: 将内存的极速与磁盘的可靠结合，支持自动的**读修复 (Read Repair)**。

## 🚀 快速开始

### 环境要求
*   Rust 2021 Edition 或更高版本
*   Cargo

### 编译与运行
```bash
# 模式启动
cargo run

# 运行测试套件
cargo test
```

### 示例交互
你可以通过命令行输入类似 Redis 的指令：
```bash
> set user:1 "arookie"
> get user:1
> incr counter
> select --disk user:1  # 强制从物理磁盘读取
```

## 🛠️ 当前进度
- [x] 工程结构 Java 化改造 (40+ 文件拆分)
- [x] 实现缓冲池管理器 (Buffer Pool Manager)
- [x] 重构物理页 B+ 树基础架构 (Page-based B+Tree)
- [ ] B+ 树节点分裂与合并逻辑 (Split/Merge)
- [ ] 实现 MVCC 与事务管理器 (Transaction Manager)
- [ ] 异步 Binlog 无效化缓存机制

## 🤝 参与贡献
如果你对数据库底层的物理实现、缓冲池调度或一致性协议感兴趣，欢迎提交 Pull Request 或 Issue。
