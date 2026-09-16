# HyperMind

AI एजेंटों के लिए टिकाऊ, साक्ष्य-आधारित मेमोरी: सत्रों के बीच जानकारी रखें, रीस्टार्ट के बाद काम जारी रखें और याद की गई सामग्री को अधिकार न समझें।

[English](README.md) · [简体中文](README.zh-CN.md) · [हिन्दी](README.hi.md) · [Español](README.es.md) · [Français](README.fr.md) · [العربية](README.ar.md) · [Português](README.pt.md) · [Русский](README.ru.md) · [日本語](README.ja.md) · [Deutsch](README.de.md)

सक्रिय विकास जारी है: **v1.0.0 का योग्यता सत्यापन और रिलीज़ अभी नहीं हुआ है**। स्रोत उपलब्ध होना, स्थानीय परीक्षण सफल होना और रिलीज़ का योग्य होना अलग बातें हैं। [मौजूदा साक्ष्य और सीमाएँ](docs/evaluation/results.md) देखें (विस्तृत दस्तावेज़ अंग्रेज़ी में हैं)।

## HyperMind क्यों?

एजेंट की कॉन्टेक्स्ट विंडो अस्थायी होती है। उपयोगी मेमोरी को प्रक्रिया बंद होने के बाद भी बचना चाहिए और जानकारी का स्रोत बनाए रखना चाहिए। HyperMind एक Rust मेमोरी इंजन है, जिसमें स्थानीय भंडारण, स्पष्ट स्रोत-इतिहास और एम्बेडेड अनुप्रयोगों तथा लगातार चलने वाले एजेंटों के लिए इंटरफ़ेस हैं।

- टिकाऊ घटनाएँ: एन्क्रिप्टेड, केवल जोड़ने वाला लेजर, दोबारा बनाई जा सकने वाली प्रोजेक्शन, चेकपॉइंट और रीस्टार्ट निरंतरता।
- साक्ष्य सहित खोज: शब्द-आधारित खोज, वैकल्पिक एम्बेडिंग, समय-संबंधी विश्वास, विवाद और स्रोत संदर्भ।
- सीमित एक्टिवेशन: टोकन बजट में प्रासंगिक संदर्भ, अविश्वसनीय मेमोरी के लेबल बनाए रखते हुए।
- अवलोकन पर आधारित अगला काम: इरादे, भविष्यवाणियाँ, परिणाम, शांत समय में ध्यान-बैच और साक्ष्य-समर्थित प्रक्रियाएँ।
- अनेक इंटरफ़ेस: stdio MCP, Unix सॉकेट डेमन, प्रमाणित gRPC/REST और स्रोत रूप में SDK।

## घटक कैसे जुड़ते हैं

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

लेजर मूल सत्य-स्रोत है; प्रोजेक्शन और इंडेक्स उससे निकले दृश्य हैं। किसी दावे को याद रखना उसे सत्यापित नहीं करता। एक्टिवेशन साक्ष्य-चिह्नित संदर्भ बनाता है, चलाने योग्य निर्देश नहीं। [आर्किटेक्चर](docs/concepts/architecture.md) · [अधिकार मॉडल](docs/concepts/authority.md)

## स्रोत से बिल्ड और इंस्टॉल करें

Unix विकास मशीन, Git, rustup और मूल C/C++ टूलचेन चाहिए: कंपाइलर, लिंकर, make, CMake, Perl और pkg-config। रिपॉज़िटरी Rust 1.93.0 तय करती है। बिल्ड Rust निर्भरताएँ और protobuf कंपाइलर प्राप्त करता है; डाउनलोड के लिए नेटवर्क चाहिए।

```sh
git clone https://github.com/Sidiora-Labs/HyperMind-engine.git
cd HyperMind-engine
rustup toolchain install 1.93.0 --profile minimal
cargo build --locked -p hm-cli -p hm-mcp
cargo install --path crates/hm-cli --locked
cargo install --path crates/hm-mcp --locked
```

