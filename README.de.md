![HyperMind](spec/readme_img.png)

# HyperMind

Dauerhafter, evidenzbasierter Speicher für KI-Agenten: Wissen über Sitzungen hinweg erhalten, nach Neustarts fortsetzen und Erinnerungen nicht mit Handlungsbefugnissen verwechseln.

[English](README.md) · [简体中文](README.zh-CN.md) · [हिन्दी](README.hi.md) · [Español](README.es.md) · [Français](README.fr.md) · [العربية](README.ar.md) · [Português](README.pt.md) · [Русский](README.ru.md) · [日本語](README.ja.md) · [Deutsch](README.de.md)

Aktive Entwicklung: **Version 1.0.0 ist weder qualifiziert noch veröffentlicht**. Verfügbarer Quellcode, lokale Testergebnisse und Release-Qualifikation sind unterschiedliche Aussagen. Siehe [aktueller Nachweisstand und Grenzen](docs/evaluation/results.md) (Dokumentation auf Englisch).

## Warum HyperMind?

Das Kontextfenster eines Agenten ist vorübergehend. Nützlicher Speicher muss Prozessenden überstehen und die Herkunft der Informationen bewahren. HyperMind ist eine Rust-Speicherengine mit lokaler Speicherung, expliziter Provenienz und Schnittstellen für eingebettete Anwendungen sowie dauerhaft laufende Agenten.

- Dauerhafte Ereignisse: verschlüsseltes Append-only-Journal, wiederherstellbare Projektionen, Checkpoints und Neustartkontinuität.
- Nachvollziehbarer Abruf: lexikalische Suche, optionale Embeddings, zeitbezogene Überzeugungen, Widersprüche und Quellenverweise.
- Begrenzte Aktivierung: relevanter Kontext innerhalb eines Token-Budgets mit Kennzeichnung als nicht vertrauenswürdiger Speicherinhalt.
- Beobachtete Folgearbeit: Absichten, Vorhersagen, Ergebnisse, Aufmerksamkeitsbündel während Ruhezeiten und belegte Verfahren.
- Mehrere Zugänge: stdio-MCP, Unix-Socket-Daemon, authentifiziertes gRPC/REST und SDK-Quellcode.

## Zusammenspiel der Komponenten

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

Das Journal ist die maßgebliche Datenquelle; Projektionen und Indizes sind abgeleitet. Eine gespeicherte Behauptung ist damit nicht verifiziert. Aktivierung liefert belegbaren Kontext, keine ausführbaren Anweisungen. [Architektur](docs/concepts/architecture.md) · [Autoritätsmodell](docs/concepts/authority.md)

## Aus dem Quellcode bauen und installieren

Benötigt werden ein Unix-Entwicklungsrechner, Git, rustup und native C/C++-Werkzeuge: Compiler, Linker, make, CMake, Perl und pkg-config. Das Repository legt Rust 1.93.0 fest. Der Build löst Rust-Abhängigkeiten und den protobuf-Compiler auf; Downloads benötigen Netzwerkzugriff.

```sh
git clone https://github.com/Sidiora-Labs/HyperMind-engine.git
cd HyperMind-engine
rustup toolchain install 1.93.0 --profile minimal
cargo build --locked -p hm-cli -p hm-mcp
cargo install --path crates/hm-cli --locked
cargo install --path crates/hm-mcp --locked
```

Diese Befehle installieren den ausgecheckten Quellcode, keine veröffentlichten Registry-Pakete. Das Cargo-Binärverzeichnis muss im PATH des Clients liegen. Der standardmäßige lexikalische Pfad benötigt weder Modelldownload noch Anbieter-Schlüssel. [Installationsanleitung](docs/start/quickstart.md)

## Einen MCP-Client verbinden

Nach der Installation privaten Zustand initialisieren und den stdio-Server registrieren. Dieses Beispiel mit zwei Befehlen verwendet Claude Code:

```sh
hm init --path .hypermind --json
claude mcp add hypermind -- hm-mcp --config .hypermind/hypermind.conf
```

Andere Clients benötigen einen entsprechenden MCP-Servereintrag. Den Konfigurationspfad durch einen absoluten Pfad ersetzen und sicherstellen, dass der Client `hm-mcp` findet:

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

`remember` mit `{"conversation":"demo","kind":"user","content":"The region is eu-central-1."}` aufrufen, anschließend `recall` mit `{"mode":"lexical","query":"region","limit":5}`. Der [Katalog der 14 Werkzeuge](docs/reference/generated/tools.md) umfasst remember, recall, activate, believe, retract, dispute, intend, bind, predict, outcome, attest, consolidate, inspect und forget.

