# HyperMind

Memória durável e baseada em evidências para agentes de IA: preserve informações entre sessões, retome após reinicializações e não confunda lembranças com autoridade.

[English](README.md) · [简体中文](README.zh-CN.md) · [हिन्दी](README.hi.md) · [Español](README.es.md) · [Français](README.fr.md) · [العربية](README.ar.md) · [Português](README.pt.md) · [Русский](README.ru.md) · [日本語](README.ja.md) · [Deutsch](README.de.md)

Desenvolvimento ativo: **a versão 1.0.0 não está qualificada nem lançada**. Código disponível, testes locais e qualificação de uma versão são coisas diferentes. Consulte as [evidências e limitações atuais](docs/evaluation/results.md) (documentação em inglês).

## Por que HyperMind?

A janela de contexto de um agente é temporária. Uma memória útil precisa sobreviver ao processo e preservar a origem das informações. HyperMind é um motor Rust com armazenamento local, proveniência explícita e interfaces para aplicações integradas e agentes persistentes.

- Eventos duráveis: registro criptografado somente de acréscimo, projeções reproduzíveis, checkpoints e retomada.
- Recuperação com evidências: busca lexical, embeddings opcionais, crenças temporais, contestações e referências.
- Ativação limitada: contexto relevante dentro do orçamento de tokens, mantendo rótulos de memória não confiável.
- Acompanhamento observado: intenções, previsões, resultados, lotes de atenção em horários de silêncio e procedimentos sustentados por evidências.
- Várias interfaces: MCP stdio, daemon por socket Unix, gRPC/REST autenticados e SDKs disponíveis no repositório.

## Como funciona

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

O registro é a fonte de verdade; projeções e índices são derivados. Armazenar uma afirmação não a verifica. A ativação produz contexto identificado por evidências, não instruções executáveis. [Arquitetura](docs/concepts/architecture.md) · [Modelo de autoridade](docs/concepts/authority.md)

## Compilar e instalar a partir do código

Use um ambiente Unix com Git, rustup e ferramentas nativas C/C++: compilador, linker, make, CMake, Perl e pkg-config. O repositório fixa Rust 1.93.0. O build resolve dependências Rust e o compilador protobuf; os downloads precisam de rede.

```sh
git clone https://github.com/Sidiora-Labs/HyperMind-engine.git
cd HyperMind-engine
rustup toolchain install 1.93.0 --profile minimal
cargo build --locked -p hm-cli -p hm-mcp
cargo install --path crates/hm-cli --locked
cargo install --path crates/hm-mcp --locked
```

Estes comandos instalam este checkout, não pacotes publicados. Inclua o diretório de executáveis Cargo no PATH do cliente. O modo lexical padrão não precisa baixar modelos nem fornecer chaves. [Guia de instalação](docs/start/quickstart.md)

## Conectar um cliente MCP

Após instalar, inicialize o estado privado e registre o servidor stdio. Este exemplo de dois comandos usa Claude Code:

```sh
hm init --path .hypermind --json
claude mcp add hypermind -- hm-mcp --config .hypermind/hypermind.conf
```

Para outros clientes, use uma entrada MCP equivalente. Substitua o caminho por um absoluto e garanta que o cliente encontre `hm-mcp`:

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

Experimente `remember` com `{"conversation":"demo","kind":"user","content":"The region is eu-central-1."}` e depois `recall` com `{"mode":"lexical","query":"region","limit":5}`. O [catálogo de 14 ferramentas](docs/reference/generated/tools.md) cobre remember, recall, activate, believe, retract, dispute, intend, bind, predict, outcome, attest, consolidate, inspect e forget.

Mantenha um único proprietário por diretório de ator: MCP, daemon ou motor integrado, sem escritores simultâneos. A configuração gerada contém chaves e capacidades de ator/administrador; mantenha-a privada e fora do Git.

## Usar o daemon e a CLI

Pare primeiro o proprietário MCP. Inicie o daemon em um terminal:

```sh
hm serve --config .hypermind/hypermind.conf --json
```

Em outro terminal, registre uma lembrança, recupere identificadores do registro e solicite um pacote de contexto:

```sh
hm remember --config .hypermind/hypermind.conf --conversation demo \
  --content "The deployment region is eu-central-1." --json
hm recall --config .hypermind/hypermind.conf --query "deployment region" --json
hm activate --config .hypermind/hypermind.conf --conversation demo \
  --query "deployment region" --budget-tokens 1024 --json
```

Hoje, `recall` do daemon retorna `lsns`, não uma resposta renderizada; `activate` retorna um pacote HMA1 em base64. Use MCP ou um renderizador SDK para ler o texto. `--embedded` é uma alternativa somente depois de parar o daemon. [Contratos da CLI](docs/reference/cli.md)

## Acesso remoto exige TLS mútuo

Listeners remotos são opcionais. Forneça certificado/chave do servidor, uma CA confiável de clientes e um token de capacidade válido no cliente. Execute isto no lugar do daemon local; os certificados precisam estar provisionados:

```sh
hm serve --config .hypermind/hypermind.conf \
  --grpc-bind 127.0.0.1:7443 --rest-bind 127.0.0.1:8443 \
  --tls-cert /secure/server.pem --tls-key /secure/server.key \
  --tls-client-ca /secure/client-ca.pem --json
```

Separe listeners de ator e administrador: um certificado de cliente não concede sozinho autoridade de ator. Veja [implantação remota](docs/guides/deployment.md), [protocolo](docs/reference/protocol.md) e [Docker, Compose, systemd e Helm](deploy/README.md). Nenhuma imagem já publicada é presumida.

## Pontos de entrada dos SDKs