ये कमांड इसी चेकआउट को इंस्टॉल करते हैं, प्रकाशित पैकेजों का दावा नहीं करते। Cargo के बाइनरी फ़ोल्डर को क्लाइंट के PATH में रखें। डिफ़ॉल्ट शब्द-आधारित खोज के लिए मॉडल डाउनलोड या प्रदाता कुंजी नहीं चाहिए। [इंस्टॉलेशन गाइड](docs/start/quickstart.md)

## MCP क्लाइंट जोड़ें

इंस्टॉल करने के बाद निजी स्थिति बनाएँ और stdio सर्वर रजिस्टर करें। दो कमांड का यह उदाहरण Claude Code के लिए है:

```sh
hm init --path .hypermind --json
claude mcp add hypermind -- hm-mcp --config .hypermind/hypermind.conf
```

अन्य क्लाइंट में समान MCP सर्वर प्रविष्टि दें। कॉन्फ़िगरेशन का पूर्ण पथ लिखें और सुनिश्चित करें कि क्लाइंट `hm-mcp` खोज सके:

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

`remember` को `{"conversation":"demo","kind":"user","content":"The region is eu-central-1."}` दें, फिर `recall` को `{"mode":"lexical","query":"region","limit":5}` दें। [14 टूल की सूची](docs/reference/generated/tools.md) में remember, recall, activate, believe, retract, dispute, intend, bind, predict, outcome, attest, consolidate, inspect और forget शामिल हैं।

हर actor फ़ोल्डर का केवल एक स्वामी हो: MCP, डेमन या एम्बेडेड इंजन; समानांतर लेखक नहीं। बनी हुई कॉन्फ़िगरेशन में कुंजियाँ और actor/admin क्षमता टोकन होते हैं; इसे निजी रखें और संस्करण नियंत्रण में न डालें।

## डेमन और CLI इस्तेमाल करें

पहले MCP स्वामी को रोकें। एक टर्मिनल में डेमन शुरू करें:

```sh
hm serve --config .hypermind/hypermind.conf --json
```

दूसरे टर्मिनल में मेमोरी जोड़ें, लेजर पहचानकर्ता खोजें और कॉन्टेक्स्ट बंडल माँगें:

```sh
hm remember --config .hypermind/hypermind.conf --conversation demo \
  --content "The deployment region is eu-central-1." --json
hm recall --config .hypermind/hypermind.conf --query "deployment region" --json
hm activate --config .hypermind/hypermind.conf --conversation demo \
  --query "deployment region" --budget-tokens 1024 --json
```

अभी डेमन का `recall` रेंडर किया गया उत्तर नहीं, `lsns` लौटाता है; `activate` base64 में HMA1 बंडल देता है। मेमोरी का पाठ पढ़ने के लिए MCP या SDK रेंडरर इस्तेमाल करें। CLI का `--embedded` डेमन रुकने के बाद ही विकल्प है। [CLI अनुबंध](docs/reference/cli.md)

## दूरस्थ पहुँच के लिए म्यूचुअल TLS अनिवार्य है

दूरस्थ लिसनर स्पष्ट रूप से चालू करने होते हैं। सर्वर प्रमाणपत्र और कुंजी, विश्वसनीय क्लाइंट CA तथा क्लाइंट का वैध क्षमता टोकन चाहिए। इसे स्थानीय डेमन की जगह चलाएँ; नीचे दिए प्रमाणपत्र पहले उपलब्ध कराएँ:

```sh
hm serve --config .hypermind/hypermind.conf \
  --grpc-bind 127.0.0.1:7443 --rest-bind 127.0.0.1:8443 \
  --tls-cert /secure/server.pem --tls-key /secure/server.key \
  --tls-client-ca /secure/client-ca.pem --json
```

actor और admin लिसनर अलग रखें; केवल क्लाइंट प्रमाणपत्र actor अधिकार नहीं देता। [दूरस्थ डिप्लॉयमेंट](docs/guides/deployment.md), [प्रोटोकॉल अनुबंध](docs/reference/protocol.md) और [Docker, Compose, systemd तथा Helm](deploy/README.md) देखें। पहले से प्रकाशित इमेज का दावा नहीं है।