Pro Actor-Verzeichnis darf nur ein Prozess Eigentümer sein: MCP, Daemon oder eingebettete Engine, keine parallelen Schreiber. Die generierte Konfiguration enthält Schlüssel sowie Actor-/Admin-Capabilities; vertraulich behandeln und nicht versionieren.

## Daemon und CLI verwenden

Zuerst den MCP-Eigentümer beenden. Dann den Daemon in einem Terminal starten:

```sh
hm serve --config .hypermind/hypermind.conf --json
```

In einem zweiten Terminal eine Erinnerung speichern, Journal-IDs abrufen und ein Kontextpaket anfordern:

```sh
hm remember --config .hypermind/hypermind.conf --conversation demo \
  --content "The deployment region is eu-central-1." --json
hm recall --config .hypermind/hypermind.conf --query "deployment region" --json
hm activate --config .hypermind/hypermind.conf --conversation demo \
  --query "deployment region" --budget-tokens 1024 --json
```

Daemon-`recall` liefert derzeit `lsns`, keine gerenderte Antwort; `activate` liefert ein base64-kodiertes HMA1-Paket. Für lesbaren Speichertext MCP oder einen SDK-Renderer nutzen. CLI-`--embedded` ist erst nach dem Beenden des Daemons eine Alternative. [CLI-Verträge](docs/reference/cli.md)

## Fernzugriff erfordert gegenseitiges TLS

Remote-Listener werden ausdrücklich aktiviert. Benötigt werden Serverzertifikat/-schlüssel, eine vertrauenswürdige Client-CA und ein gültiges Capability-Token im Client. Diesen Befehl statt des rein lokalen Daemons ausführen; die Zertifikate müssen bereitgestellt sein:

```sh
hm serve --config .hypermind/hypermind.conf \
  --grpc-bind 127.0.0.1:7443 --rest-bind 127.0.0.1:8443 \
  --tls-cert /secure/server.pem --tls-key /secure/server.key \
  --tls-client-ca /secure/client-ca.pem --json
```

Actor- und Admin-Listener getrennt halten: Ein Clientzertifikat verleiht allein keine Actor-Befugnisse. Siehe [Remote-Betrieb](docs/guides/deployment.md), [Protokollverträge](docs/reference/protocol.md) und [Docker, Compose, systemd und Helm](deploy/README.md). Ein bereits veröffentlichtes Image wird nicht vorausgesetzt.

## SDK-Einstiegspunkte

Die SDKs liegen in diesem Repository. Paketveröffentlichung und sprachübergreifende Release-Qualifikation sind getrennte Arbeiten; Quellcode-Verfügbarkeit verspricht keine npm-/PyPI-Pakete oder vorgebauten Binärdateien.

| Sprache | Quellcode | Einstiegspunkte |
| --- | --- | --- |
| Rust | [hm-serve](crates/hm-serve/src/embedded.rs) | `HyperMind::open`, `session`, `remember`, `recall`, `activate`, `render` |
| TypeScript | [sdk/typescript](sdk/typescript) | `@hypermind/client`: `Client.connect`; `engine`, `render`, `migrate` |
| Python | [sdk/python](sdk/python) | `Engine.open`, `session`, `remember`, `recall`, `activate`, `render`; `Client` (mTLS) |
| Go | [sdk/go](sdk/go) | `centra/core/cortexclient`: `Dial`, `Session`, `Remember`, `Recall`, `Activate` |

Für denselben Actor entweder eine eingebettete Engine besitzen oder einen Daemon-Client nutzen, nicht beides. Python erfordert 3.10+, Go deklariert 1.25.0. TypeScript-Buildskripte stehen im eigenen Workspace. Siehe [SDK-Hinweise](docs/reference/sdks.md) und [quellcodebasierten API-Katalog](docs/reference/generated/sdk-api.md).

## Optionale Anbieter über Centra

Lokaler lexikalischer Speicher funktioniert ohne Remote-Anbieter. Nur benötigte Funktionen in der Umgebung des Eigentümerprozesses aktivieren; dieses Beispiel schaltet ausdrücklich alle drei Pfade ein:

```sh
export CENTRA_GATEWAY_URL="https://gateway.centra.ag/v1"
export CENTRA_GATEWAY_API_KEY="<your-secret-key>"
export HM_EMBEDDING_PROVIDER=centra
export HM_RECONSTRUCTION_PROVIDER=centra
export HM_CONSOLIDATION_PROVIDER=centra
```

Embeddings verwenden `openrouter/openai/text-embedding-3-large`. Rekonstruktion und Konsolidierung verwenden **`openrouter/openai/gpt-5.6-luna` über `CENTRA_GATEWAY_URL`**. Diese Optionen ändern nicht die eigenständige Benchmark-Konfiguration. Historische Testaufzeichnungen sind keine neuen Anbieteraufrufe.

