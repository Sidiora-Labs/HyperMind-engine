![HyperMind](spec/readme_img.png)

# HyperMind

AI エージェントのための、永続的で根拠を追跡できるメモリ。セッションを越えて情報を保持し、再起動後に再開し、記憶と権限を混同しません。

[English](README.md) · [简体中文](README.zh-CN.md) · [हिन्दी](README.hi.md) · [Español](README.es.md) · [Français](README.fr.md) · [العربية](README.ar.md) · [Português](README.pt.md) · [Русский](README.ru.md) · [日本語](README.ja.md) · [Deutsch](README.de.md)

開発中です。**v1.0.0 は適格性検証もリリースも完了していません**。ソース公開、個別テストの成功、リリース適格性は異なる主張です。[現在の根拠と制限](docs/evaluation/results.md)を参照してください（詳細文書は英語）。

## HyperMind が解決すること

エージェントのコンテキストウィンドウは一時的です。有用なメモリはプロセス終了後も残り、情報の出所を保持する必要があります。HyperMind はローカル保存、明示的な来歴、組み込みアプリと常駐エージェント向けインターフェースを備えた Rust 製メモリエンジンです。

- 永続イベント：暗号化された追記専用台帳、再生可能なプロジェクション、チェックポイント、再起動後の継続。
- 根拠付き検索：語彙検索、任意の埋め込み、時間を考慮した信念、異議、出所参照。
- 上限付きアクティベーション：トークン予算内で関連コンテキストを組み立て、非信頼メモリのラベルを保持。
- 観測に基づく継続作業：意図、予測、結果、静穏時間帯の通知集約、根拠のある手順。
- 複数の接続面：stdio MCP、Unix ソケットのデーモン、認証付き gRPC/REST、ソース提供の SDK。

## 構成とデータの流れ

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

台帳が正本であり、プロジェクションとインデックスは派生ビューです。主張の保存は、その検証を意味しません。アクティベーションが生成するのは根拠を示したコンテキストであり、実行命令ではありません。[アーキテクチャ](docs/concepts/architecture.md) · [権限モデル](docs/concepts/authority.md)

## ソースからビルドしてインストール

Unix 開発環境、Git、rustup、および C/C++ ネイティブツールチェーン（コンパイラ、リンカ、make、CMake、Perl、pkg-config）が必要です。Rust は 1.93.0 に固定されています。Rust 依存関係と protobuf コンパイラはビルド時に解決され、ダウンロードにはネットワークが必要です。

```sh
git clone https://github.com/Sidiora-Labs/HyperMind-engine.git
cd HyperMind-engine
rustup toolchain install 1.93.0 --profile minimal
cargo build --locked -p hm-cli -p hm-mcp
cargo install --path crates/hm-cli --locked
cargo install --path crates/hm-mcp --locked
```

これは手元のソースをインストールするコマンドであり、レジストリ公開済みパッケージを示すものではありません。Cargo の実行ファイルディレクトリをクライアントの PATH に追加してください。標準の語彙検索にはモデルのダウンロードもプロバイダーキーも不要です。[インストール手順](docs/start/quickstart.md)

## MCP クライアントを接続

インストール後、非公開の状態領域を初期化し、stdio サーバーを登録します。Claude Code を使う 2 コマンドの例です：

```sh
hm init --path .hypermind --json
claude mcp add hypermind -- hm-mcp --config .hypermind/hypermind.conf
```

ほかのクライアントでは同等の MCP 設定を使用します。設定ファイルを絶対パスに置き換え、クライアントから `hm-mcp` を実行できるようにしてください：

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

`remember` に `{"conversation":"demo","kind":"user","content":"The region is eu-central-1."}` を渡し、続いて `recall` に `{"mode":"lexical","query":"region","limit":5}` を渡します。[14 ツールの一覧](docs/reference/generated/tools.md)は remember、recall、activate、believe、retract、dispute、intend、bind、predict、outcome、attest、consolidate、inspect、forget を網羅します。