## SDK प्रवेश बिंदु

SDK इसी रिपॉज़िटरी में हैं। पैकेज प्रकाशित करना और भाषाओं के बीच रिलीज़ सत्यापन अलग काम हैं; स्रोत उपलब्ध होने का मतलब npm, PyPI या पहले से बने बाइनरी उपलब्ध होना नहीं है।

| भाषा | स्रोत | प्रवेश बिंदु |
| --- | --- | --- |
| Rust | [hm-serve](crates/hm-serve/src/embedded.rs) | `HyperMind::open`, `session`, `remember`, `recall`, `activate`, `render` |
| TypeScript | [sdk/typescript](sdk/typescript) | `@hypermind/client`: `Client.connect`; `engine`, `render`, `migrate` |
| Python | [sdk/python](sdk/python) | `Engine.open`, `session`, `remember`, `recall`, `activate`, `render`; `Client` (mTLS) |
| Go | [sdk/go](sdk/go) | `centra/core/cortexclient`: `Dial`, `Session`, `Remember`, `Recall`, `Activate` |

एक actor के लिए एम्बेडेड स्वामी या डेमन क्लाइंट चुनें, दोनों नहीं। Python 3.10+ चाहिए; Go में 1.25.0 घोषित है। TypeScript बिल्ड स्क्रिप्ट उसके workspace में हैं। मौजूदा हस्ताक्षरों के लिए [SDK मार्गदर्शन](docs/reference/sdks.md) और [स्रोत से बनी API सूची](docs/reference/generated/sdk-api.md) देखें।

## Centra से वैकल्पिक प्रदाता

स्थानीय शब्द-आधारित मेमोरी दूरस्थ प्रदाता के बिना काम करती है। स्वामी प्रक्रिया के वातावरण में केवल ज़रूरी सुविधाएँ चालू करें; यह उदाहरण तीनों प्रदाता-समर्थित रास्ते स्पष्ट रूप से चालू करता है:

```sh
export CENTRA_GATEWAY_URL="https://gateway.centra.ag/v1"
export CENTRA_GATEWAY_API_KEY="<your-secret-key>"
export HM_EMBEDDING_PROVIDER=centra
export HM_RECONSTRUCTION_PROVIDER=centra
export HM_CONSOLIDATION_PROVIDER=centra
```

एम्बेडिंग `openrouter/openai/text-embedding-3-large` इस्तेमाल करती है। पुनर्निर्माण और समेकन **`CENTRA_GATEWAY_URL` से `openrouter/openai/gpt-5.6-luna`** इस्तेमाल करते हैं। ये विकल्प अलग बेंचमार्क कॉन्फ़िगरेशन नहीं बदलते। पुराने रिकॉर्ड किए परीक्षण नए प्रदाता कॉल नहीं हैं।

असली कुंजियाँ Git या MCP JSON में कभी न रखें। प्रदाता को चुनी हुई सामग्री भेजी जाती है और शुल्क लग सकता है; पहले डेटा हस्तांतरण की अनुमति लें और खर्च की सीमा तय करें। मॉडल डाउनलोड वैकल्पिक हैं और अपने-आप स्थानीय ONNX इन्फ़रेंस चालू नहीं करते। [कॉन्फ़िगरेशन](docs/reference/config.md)

## साक्ष्य, अधिकार और सीमाएँ

- खोजी गई मेमोरी अविश्वसनीय डेटा है, सिस्टम/डेवलपर निर्देश या कार्रवाई की अनुमति नहीं।
- दावे, प्रमाणन, देखे गए टूल परिणाम और उनसे बनी प्रक्रियाएँ अलग साक्ष्य भूमिकाएँ बनाए रखते हैं।
- शांत समय और ध्यान नीति आगे के काम को नियंत्रित करते हैं; भविष्यवाणी परिणाम घटने का प्रमाण नहीं है।
- लेजर एन्क्रिप्शन का मतलब सभी प्रोजेक्शन, निर्यात, लॉग या SDK बफ़र एन्क्रिप्टेड होना नहीं। पूरे स्थिति फ़ोल्डर की रक्षा करें।
- एकल लेखक का नियम ज़रूरी है। यह वितरित बहु-लेखक डेटाबेस या क्रेडेंशियल वॉल्ट नहीं है।
- `ok`, `health`, `gaps` और स्रोत-इतिहास जाँचें; ट्रांसपोर्ट सफल होना पूरी या सही मेमोरी की गारंटी नहीं।

