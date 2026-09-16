# HyperMind

Une mémoire durable, fondée sur les preuves, pour les agents IA : conserver le contexte entre sessions, reprendre après un redémarrage et ne jamais confondre un souvenir avec une autorisation.

[English](README.md) · [简体中文](README.zh-CN.md) · [हिन्दी](README.hi.md) · [Español](README.es.md) · [Français](README.fr.md) · [العربية](README.ar.md) · [Português](README.pt.md) · [Русский](README.ru.md) · [日本語](README.ja.md) · [Deutsch](README.de.md)

Développement actif : **la version 1.0.0 n’est ni qualifiée ni publiée**. Le code source, les tests locaux et la qualification d’une version sont des éléments distincts. Voir les [preuves et limites actuelles](docs/evaluation/results.md) (documentation en anglais).

## Pourquoi HyperMind ?

La fenêtre de contexte d’un agent est temporaire. Une mémoire utile doit survivre au processus et conserver l’origine des informations. HyperMind est un moteur Rust avec stockage local, provenance explicite et interfaces intégrées ou clientes.

- Événements durables : journal chiffré à ajout seul, projections rejouables, points de contrôle et reprise.
- Rappel traçable : recherche lexicale, embeddings facultatifs, croyances temporelles, contestations et références.
- Activation bornée : contexte pertinent dans un budget de tokens, toujours étiqueté comme mémoire non fiable.
- Suivi observé : intentions, prédictions, résultats, regroupement pendant les heures calmes et procédures étayées.
- Interfaces multiples : MCP stdio, démon sur socket Unix, gRPC/REST authentifiés et SDK disponibles dans le dépôt.

## Architecture

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

Le journal constitue la source de vérité ; projections et index sont dérivés. Enregistrer une affirmation ne la vérifie pas. L’activation fournit un contexte avec preuves, pas des instructions exécutables. [Architecture](docs/concepts/architecture.md) · [Modèle d’autorité](docs/concepts/authority.md)

## Compiler et installer les sources

Prévoir un hôte Unix avec Git, rustup et les outils natifs C/C++ : compilateur, éditeur de liens, make, CMake, Perl et pkg-config. Le dépôt fixe Rust 1.93.0. La compilation résout les dépendances Rust et le compilateur protobuf ; leur téléchargement nécessite le réseau.

```sh
git clone https://github.com/Sidiora-Labs/HyperMind-engine.git
cd HyperMind-engine
rustup toolchain install 1.93.0 --profile minimal
cargo build --locked -p hm-cli -p hm-mcp
cargo install --path crates/hm-cli --locked
cargo install --path crates/hm-mcp --locked
```

Ces commandes installent le code local, sans supposer de paquet publié. Ajouter le répertoire des exécutables Cargo au PATH du client. Le mode lexical par défaut ne nécessite ni modèle téléchargé ni clé fournisseur. [Guide d’installation](docs/start/quickstart.md)

## Connecter un client MCP

Après installation, initialiser l’état privé et enregistrer le serveur stdio. Exemple en deux commandes pour Claude Code :

```sh
hm init --path .hypermind --json
claude mcp add hypermind -- hm-mcp --config .hypermind/hypermind.conf
```

Pour un autre client, utiliser une configuration MCP équivalente. Remplacer le chemin par un chemin absolu et rendre `hm-mcp` accessible :

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

Essayer `remember` avec `{"conversation":"demo","kind":"user","content":"The region is eu-central-1."}`, puis `recall` avec `{"mode":"lexical","query":"region","limit":5}`. Le [catalogue des 14 outils](docs/reference/generated/tools.md) couvre remember, recall, activate, believe, retract, dispute, intend, bind, predict, outcome, attest, consolidate, inspect et forget.

Un seul processus propriétaire par répertoire d’acteur : MCP, démon ou moteur intégré, sans écritures concurrentes. La configuration générée contient des clés et des capacités acteur/administrateur ; la garder privée et hors du dépôt.