Echte Schlüssel weder committen noch in MCP-JSON schreiben. Anbieter erhalten ausgewählte Inhalte und können Kosten verursachen: Datenübertragung genehmigen lassen und vorher ein Ausgabenbudget setzen. Modelldownloads sind optional und aktivieren für sich genommen keine lokale ONNX-Inferenz. [Konfiguration](docs/reference/config.md)

## Nachweise, Befugnisse und Grenzen

- Abgerufener Speicher ist nicht vertrauenswürdiger Dateninhalt, niemals System-/Entwickleranweisung oder Handlungserlaubnis.
- Behauptungen, Attestierungen, beobachtete Werkzeugergebnisse und abgeleitete Verfahren behalten unterschiedliche Nachweisrollen.
- Ruhezeiten und Aufmerksamkeitsrichtlinien steuern Folgearbeit; eine Vorhersage beweist kein eingetretenes Ergebnis.
- Journalverschlüsselung bedeutet nicht, dass alle Projektionen, Exporte, Logs oder SDK-Puffer verschlüsselt sind. Das gesamte Zustandsverzeichnis schützen.
- Ein einzelner Schreiber ist wesentlich. Dies ist weder eine verteilte Multi-Writer-Datenbank noch ein Zugangsdaten-Tresor.
- `ok`, `health`, `gaps` und Provenienz prüfen; erfolgreicher Transport garantiert keinen vollständigen oder korrekten Speicher.

Vor Netzwerkfreigabe oder Import nicht vertrauenswürdiger Verläufe das [Bedrohungsmodell](docs/security/threat-model.md) lesen.

## Benchmarks: Ziele sind keine Ergebnisse

Die Spezifikation definiert diese Abnahmeziele:

| Prüfung | Ziel — keine gemessene Aussage |
| --- | --- |
| LongMemEval | 500 Fragen; Genauigkeit ≥ 0,90 |
| LoCoMo | Vollständige Abdeckung; nicht-adversarieller F1 ≥ 0,75 |
| Recall@10 | ≥ 0,95 bei 10.000 Einträgen |
| Warme Aktivierung | p99 < 10 ms bei 100.000 Einträgen |

Der vollständige lokale LongMemEval-Lauf erreichte **459/500 (91,8 %)** mit Luna über Centra und ausschließlich lexikalischer Suche. LoCoMo läuft; seine Qualifikation steht weiterhin aus. Auch der gezielte reale slice-7-Durchlauf bestand. Diese Ergebnisse belegen weder einen erfolgreichen gehosteten CI-Lauf noch die Release-Qualifikation. Siehe [Messstand und Nachweise](docs/evaluation/results.md) und [Methodik](docs/evaluation/methodology.md).

## Entwickeln und Dokumentation prüfen

Vom Repository-Stamm aus den gezielten Durchlauf und die Dokumentationsprüfungen verwenden. Die Dokumentation benötigt zusätzlich Node.js und mdBook:

```sh
cargo test -p hm-sim --test slice7_journey
node docs/tools/catalog.mjs --write
cargo run -p hm-eval -- docs-gate
mdbook build docs
```

Der Katalog wird aus tatsächlichen öffentlichen Quellcode-Schnittstellen erzeugt. `docs-gate` weist veraltete Abdeckung und defekte lokale Links zurück; Prosa oder externe URLs zertifiziert es nicht. [AGENTS.md](AGENTS.md) und die [Aufgabenspezifikation](spec/hypermind-01/spec.kvx) beachten.

## Repository-Übersicht

| Pfad | Inhalt |
| --- | --- |
| `crates/` | Rust-Kern, Speicherung, Kognition, Schnittstellen, CLI und Evaluation |
| `schemas/` | Kanonische FlatBuffers- und protobuf-Verträge |
| `sdk/` | TypeScript-, Python- und Go-Quellpakete |
| `docs/` | mdBook, generierte Referenzen und ADR-001–010 |
| `eval/` | Datensatzwerkzeuge, Benchmarks und Nachweisberichte |
| `deploy/` | Container-Build und Bereitstellungsmanifeste |
| `spec/` | Anforderungen, Entwurf, Arbeitsablauf und Aufgabenstatus |

Einstieg über den [Dokumentationsindex](docs/SUMMARY.md) (Englisch).

## Lizenz

Apache-2.0 gemäß Rust-Workspace. Siehe [LICENSE](LICENSE). Lizenzen von Modellen und Datensätzen separat prüfen; die Projektlizenz ersetzt diese nicht.