actor ディレクトリを所有できるプロセスは 1 つだけです。MCP、デーモン、組み込みエンジンを同時に書き込み所有者として使わないでください。生成設定には鍵と actor/管理者の capability が含まれるため、非公開にし、バージョン管理に入れないでください。

## デーモンと CLI を使う

先に MCP の所有プロセスを停止します。1 つのターミナルでデーモンを起動します：

```sh
hm serve --config .hypermind/hypermind.conf --json
```

別のターミナルで記憶を追加し、台帳 ID を取得して、コンテキストバンドルを要求します：

```sh
hm remember --config .hypermind/hypermind.conf --conversation demo \
  --content "The deployment region is eu-central-1." --json
hm recall --config .hypermind/hypermind.conf --query "deployment region" --json
hm activate --config .hypermind/hypermind.conf --conversation demo \
  --query "deployment region" --budget-tokens 1024 --json
```

現在、デーモンの `recall` は整形された回答ではなく `lsns` を返し、`activate` は base64 の HMA1 バンドルを返します。記憶の本文は MCP または SDK レンダラーで読んでください。CLI の `--embedded` はデーモン停止後に限り代替として使えます。[CLI 契約](docs/reference/cli.md)

## リモート接続には相互 TLS が必須

リモートリスナーは明示的に有効化します。サーバー証明書と鍵、信頼するクライアント CA、クライアント側の有効な capability トークンが必要です。ローカル専用デーモンの代わりに以下を実行します。証明書は事前に用意してください：

```sh
hm serve --config .hypermind/hypermind.conf \
  --grpc-bind 127.0.0.1:7443 --rest-bind 127.0.0.1:8443 \
  --tls-cert /secure/server.pem --tls-key /secure/server.key \
  --tls-client-ca /secure/client-ca.pem --json
```

actor と管理者のリスナーを分離してください。クライアント証明書だけでは actor 権限は付与されません。[リモート配置](docs/guides/deployment.md)、[プロトコル契約](docs/reference/protocol.md)、[Docker・Compose・systemd・Helm](deploy/README.md)を参照してください。公開済みイメージの存在を前提にはしません。

## SDK の入口

SDK はこのリポジトリ内にあります。パッケージ公開と複数言語のリリース適格性検証は別作業です。ソースがあることは npm、PyPI、ビルド済みバイナリの提供を保証しません。

| 言語 | ソース | 入口 |
| --- | --- | --- |
| Rust | [hm-serve](crates/hm-serve/src/embedded.rs) | `HyperMind::open`, `session`, `remember`, `recall`, `activate`, `render` |
| TypeScript | [sdk/typescript](sdk/typescript) | `@hypermind/client`: `Client.connect`; `engine`, `render`, `migrate` |
| Python | [sdk/python](sdk/python) | `Engine.open`, `session`, `remember`, `recall`, `activate`, `render`; `Client` (mTLS) |
| Go | [sdk/go](sdk/go) | `centra/core/cortexclient`: `Dial`, `Session`, `Remember`, `Recall`, `Activate` |

同じ actor では組み込み所有者かデーモンクライアントのどちらかを選びます。Python は 3.10 以上、Go は 1.25.0 を指定しています。TypeScript のビルドスクリプトは専用ワークスペース内にあります。[SDK ガイド](docs/reference/sdks.md)と[ソース由来の API 一覧](docs/reference/generated/sdk-api.md)で実際のシグネチャを確認してください。

## Centra 経由の任意プロバイダー

ローカル語彙メモリは外部プロバイダーなしで動作します。所有プロセスの環境で必要な機能だけ有効にしてください。この例は 3 つの経路をすべて明示的に有効化します：

```sh
export CENTRA_GATEWAY_URL="https://gateway.centra.ag/v1"
export CENTRA_GATEWAY_API_KEY="<your-secret-key>"
export HM_EMBEDDING_PROVIDER=centra
export HM_RECONSTRUCTION_PROVIDER=centra
export HM_CONSOLIDATION_PROVIDER=centra
```