## Utiliser le démon et la CLI

Arrêter d’abord le propriétaire MCP. Démarrer le démon dans un terminal :

```sh
hm serve --config .hypermind/hypermind.conf --json
```

Dans un autre terminal, enregistrer un souvenir, récupérer ses identifiants et demander un paquet de contexte :

```sh
hm remember --config .hypermind/hypermind.conf --conversation demo \
  --content "The deployment region is eu-central-1." --json
hm recall --config .hypermind/hypermind.conf --query "deployment region" --json
hm activate --config .hypermind/hypermind.conf --conversation demo \
  --query "deployment region" --budget-tokens 1024 --json
```

Le `recall` du démon retourne actuellement des `lsns`, pas une réponse mise en forme ; `activate` retourne un paquet HMA1 encodé en base64. Utiliser MCP ou un moteur de rendu SDK pour lire le texte. `--embedded` est une alternative uniquement après l’arrêt du démon. [Contrats CLI](docs/reference/cli.md)

## Accès distant avec TLS mutuel

Les écouteurs distants sont facultatifs. Fournir certificat et clé serveur, autorité cliente de confiance et jeton de capacité valide côté client. Cette commande remplace le démon local ; les certificats doivent déjà être provisionnés :

```sh
hm serve --config .hypermind/hypermind.conf \
  --grpc-bind 127.0.0.1:7443 --rest-bind 127.0.0.1:8443 \
  --tls-cert /secure/server.pem --tls-key /secure/server.key \
  --tls-client-ca /secure/client-ca.pem --json
```

Séparer les écouteurs acteur et administrateur : un certificat client ne suffit pas à accorder l’autorité d’un acteur. Voir [déploiement distant](docs/guides/deployment.md), [protocole](docs/reference/protocol.md) et [Docker, Compose, systemd et Helm](deploy/README.md). Aucune image déjà publiée n’est présumée.

## Points d’entrée des SDK

Les SDK sont présents dans le dépôt. Leur publication et leur qualification interlangages sont distinctes ; cela ne promet ni paquet npm/PyPI ni binaire précompilé.

| Langage | Sources | Points d’entrée |
| --- | --- | --- |
| Rust | [hm-serve](crates/hm-serve/src/embedded.rs) | `HyperMind::open`, `session`, `remember`, `recall`, `activate`, `render` |
| TypeScript | [sdk/typescript](sdk/typescript) | `@hypermind/client`: `Client.connect`; `engine`, `render`, `migrate` |
| Python | [sdk/python](sdk/python) | `Engine.open`, `session`, `remember`, `recall`, `activate`, `render`; `Client` (mTLS) |
| Go | [sdk/go](sdk/go) | `centra/core/cortexclient`: `Dial`, `Session`, `Remember`, `Recall`, `Activate` |

Choisir un propriétaire intégré ou un client du démon pour un acteur, pas les deux. Python nécessite 3.10+ ; Go déclare 1.25.0. Les scripts TypeScript sont dans son espace de travail. Voir le [guide SDK](docs/reference/sdks.md) et le [catalogue issu des sources](docs/reference/generated/sdk-api.md).

## Fournisseurs facultatifs via Centra

La mémoire lexicale locale fonctionne sans fournisseur distant. Activer seulement les fonctions nécessaires dans l’environnement du processus propriétaire. Cet exemple active explicitement les trois chemins :

```sh
export CENTRA_GATEWAY_URL="https://gateway.centra.ag/v1"
export CENTRA_GATEWAY_API_KEY="<your-secret-key>"
export HM_EMBEDDING_PROVIDER=centra
export HM_RECONSTRUCTION_PROVIDER=centra
export HM_CONSOLIDATION_PROVIDER=centra
```

Les embeddings utilisent `openrouter/openai/text-embedding-3-large`. Reconstruction et consolidation utilisent **`openrouter/openai/gpt-5.6-luna` via `CENTRA_GATEWAY_URL`**. Ces options ne modifient pas la configuration indépendante des benchmarks. Les anciennes captures de tests ne sont pas de nouveaux appels.