सेवा सार्वजनिक करने या अविश्वसनीय इतिहास आयात करने से पहले [खतरा मॉडल](docs/security/threat-model.md) पढ़ें।

## बेंचमार्क: लक्ष्य परिणाम नहीं हैं

विनिर्देश में ये स्वीकृति लक्ष्य हैं:

| जाँच | लक्ष्य — मापा हुआ दावा नहीं |
| --- | --- |
| LongMemEval | 500 प्रश्न; सटीकता ≥ 0.90 |
| LoCoMo | पूरा कवरेज; गैर-प्रतिद्वंद्वी F1 ≥ 0.75 |
| Recall@10 | 10 हज़ार आइटम पर ≥ 0.95 |
| वार्म एक्टिवेशन | 1 लाख आइटम पर p99 < 10 ms |

पूर्ण LongMemEval रन जारी है; LoCoMo योग्यता सत्यापन बाकी है। वास्तविक slice-7 केंद्रित यात्रा सफल हुई, लेकिन वह या चुने हुए निदान रिलीज़ को योग्य नहीं बनाते। [मापा हुआ दर्जा](docs/evaluation/results.md) और [मूल्यांकन पद्धति](docs/evaluation/methodology.md) देखें, आंशिक स्कोर को अंतिम न मानें।

## विकास और दस्तावेज़ जाँच

रिपॉज़िटरी की जड़ से नीचे दी गई केंद्रित यात्रा और दस्तावेज़ जाँच चलाएँ। दस्तावेज़ टूल के लिए Node.js और mdBook भी चाहिए:

```sh
cargo test -p hm-sim --test slice7_journey
node docs/tools/catalog.mjs --write
cargo run -p hm-eval -- docs-gate
mdbook build docs
```

कैटलॉग कमांड असली स्रोत से सार्वजनिक इंटरफ़ेस दस्तावेज़ फिर बनाता है। `docs-gate` पुराने कवरेज और टूटे स्थानीय लिंक अस्वीकार करता है; पाठ की शुद्धता या बाहरी URL प्रमाणित नहीं करता। [AGENTS.md](AGENTS.md) और [कार्य विनिर्देश](spec/hypermind-01/spec.kvx) का पालन करें।

## रिपॉज़िटरी का नक्शा

| पथ | सामग्री |
| --- | --- |
| `crates/` | Rust कर्नेल, भंडारण, संज्ञान, इंटरफ़ेस, CLI और मूल्यांकन |
| `schemas/` | आधिकारिक FlatBuffers और protobuf अनुबंध |
| `sdk/` | TypeScript, Python और Go स्रोत पैकेज |
| `docs/` | mdBook, बनी हुई संदर्भ सूची और ADR-001–010 |
| `eval/` | डेटासेट टूल, बेंचमार्क परिभाषाएँ और साक्ष्य रिपोर्ट |
| `deploy/` | स्रोत से कंटेनर बिल्ड और डिप्लॉयमेंट मैनिफ़ेस्ट |
| `spec/` | आवश्यकताएँ, डिज़ाइन, कार्यप्रवाह और कार्य स्थिति |

[दस्तावेज़ सूची](docs/SUMMARY.md) से शुरू करें (अंग्रेज़ी)।

## लाइसेंस

Rust workspace के अनुसार Apache-2.0। [LICENSE](LICENSE) देखें। मॉडल और डेटासेट के मूल लाइसेंस अलग से जाँचें; परियोजना का लाइसेंस उन्हें नहीं बदलता।