埋め込みには `openrouter/openai/text-embedding-3-large` を使用します。再構成と統合は **`CENTRA_GATEWAY_URL` 経由で `openrouter/openai/gpt-5.6-luna`** を使用します。これらの設定は独立したベンチマーク設定を変更しません。過去の記録済みテストデータは新しいプロバイダー呼び出しではありません。

実際の鍵をコミットしたり MCP JSON に記載したりしないでください。プロバイダーには選択された内容が送られ、料金が発生し得ます。事前にデータ転送の許可と支出上限を決めてください。モデルのダウンロードは任意であり、それだけではローカル ONNX 推論は有効になりません。[設定](docs/reference/config.md)

## 根拠、権限、制限

- 検索されたメモリは非信頼データであり、system/developer 指示や行動許可ではありません。
- 主張、証明、観測済みツール結果、派生手順は異なる根拠の役割を維持します。
- 静穏時間帯と注意ポリシーが継続作業を制御します。予測は結果発生の証明ではありません。
- 台帳の暗号化は、すべてのプロジェクション、エクスポート、ログ、SDK バッファの暗号化を意味しません。状態ディレクトリ全体を保護してください。
- 単一書き込み所有者が必要です。分散マルチライターデータベースや認証情報保管庫ではありません。
- `ok`、`health`、`gaps`、来歴を確認してください。通信成功だけでは記憶の完全性や正しさは保証されません。

サービス公開や非信頼履歴のインポート前に[脅威モデル](docs/security/threat-model.md)を確認してください。

## ベンチマーク：目標と結果は別

仕様には次の受け入れ目標があります：

| 検証項目 | 目標値 — 実測結果ではありません |
| --- | --- |
| LongMemEval | 500 問、正解率 ≥ 0.90 |
| LoCoMo | 全件対象、非敵対的 F1 ≥ 0.75 |
| Recall@10 | 1 万件で ≥ 0.95 |
| ウォーム時のアクティベーション | 10 万件で p99 < 10 ms |

LongMemEval のローカル全件実行は **459/500（91.8%）** でした。Centra 経由の Luna と語彙検索のみを使用しています。LoCoMo は実行中で、適格性検証は未完了です。実コードの slice-7 集中シナリオも成功しました。これらの結果は、ホスト型 CI の成功やリリース適格性を証明しません。[測定状況と証拠](docs/evaluation/results.md)および[評価方法](docs/evaluation/methodology.md)を確認してください。

## 開発と文書のチェック

リポジトリのルートから以下の集中シナリオと文書チェックを実行します。文書ツールには Node.js と mdBook も必要です：

```sh
cargo test -p hm-sim --test slice7_journey
node docs/tools/catalog.mjs --write
cargo run -p hm-eval -- docs-gate
mdbook build docs
```

カタログは実ソースの公開インターフェースから再生成されます。`docs-gate` は古い網羅情報と壊れたローカルリンクを拒否しますが、文章の正確性や外部 URL は認証しません。[AGENTS.md](AGENTS.md) と[タスク仕様](spec/hypermind-01/spec.kvx)に従ってください。

## リポジトリ構成

| パス | 内容 |
| --- | --- |
| `crates/` | Rust カーネル、保存、認知、インターフェース、CLI、評価 |
| `schemas/` | 正式な FlatBuffers・protobuf 契約 |
| `sdk/` | TypeScript・Python・Go のソースパッケージ |
| `docs/` | mdBook、生成リファレンス、ADR-001–010 |
| `eval/` | データセットツール、ベンチマーク定義、根拠レポート |
| `deploy/` | コンテナのソースビルドと配置マニフェスト |
| `spec/` | 要件、設計、ワークフロー、タスク状態 |

[文書目次](docs/SUMMARY.md)から参照できます（英語）。

## ライセンス

Rust ワークスペースの宣言に従い Apache-2.0 です。[LICENSE](LICENSE) を参照してください。上流モデルやデータセットのライセンスは個別に確認してください。本プロジェクトのライセンスで置き換わるものではありません。
