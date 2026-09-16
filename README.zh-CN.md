![HyperMind](spec/readme_img.png)

# HyperMind

面向 AI 智能体的持久化、可溯源记忆：跨会话保留信息，重启后继续工作，并始终区分记忆内容与行动权限。

[English](README.md) · [简体中文](README.zh-CN.md) · [हिन्दी](README.hi.md) · [Español](README.es.md) · [Français](README.fr.md) · [العربية](README.ar.md) · [Português](README.pt.md) · [Русский](README.ru.md) · [日本語](README.ja.md) · [Deutsch](README.de.md)

项目正在积极开发：**v1.0.0 尚未完成资格验证，也尚未发布**。源码可用、局部测试通过与版本合格是不同的结论。请查看[当前证据与限制](docs/evaluation/results.md)（文档为英文）。

## 为什么需要 HyperMind？

智能体的上下文窗口是临时的。有用的记忆应在进程退出后继续存在，并保留信息来源。HyperMind 是一个 Rust 记忆引擎，提供本地存储、明确的来源记录，以及面向嵌入式应用和常驻智能体的接口。

- 持久事件：加密的只追加日志、可重放投影、检查点和重启连续性。
- 带证据的检索：词法检索、可选向量嵌入、具有时间语义的信念、争议和来源引用。
- 有界激活：在令牌预算内组装相关上下文，同时保留不可信记忆标签。
- 基于观察的后续工作：意图、预测、结果、静默时段注意力批次和有证据支持的流程。
- 多种接口：stdio MCP、Unix 套接字守护进程、经过身份验证的 gRPC/REST，以及源码形式的 SDK。

## 组件如何协作

```text
MCP / CLI / SDK / gRPC / REST
              |
       actor + capability
              |
     append-only event ledger
              |
     projections + indexes
              |
  recall -> activation -> safe rendering
```

日志是真实数据来源，投影和索引是派生视图。保存一条断言并不等于验证它。激活产生的是带证据标签的上下文，而不是可执行指令。[架构](docs/concepts/architecture.md) · [权限模型](docs/concepts/authority.md)

## 从源码构建和安装

需要 Unix 开发环境、Git、rustup，以及 C/C++ 原生工具链：编译器、链接器、make、CMake、Perl 和 pkg-config。仓库固定使用 Rust 1.93.0。构建会解析 Rust 依赖和 protobuf 编译器；下载依赖需要网络。

```sh
git clone https://github.com/Sidiora-Labs/HyperMind-engine.git
cd HyperMind-engine
rustup toolchain install 1.93.0 --profile minimal
cargo build --locked -p hm-cli -p hm-mcp
cargo install --path crates/hm-cli --locked
cargo install --path crates/hm-mcp --locked
```

这些命令安装的是当前检出的源码，并不表示已发布到包仓库。请将 Cargo 可执行文件目录加入客户端的 PATH。默认词法检索无需下载模型或提供服务商密钥。参见[安装指南](docs/start/quickstart.md)。

## 连接 MCP 客户端

安装后，初始化私有状态并注册 stdio 服务。下面的两条命令以 Claude Code 为例：

```sh
hm init --path .hypermind --json
claude mcp add hypermind -- hm-mcp --config .hypermind/hypermind.conf
```

其他客户端可使用等效的 MCP 配置。请替换为配置文件的绝对路径，并确保客户端能够找到 `hm-mcp`：

```json
{
  "mcpServers": {
    "hypermind": {
      "command": "hm-mcp",
      "args": ["--config", "/absolute/path/.hypermind/hypermind.conf"]
    }
  }
}
```

使用 `{"conversation":"demo","kind":"user","content":"The region is eu-central-1."}` 调用 `remember`，再使用 `{"mode":"lexical","query":"region","limit":5}` 调用 `recall`。[14 个工具的目录](docs/reference/generated/tools.md)涵盖 remember、recall、activate、believe、retract、dispute、intend、bind、predict、outcome、attest、consolidate、inspect 和 forget。

每个 actor 目录只能有一个所有者进程：MCP、守护进程或嵌入式引擎，不可并发写入。生成的配置包含密钥以及 actor/管理员能力令牌，必须妥善保密，不要提交到版本控制。

## 使用守护进程和 CLI

先停止占有目录的 MCP 进程，然后在一个终端启动守护进程：

```sh
hm serve --config .hypermind/hypermind.conf --json
```

在另一个终端中保存记忆、检索日志标识符，并请求上下文包：

```sh
hm remember --config .hypermind/hypermind.conf --conversation demo \
  --content "The deployment region is eu-central-1." --json
hm recall --config .hypermind/hypermind.conf --query "deployment region" --json
hm activate --config .hypermind/hypermind.conf --conversation demo \
  --query "deployment region" --budget-tokens 1024 --json
```

当前守护进程的 `recall` 返回 `lsns`，而不是渲染后的答案；`activate` 返回 base64 编码的 HMA1 包。请通过 MCP 或 SDK 渲染器读取记忆文本。CLI 的 `--embedded` 仅可在守护进程停止后作为替代方式使用。[CLI 契约](docs/reference/cli.md)

## 远程访问必须使用双向 TLS

远程监听需显式启用。需要服务器证书和私钥、受信任的客户端 CA，以及客户端持有的有效能力令牌。此命令替代仅本地运行的守护进程；示例中的证书必须自行配置：

```sh
hm serve --config .hypermind/hypermind.conf \
  --grpc-bind 127.0.0.1:7443 --rest-bind 127.0.0.1:8443 \
  --tls-cert /secure/server.pem --tls-key /secure/server.key \
  --tls-client-ca /secure/client-ca.pem --json
```