Ne jamais committer de vraies clés ni les mettre dans le JSON MCP. Les fournisseurs reçoivent du contenu sélectionné et peuvent facturer des appels : autoriser les transferts et fixer un budget avant usage. Télécharger des modèles reste facultatif et n’active pas, à lui seul, l’inférence ONNX locale. [Configuration](docs/reference/config.md)

## Preuves, autorité et limites

- La mémoire rappelée reste une donnée non fiable, jamais une instruction système/développeur ni une permission d’agir.
- Affirmations, attestations, résultats observés et procédures dérivées gardent des rôles probatoires distincts.
- Les heures calmes et la politique d’attention encadrent le suivi ; une prédiction ne prouve pas sa réalisation.
- Le chiffrement du journal ne signifie pas que projections, exports, logs et buffers SDK sont tous chiffrés. Protéger tout le répertoire.
- Un seul propriétaire écrit. Ce n’est ni une base distribuée multi-écrivains ni un coffre à secrets.
- Examiner `ok`, `health`, `gaps` et la provenance ; une réussite réseau ne garantit pas une mémoire complète ou correcte.

Lire le [modèle de menace](docs/security/threat-model.md) avant exposition réseau ou import d’historiques non fiables.

## Benchmarks : objectifs, pas résultats

La spécification fixe les critères d’acceptation suivants :

| Contrôle | Objectif — pas une mesure |
| --- | --- |
| LongMemEval | 500 questions ; exactitude ≥ 0,90 |
| LoCoMo | Couverture complète ; F1 non adversarial ≥ 0,75 |
| Recall@10 | ≥ 0,95 sur 10 000 éléments |
| Activation à chaud | p99 < 10 ms sur 100 000 éléments |

Le passage local complet de LongMemEval a obtenu **459/500 (91,8 %)**, avec Luna via Centra et une recherche exclusivement lexicale. LoCoMo est en cours ; sa qualification reste en attente. Le parcours réel ciblé slice-7 a également réussi. Ces résultats ne prouvent ni la réussite de la CI hébergée ni la qualification d’une version. Consulter les [mesures et preuves](docs/evaluation/results.md) et la [méthodologie](docs/evaluation/methodology.md).

## Développer et vérifier la documentation

Depuis la racine du dépôt, lancer le parcours ciblé et les contrôles ci-dessous. La documentation nécessite aussi Node.js et mdBook :

```sh
cargo test -p hm-sim --test slice7_journey
node docs/tools/catalog.mjs --write
cargo run -p hm-eval -- docs-gate
mdbook build docs
```

Le catalogue est régénéré depuis les surfaces publiques réelles. `docs-gate` refuse les catalogues périmés et liens locaux cassés ; il ne certifie ni le contenu rédactionnel ni les URL externes. Suivre [AGENTS.md](AGENTS.md) et la [spécification des tâches](spec/hypermind-01/spec.kvx).

## Organisation du dépôt

| Chemin | Contenu |
| --- | --- |
| `crates/` | Noyau Rust, stockage, cognition, interfaces, CLI et évaluation |
| `schemas/` | Contrats canoniques FlatBuffers et protobuf |
| `sdk/` | Sources TypeScript, Python et Go |
| `docs/` | mdBook, références générées et ADR-001–010 |
| `eval/` | Outils de jeux de données, benchmarks et rapports de preuves |
| `deploy/` | Construction du conteneur et manifestes de déploiement |
| `spec/` | Exigences, conception, workflow et état des tâches |

Commencer par l’[index documentaire](docs/SUMMARY.md) (en anglais).

## Licence

Apache-2.0, selon l’espace de travail Rust. Voir [LICENSE](LICENSE). Vérifier séparément les licences des modèles et jeux de données ; la licence du projet ne les remplace pas.
