![HyperMind](spec/readme_img.png)

# HyperMind

Memoria duradera y respaldada por evidencia para agentes de IA: conserva información entre sesiones, se recupera tras reinicios y distingue los recuerdos de la autoridad.

[English](README.md) · [简体中文](README.zh-CN.md) · [हिन्दी](README.hi.md) · [Español](README.es.md) · [Français](README.fr.md) · [العربية](README.ar.md) · [Português](README.pt.md) · [Русский](README.ru.md) · [日本語](README.ja.md) · [Deutsch](README.de.md)

Desarrollo activo: **v1.0.0 no está cualificada ni publicada**. El código, las pruebas locales y la cualificación de una versión son cosas distintas. Consulta las [evidencias y limitaciones actuales](docs/evaluation/results.md) (documentación en inglés).

## ¿Por qué HyperMind?

La ventana de contexto de un agente es temporal. La memoria útil debe sobrevivir al proceso y conservar el origen de cada dato. HyperMind es un motor Rust con almacenamiento local, procedencia explícita e interfaces para aplicaciones integradas y agentes persistentes.

- Eventos duraderos: registro cifrado de solo anexado, proyecciones reproducibles, puntos de control y recuperación.
- Recuperación con evidencia: búsqueda léxica, embeddings opcionales, creencias temporales, disputas y referencias de origen.
- Activación acotada: contexto relevante dentro de un presupuesto de tokens, etiquetado como memoria no confiable.
- Seguimiento observado: intenciones, predicciones, resultados, lotes de atención en horas de silencio y procedimientos respaldados.
- Varias interfaces: MCP stdio, demonio mediante socket Unix, gRPC/REST autenticados y SDK disponibles en el repositorio.

## Cómo se conectan las piezas

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

El registro es la fuente de verdad; proyecciones e índices son derivados. Recordar una afirmación no la verifica. La activación entrega contexto con evidencia, no instrucciones ejecutables. [Arquitectura](docs/concepts/architecture.md) · [Modelo de autoridad](docs/concepts/authority.md)

## Compilar e instalar desde el código

Necesitas un entorno Unix con Git, rustup y herramientas nativas C/C++: compilador, enlazador, make, CMake, Perl y pkg-config. El repositorio fija Rust 1.93.0. La compilación resuelve dependencias Rust y el compilador protobuf; descargarlos requiere red.

```sh
git clone https://github.com/Sidiora-Labs/HyperMind-engine.git
cd HyperMind-engine
rustup toolchain install 1.93.0 --profile minimal
cargo build --locked -p hm-cli -p hm-mcp
cargo install --path crates/hm-cli --locked
cargo install --path crates/hm-mcp --locked
```

Estos comandos instalan este código local, no paquetes publicados. Añade el directorio de binarios de Cargo al PATH del cliente. La recuperación léxica predeterminada no necesita modelos descargados ni claves de proveedor. [Guía de instalación](docs/start/quickstart.md)

## Conectar un cliente MCP

Después de instalar, inicializa el estado privado y registra el servidor stdio. Este ejemplo de dos comandos usa Claude Code:

```sh
hm init --path .hypermind --json
claude mcp add hypermind -- hm-mcp --config .hypermind/hypermind.conf
```

Para otros clientes, configura una entrada MCP equivalente. Sustituye la ruta por una absoluta y asegúrate de que el cliente encuentre `hm-mcp`:

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

Prueba `remember` con `{"conversation":"demo","kind":"user","content":"The region is eu-central-1."}` y luego `recall` con `{"mode":"lexical","query":"region","limit":5}`. El [catálogo de 14 herramientas](docs/reference/generated/tools.md) incluye remember, recall, activate, believe, retract, dispute, intend, bind, predict, outcome, attest, consolidate, inspect y forget.

Usa un solo propietario por directorio de actor: MCP, demonio o motor integrado, sin escritores concurrentes. La configuración generada contiene claves y capacidades de actor/administrador; mantenla privada y fuera del control de versiones.

## Usar el demonio y la CLI

Detén primero el propietario MCP. Inicia el demonio en una terminal:

```sh
hm serve --config .hypermind/hypermind.conf --json
```

En otra terminal, guarda un recuerdo, recupera identificadores del registro y solicita un paquete de contexto:

```sh
hm remember --config .hypermind/hypermind.conf --conversation demo \
  --content "The deployment region is eu-central-1." --json
hm recall --config .hypermind/hypermind.conf --query "deployment region" --json
hm activate --config .hypermind/hypermind.conf --conversation demo \
  --query "deployment region" --budget-tokens 1024 --json
```

Actualmente `recall` del demonio devuelve `lsns`, no una respuesta renderizada; `activate` devuelve un paquete HMA1 en base64. Para leer el texto usa MCP o un renderizador SDK. `--embedded` solo es una alternativa cuando el demonio está detenido. [Contratos CLI](docs/reference/cli.md)

## El acceso remoto requiere TLS mutuo

Los listeners remotos son opcionales. Proporciona certificado y clave del servidor, una CA de clientes confiable y un token de capacidad válido en el cliente. Ejecuta esto en lugar del demonio local; debes aprovisionar los certificados indicados:

```sh
hm serve --config .hypermind/hypermind.conf \
  --grpc-bind 127.0.0.1:7443 --rest-bind 127.0.0.1:8443 \
  --tls-cert /secure/server.pem --tls-key /secure/server.key \
  --tls-client-ca /secure/client-ca.pem --json
```

Separa los listeners de actor y administrador: un certificado de cliente no concede por sí solo autoridad de actor. Consulta [despliegue remoto](docs/guides/deployment.md), [protocolo](docs/reference/protocol.md) y [Docker, Compose, systemd y Helm](deploy/README.md). No se presupone una imagen ya publicada.