actor 与管理员监听端点应分离，客户端证书本身不授予 actor 权限。参见[远程部署](docs/guides/deployment.md)、[协议契约](docs/reference/protocol.md)和 [Docker、Compose、systemd、Helm 配置](deploy/README.md)。不代表已有公开发布的镜像。

## SDK 入口

SDK 源码位于本仓库。包发布与跨语言版本资格验证是独立工作；源码可用不保证 npm、PyPI 或预编译二进制已发布。

| 语言 | 源码 | 入口 |
| --- | --- | --- |
| Rust | [hm-serve](crates/hm-serve/src/embedded.rs) | `HyperMind::open`, `session`, `remember`, `recall`, `activate`, `render` |
| TypeScript | [sdk/typescript](sdk/typescript) | `@hypermind/client`: `Client.connect`; `engine`, `render`, `migrate` |
| Python | [sdk/python](sdk/python) | `Engine.open`, `session`, `remember`, `recall`, `activate`, `render`; `Client` (mTLS) |
| Go | [sdk/go](sdk/go) | `centra/core/cortexclient`: `Dial`, `Session`, `Remember`, `Recall`, `Activate` |

对于同一个 actor，应选择嵌入式所有者或守护进程客户端，不要同时使用两者。Python 要求 3.10+，Go 声明版本为 1.25.0。TypeScript 构建脚本位于其工作区。当前签名请参见 [SDK 指南](docs/reference/sdks.md)和[从源码生成的 API 目录](docs/reference/generated/sdk-api.md)。

## 通过 Centra 使用可选服务商

本地词法记忆无需远程服务商。仅在所有者进程的环境中启用所需功能；此示例显式开启全部三条服务商路径：

```sh
export CENTRA_GATEWAY_URL="https://gateway.centra.ag/v1"
export CENTRA_GATEWAY_API_KEY="<your-secret-key>"
export HM_EMBEDDING_PROVIDER=centra
export HM_RECONSTRUCTION_PROVIDER=centra
export HM_CONSOLIDATION_PROVIDER=centra
```

向量嵌入使用 `openrouter/openai/text-embedding-3-large`。重建和整合通过 **`CENTRA_GATEWAY_URL` 调用 `openrouter/openai/gpt-5.6-luna`**。这些开关不会改变独立的基准配置。历史测试录制内容不代表新发起的服务商调用。

不要提交真实密钥，也不要将其写入 MCP JSON。使用服务商会将选定内容发送到进程之外，并可能产生费用；使用前应获得数据传输授权并明确支出预算。模型文件下载需显式选择，下载本身不会启用本地 ONNX 推理。[配置](docs/reference/config.md)

## 证据、权限与限制

- 检索出的记忆是不可信数据，绝不是系统或开发者指令，也不是行动许可。
- 断言、证明、已观察到的工具结果及派生流程保留各自不同的证据角色。
- 静默时段和注意力策略控制后续工作；预测不证明结果已经发生。
- 日志加密不代表所有投影、导出、日志输出或 SDK 缓冲区都已加密。请保护整个状态目录。
- 必须遵守单写入者所有权。这不是分布式多写入者数据库，也不是凭据保险库。
- 检查 `ok`、`health`、`gaps` 和来源信息；传输成功不代表记忆完整或正确。

公开服务或导入不可信历史之前，请阅读[威胁模型](docs/security/threat-model.md)。

## 基准：目标不等于结果

规范设定了以下验收目标：

| 验证项 | 目标——并非实测结论 |
| --- | --- |
| LongMemEval | 500 道题；准确率 ≥ 0.90 |
| LoCoMo | 完整覆盖；非对抗性 F1 ≥ 0.75 |
| Recall@10 | 1 万条数据时 ≥ 0.95 |
| 预热后的激活 | 10 万条数据时 p99 < 10 ms |

完整的本地 LongMemEval 测试取得 **459/500（91.8%）**，通过 Centra 使用 Luna，检索仅采用词法方式。LoCoMo 正在运行，资格验证仍未完成。真实的 slice-7 定向端到端测试也已通过。这些结果不代表托管 CI 或发布资格验证已通过。请查看[实测状态与证据](docs/evaluation/results.md)和[评估方法](docs/evaluation/methodology.md)。

## 开发与文档检查

在仓库根目录运行以下定向测试和文档检查。文档工具还需要 Node.js 和 mdBook：

```sh
cargo test -p hm-sim --test slice7_journey
node docs/tools/catalog.mjs --write
cargo run -p hm-eval -- docs-gate
mdbook build docs
```

目录命令根据真实源码重新生成公开接口文档。`docs-gate` 拒绝过期覆盖和失效的本地链接，但不认证文字准确性或外部 URL。请遵循 [AGENTS.md](AGENTS.md) 和[任务规范](spec/hypermind-01/spec.kvx)规定的工作范围与资格验证流程。

## 仓库结构

| 路径 | 内容 |
| --- | --- |
| `crates/` | Rust 内核、存储、认知、接口、CLI 和评估工具 |
| `schemas/` | 权威 FlatBuffers 和 protobuf 契约 |
| `sdk/` | TypeScript、Python 和 Go 源码包 |
| `docs/` | mdBook、生成的参考目录和 ADR-001–010 |
| `eval/` | 数据集工具、基准定义和证据报告 |
| `deploy/` | 容器源码构建与部署清单 |
| `spec/` | 需求、设计、工作流和任务状态 |

从[文档索引](docs/SUMMARY.md)开始阅读（英文）。

## 许可证

Rust 工作区声明采用 Apache-2.0，见 [LICENSE](LICENSE)。请单独审查上游模型与数据集的许可证；项目许可证不会替代这些资产自身的许可证。