Os SDKs estão neste repositório. Publicação e qualificação entre linguagens são trabalhos separados; código disponível não promete pacotes npm/PyPI nem binários pré-compilados.

| Linguagem | Código | Pontos de entrada |
| --- | --- | --- |
| Rust | [hm-serve](crates/hm-serve/src/embedded.rs) | `HyperMind::open`, `session`, `remember`, `recall`, `activate`, `render` |
| TypeScript | [sdk/typescript](sdk/typescript) | `@hypermind/client`: `Client.connect`; `engine`, `render`, `migrate` |
| Python | [sdk/python](sdk/python) | `Engine.open`, `session`, `remember`, `recall`, `activate`, `render`; `Client` (mTLS) |
| Go | [sdk/go](sdk/go) | `centra/core/cortexclient`: `Dial`, `Session`, `Remember`, `Recall`, `Activate` |

Escolha um proprietário integrado ou um cliente do daemon, não ambos para o mesmo ator. Python exige 3.10+; Go declara 1.25.0. Scripts TypeScript estão em seu workspace. Consulte o [guia de SDKs](docs/reference/sdks.md) e o [catálogo derivado do código](docs/reference/generated/sdk-api.md).

## Provedores opcionais via Centra

A memória lexical local funciona sem provedor remoto. Ative apenas o necessário no ambiente do processo proprietário; este exemplo habilita explicitamente os três caminhos:

```sh
export CENTRA_GATEWAY_URL="https://gateway.centra.ag/v1"
export CENTRA_GATEWAY_API_KEY="<your-secret-key>"
export HM_EMBEDDING_PROVIDER=centra
export HM_RECONSTRUCTION_PROVIDER=centra
export HM_CONSOLIDATION_PROVIDER=centra
```

Embeddings usam `openrouter/openai/text-embedding-3-large`. Reconstrução e consolidação usam **`openrouter/openai/gpt-5.6-luna` via `CENTRA_GATEWAY_URL`**. Essas opções não alteram a configuração independente dos benchmarks. Capturas históricas de testes não são novas chamadas.

Nunca versione chaves reais nem as coloque no JSON MCP. Provedores recebem conteúdo selecionado e podem gerar cobranças: autorize a transferência e defina um orçamento antes de usá-los. Baixar modelos é opcional e não ativa, por si só, inferência ONNX local. [Configuração](docs/reference/config.md)

## Evidências, autoridade e limites

- Memória recuperada é dado não confiável, nunca instrução de sistema/desenvolvedor ou permissão para agir.
- Afirmações, atestações, resultados observados e procedimentos derivados mantêm papéis de evidência distintos.
- Horários de silêncio e políticas de atenção regulam o acompanhamento; uma previsão não prova seu resultado.
- Criptografar o registro não significa criptografar todas as projeções, exportações, logs ou buffers SDK. Proteja todo o diretório.
- O proprietário único importa. Isto não é um banco distribuído multi-escritor nem um cofre de credenciais.
- Examine `ok`, `health`, `gaps` e proveniência; sucesso no transporte não garante memória completa ou correta.

Leia o [modelo de ameaças](docs/security/threat-model.md) antes de expor um serviço ou importar histórico não confiável.

## Benchmarks: metas não são resultados

A especificação estabelece estes critérios de aceitação:

| Verificação | Meta — não uma medição |
| --- | --- |
| LongMemEval | 500 perguntas; acurácia ≥ 0,90 |
| LoCoMo | Cobertura completa; F1 não adversarial ≥ 0,75 |
| Recall@10 | ≥ 0,95 com 10 mil itens |
| Ativação aquecida | p99 < 10 ms com 100 mil itens |

A execução local completa do LongMemEval obteve **459/500 (91,8%)**, com Luna via Centra e recuperação exclusivamente lexical. O LoCoMo está em andamento; sua qualificação permanece pendente. A jornada real e focada slice-7 também passou. Esses resultados não comprovam aprovação da CI hospedada nem qualificação de uma versão. Consulte o [estado medido e as evidências](docs/evaluation/results.md) e a [metodologia](docs/evaluation/methodology.md).

## Desenvolver e verificar a documentação

Na raiz do repositório, use a jornada focada e as verificações abaixo. A documentação também requer Node.js e mdBook:

```sh
cargo test -p hm-sim --test slice7_journey
node docs/tools/catalog.mjs --write
cargo run -p hm-eval -- docs-gate
mdbook build docs
```

O catálogo regenera as superfícies públicas a partir do código real. `docs-gate` rejeita cobertura desatualizada e links locais quebrados; não certifica a redação nem URLs externas. Siga [AGENTS.md](AGENTS.md) e a [especificação de tarefas](spec/hypermind-01/spec.kvx).

## Mapa do repositório

| Caminho | Conteúdo |
| --- | --- |
| `crates/` | Núcleo Rust, armazenamento, cognição, interfaces, CLI e avaliação |
| `schemas/` | Contratos canônicos FlatBuffers e protobuf |
| `sdk/` | Pacotes-fonte TypeScript, Python e Go |
| `docs/` | mdBook, referências geradas e ADR-001–010 |
| `eval/` | Ferramentas de datasets, benchmarks e relatórios de evidências |
| `deploy/` | Build do contêiner e manifestos de implantação |
| `spec/` | Requisitos, design, fluxo de trabalho e estado das tarefas |

Comece pelo [índice da documentação](docs/SUMMARY.md) (em inglês).

## Licença

Apache-2.0, conforme declarado no workspace Rust. Veja [LICENSE](LICENSE). Verifique separadamente as licenças de modelos e datasets; a licença do projeto não as substitui.