## Puntos de entrada de los SDK

Los SDK están en este repositorio. Su publicación y cualificación entre lenguajes son trabajos aparte; disponer del código no promete paquetes npm/PyPI ni binarios precompilados.

| Lenguaje | Código | Puntos de entrada |
| --- | --- | --- |
| Rust | [hm-serve](crates/hm-serve/src/embedded.rs) | `HyperMind::open`, `session`, `remember`, `recall`, `activate`, `render` |
| TypeScript | [sdk/typescript](sdk/typescript) | `@hypermind/client`: `Client.connect`; `engine`, `render`, `migrate` |
| Python | [sdk/python](sdk/python) | `Engine.open`, `session`, `remember`, `recall`, `activate`, `render`; `Client` (mTLS) |
| Go | [sdk/go](sdk/go) | `centra/core/cortexclient`: `Dial`, `Session`, `Remember`, `Recall`, `Activate` |

Elige un propietario integrado o un cliente del demonio, no ambos para el mismo actor. Python requiere 3.10+; Go declara 1.25.0. Los scripts TypeScript están en su workspace. Consulta la [guía SDK](docs/reference/sdks.md) y el [catálogo derivado del código](docs/reference/generated/sdk-api.md).

## Proveedores opcionales mediante Centra

La memoria léxica local funciona sin proveedor remoto. Activa solo lo necesario en el entorno del proceso propietario; este ejemplo habilita explícitamente las tres rutas:

```sh
export CENTRA_GATEWAY_URL="https://gateway.centra.ag/v1"
export CENTRA_GATEWAY_API_KEY="<your-secret-key>"
export HM_EMBEDDING_PROVIDER=centra
export HM_RECONSTRUCTION_PROVIDER=centra
export HM_CONSOLIDATION_PROVIDER=centra
```

Los embeddings usan `openrouter/openai/text-embedding-3-large`. Reconstrucción y consolidación usan **`openrouter/openai/gpt-5.6-luna` mediante `CENTRA_GATEWAY_URL`**. Estas opciones no cambian la configuración independiente de benchmarks. Las capturas históricas de pruebas no son llamadas nuevas.

Nunca guardes claves reales en Git ni en el JSON MCP. Los proveedores reciben contenido seleccionado y pueden generar cargos: autoriza la transferencia y fija un presupuesto antes de usarlos. Descargar modelos es opcional y no habilita por sí solo inferencia ONNX local. [Configuración](docs/reference/config.md)

## Evidencia, autoridad y límites

- La memoria recuperada es información no confiable, nunca instrucciones de sistema/desarrollador ni permiso para actuar.
- Afirmaciones, atestaciones, resultados observados y procedimientos derivados conservan papeles probatorios distintos.
- Las horas de silencio y la política de atención regulan el seguimiento; una predicción no prueba su resultado.
- Cifrar el registro no implica cifrar todas las proyecciones, exportaciones, logs o buffers SDK. Protege todo el directorio.
- Importa tener un único escritor. No es una base distribuida multiescritor ni una bóveda de credenciales.
- Revisa `ok`, `health`, `gaps` y procedencia; un transporte exitoso no garantiza memoria completa o correcta.

Lee el [modelo de amenazas](docs/security/threat-model.md) antes de exponer un servicio o importar historial no confiable.

## Benchmarks: objetivos, no resultados

La especificación establece estos criterios de aceptación:

| Control | Objetivo, no resultado medido |
| --- | --- |
| LongMemEval | 500 preguntas; exactitud ≥ 0,90 |
| LoCoMo | Cobertura completa; F1 no adversarial ≥ 0,75 |
| Recall@10 | ≥ 0,95 con 10 000 elementos |
| Activación en caliente | p99 < 10 ms con 100 000 elementos |

La ejecución local completa de LongMemEval obtuvo **459/500 (91,8%)**, con Luna a través de Centra y recuperación exclusivamente léxica. LoCoMo está en ejecución; su cualificación sigue pendiente. También pasó el recorrido real y enfocado slice-7. Estos resultados no demuestran una CI alojada aprobada ni la cualificación de una versión. Consulta el [estado medido y las evidencias](docs/evaluation/results.md) y la [metodología](docs/evaluation/methodology.md).

## Desarrollar y comprobar la documentación

Desde la raíz del repositorio, usa este recorrido enfocado y las comprobaciones documentales. La documentación también necesita Node.js y mdBook:

```sh
cargo test -p hm-sim --test slice7_journey
node docs/tools/catalog.mjs --write
cargo run -p hm-eval -- docs-gate
mdbook build docs
```

El catálogo regenera las superficies públicas desde el código real. `docs-gate` rechaza cobertura obsoleta y enlaces locales rotos; no certifica la prosa ni las URL externas. Sigue [AGENTS.md](AGENTS.md) y la [especificación de tareas](spec/hypermind-01/spec.kvx).

## Mapa del repositorio

| Ruta | Contenido |
| --- | --- |
| `crates/` | Núcleo Rust, almacenamiento, cognición, interfaces, CLI y evaluación |
| `schemas/` | Contratos canónicos FlatBuffers y protobuf |
| `sdk/` | Paquetes fuente TypeScript, Python y Go |
| `docs/` | mdBook, referencias generadas y ADR-001–010 |
| `eval/` | Herramientas de datos, benchmarks e informes de evidencia |
| `deploy/` | Construcción del contenedor y manifiestos de despliegue |
| `spec/` | Requisitos, diseño, flujo de trabajo y estado de tareas |

Empieza por el [índice documental](docs/SUMMARY.md) (en inglés).

## Licencia

Apache-2.0, según el workspace Rust. Consulta [LICENSE](LICENSE). Revisa aparte las licencias de modelos y datasets; la licencia del proyecto no sustituye las suyas.
