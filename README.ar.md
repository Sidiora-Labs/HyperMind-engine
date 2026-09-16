![HyperMind](spec/readme_img.png)

# HyperMind

ذاكرة دائمة قائمة على الأدلة لوكلاء الذكاء الاصطناعي: احتفظ بالمعلومات بين الجلسات، واستأنف العمل بعد إعادة التشغيل، ولا تخلط الذكريات بالصلاحيات.

[English](README.md) · [简体中文](README.zh-CN.md) · [हिन्दी](README.hi.md) · [Español](README.es.md) · [Français](README.fr.md) · [العربية](README.ar.md) · [Português](README.pt.md) · [Русский](README.ru.md) · [日本語](README.ja.md) · [Deutsch](README.de.md)

المشروع قيد التطوير النشط: **الإصدار v1.0.0 لم يجتز التأهيل ولم يُنشر بعد**. توافر الشيفرة ونجاح الاختبارات المحلية وتأهيل الإصدار ادعاءات مختلفة. راجع [الأدلة والقيود الحالية](docs/evaluation/results.md) (الوثائق التفصيلية بالإنجليزية).

## لماذا HyperMind؟

نافذة سياق الوكيل مؤقتة. ينبغي للذاكرة المفيدة أن تبقى بعد انتهاء العملية وأن تحفظ مصدر المعلومات. HyperMind محرك ذاكرة بلغة Rust يوفر تخزينًا محليًا وتتبعًا صريحًا للمصادر وواجهات للتطبيقات المضمّنة والوكلاء المستمرين.

- أحداث دائمة: سجل مشفر للإضافة فقط، وإسقاطات قابلة لإعادة البناء، ونقاط تحقق، واستمرارية بعد إعادة التشغيل.
- استرجاع بأدلة: بحث لفظي، وتضمينات اختيارية، ومعتقدات مرتبطة بالزمن، واعتراضات، ومراجع للمصادر.
- تنشيط محدود: تجميع السياق المناسب ضمن ميزانية الرموز مع إبقاء وسم الذاكرة غير الموثوقة.
- متابعة مبنية على الملاحظة: نوايا وتنبؤات ونتائج ودفعات انتباه خلال ساعات الهدوء وإجراءات مدعومة بالأدلة.
- واجهات متعددة: MCP عبر stdio، وخدمة عبر مقبس Unix، وgRPC/REST موثقان، وحزم SDK متاحة كشيفرة.

## كيف ترتبط المكونات

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

السجل هو مصدر الحقيقة، أما الإسقاطات والفهارس فهي مشاهد مشتقة. حفظ ادعاء لا يثبت صحته. ينتج التنشيط سياقًا موسومًا بالأدلة، لا تعليمات قابلة للتنفيذ. [البنية](docs/concepts/architecture.md) · [نموذج الصلاحيات](docs/concepts/authority.md)

## البناء والتثبيت من المصدر

تحتاج إلى بيئة Unix مع Git وrustup وأدوات C/C++ الأصلية: المترجم والرابط وmake وCMake وPerl وpkg-config. يثبت المستودع Rust 1.93.0. يجلب البناء اعتماديات Rust ومترجم protobuf؛ ويتطلب تنزيلها اتصالًا بالشبكة.

```sh
git clone https://github.com/Sidiora-Labs/HyperMind-engine.git
cd HyperMind-engine
rustup toolchain install 1.93.0 --profile minimal
cargo build --locked -p hm-cli -p hm-mcp
cargo install --path crates/hm-cli --locked
cargo install --path crates/hm-mcp --locked
```

تثبت هذه الأوامر نسخة المصدر الحالية، ولا تعني وجود حزم منشورة. أضف مجلد ملفات Cargo التنفيذية إلى PATH الخاص بالعميل. لا يحتاج المسار اللفظي الافتراضي إلى تنزيل نموذج أو مفتاح مزود. [دليل التثبيت](docs/start/quickstart.md)

## توصيل عميل MCP

بعد التثبيت، هيئ الحالة الخاصة وسجل خادم stdio. يستخدم مثال الأمرين هذا Claude Code:

```sh
hm init --path .hypermind --json
claude mcp add hypermind -- hm-mcp --config .hypermind/hypermind.conf
```

للعملاء الآخرين، استخدم إعداد MCP مكافئًا. استبدل مسار الإعداد بمسار مطلق وتأكد من قدرة العميل على العثور على `hm-mcp`:

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

جرّب `remember` مع `{"conversation":"demo","kind":"user","content":"The region is eu-central-1."}` ثم `recall` مع `{"mode":"lexical","query":"region","limit":5}`. يشمل [فهرس الأدوات الأربع عشرة](docs/reference/generated/tools.md) remember وrecall وactivate وbelieve وretract وdispute وintend وbind وpredict وoutcome وattest وconsolidate وinspect وforget.

اسمح بمالك واحد فقط لكل مجلد actor: إما MCP أو الخدمة أو المحرك المضمّن، دون كتّاب متزامنين. يحتوي الإعداد المولد على مفاتيح ورموز صلاحيات actor/المسؤول؛ احتفظ به سريًا وخارج التحكم بالإصدارات.

## استخدام الخدمة وواجهة CLI

أوقف مالك MCP أولًا. شغّل الخدمة في طرفية:

```sh
hm serve --config .hypermind/hypermind.conf --json
```

في طرفية أخرى، أضف ذكرى واسترجع معرّفات السجل واطلب حزمة سياق:

```sh
hm remember --config .hypermind/hypermind.conf --conversation demo \
  --content "The deployment region is eu-central-1." --json
hm recall --config .hypermind/hypermind.conf --query "deployment region" --json
hm activate --config .hypermind/hypermind.conf --conversation demo \
  --query "deployment region" --budget-tokens 1024 --json
```

يعيد `recall` في الخدمة حاليًا `lsns` وليس إجابة معروضة، بينما يعيد `activate` حزمة HMA1 بترميز base64. استخدم MCP أو عارض SDK لقراءة نص الذاكرة. خيار CLI المسمى `--embedded` بديل متاح فقط بعد إيقاف الخدمة. [عقود CLI](docs/reference/cli.md)

## الوصول البعيد يتطلب TLS متبادلًا

يجب تفعيل مستمعي الشبكة صراحةً. وفّر شهادة الخادم ومفتاحه ومرجع شهادات عملاء موثوقًا ورمز صلاحية صالحًا لدى العميل. شغّل هذا بدل الخدمة المحلية؛ يجب تجهيز الشهادات المشار إليها مسبقًا:

```sh
hm serve --config .hypermind/hypermind.conf \
  --grpc-bind 127.0.0.1:7443 --rest-bind 127.0.0.1:8443 \
  --tls-cert /secure/server.pem --tls-key /secure/server.key \
  --tls-client-ca /secure/client-ca.pem --json
```

افصل مستمع actor عن مستمع المسؤول؛ شهادة العميل وحدها لا تمنح صلاحية actor. راجع [النشر البعيد](docs/guides/deployment.md) و[عقود البروتوكول](docs/reference/protocol.md) و[إعداد Docker وCompose وsystemd وHelm](deploy/README.md). لا يُفترض وجود صورة منشورة مسبقًا.

## نقاط دخول SDK

توجد حزم SDK في هذا المستودع. نشر الحزم وتأهيل الإصدار بين اللغات عمل منفصل؛ وجود المصدر لا يضمن توافر حزم npm أو PyPI أو ملفات تنفيذية جاهزة.

| اللغة | المصدر | نقاط الدخول |
| --- | --- | --- |
| Rust | [hm-serve](crates/hm-serve/src/embedded.rs) | `HyperMind::open`, `session`, `remember`, `recall`, `activate`, `render` |
| TypeScript | [sdk/typescript](sdk/typescript) | `@hypermind/client`: `Client.connect`; `engine`, `render`, `migrate` |
| Python | [sdk/python](sdk/python) | `Engine.open`, `session`, `remember`, `recall`, `activate`, `render`; `Client` (mTLS) |
| Go | [sdk/go](sdk/go) | `centra/core/cortexclient`: `Dial`, `Session`, `Remember`, `Recall`, `Activate` |

اختر مالكًا مضمّنًا أو عميلًا للخدمة للـactor نفسه، لا كليهما. يتطلب Python الإصدار 3.10 أو أحدث، ويحدد Go الإصدار 1.25.0. توجد سكربتات TypeScript في مساحة عمله. راجع [إرشادات SDK](docs/reference/sdks.md) و[فهرس API المستخرج من المصدر](docs/reference/generated/sdk-api.md) للتواقيع الحالية.

## مزودون اختياريون عبر Centra

تعمل الذاكرة اللفظية المحلية دون مزود بعيد. فعّل الميزات اللازمة فقط في بيئة العملية المالكة؛ يفعل هذا المثال المسارات الثلاثة صراحةً:

```sh
export CENTRA_GATEWAY_URL="https://gateway.centra.ag/v1"
export CENTRA_GATEWAY_API_KEY="<your-secret-key>"
export HM_EMBEDDING_PROVIDER=centra
export HM_RECONSTRUCTION_PROVIDER=centra
export HM_CONSOLIDATION_PROVIDER=centra
```

تستخدم التضمينات `openrouter/openai/text-embedding-3-large`. تستخدم إعادة البناء والتوحيد **`openrouter/openai/gpt-5.6-luna` عبر `CENTRA_GATEWAY_URL`**. لا تغير هذه الخيارات إعداد الاختبارات المعيارية المستقل. تسجيلات الاختبار التاريخية ليست استدعاءات جديدة للمزود.

لا تحفظ مفاتيح حقيقية في Git أو JSON الخاص بـMCP. ترسل الاستعانة بالمزود محتوى مختارًا خارج العملية وقد تترتب عليها تكاليف؛ احصل مسبقًا على إذن نقل البيانات وحدد ميزانية إنفاق. تنزيل النماذج اختياري ولا يفعّل بحد ذاته استدلال ONNX المحلي. [الإعدادات](docs/reference/config.md)

## الأدلة والصلاحيات والقيود

- الذاكرة المسترجعة بيانات غير موثوقة، وليست تعليمات نظام أو مطور ولا إذنًا بالتصرف.
- تحتفظ الادعاءات والتصديقات ونتائج الأدوات المرصودة والإجراءات المشتقة بأدوار أدلة مختلفة.
- تنظم ساعات الهدوء وسياسة الانتباه المتابعة؛ التنبؤ ليس دليلًا على وقوع النتيجة.
- تشفير السجل لا يعني تشفير جميع الإسقاطات أو الصادرات أو السجلات التشغيلية أو مخازن SDK. احمِ مجلد الحالة كاملًا.
- الملكية ذات الكاتب الواحد ضرورية. هذا ليس قاعدة بيانات موزعة متعددة الكتّاب ولا خزنة بيانات اعتماد.
- افحص `ok` و`health` و`gaps` والمصادر؛ نجاح النقل لا يضمن اكتمال الذاكرة أو صحتها.

اقرأ [نموذج التهديدات](docs/security/threat-model.md) قبل إتاحة الخدمة أو استيراد سجل غير موثوق.

## المعايير: الأهداف ليست نتائج

تحدد المواصفات أهداف القبول التالية:

| الفحص | هدف، وليس نتيجة مقاسة |
| --- | --- |
| LongMemEval | 500 سؤال؛ الدقة ≥ 0.90 |
| LoCoMo | تغطية كاملة؛ F1 غير عدائي ≥ 0.75 |
| Recall@10 | ≥ 0.95 عند 10 آلاف عنصر |
| التنشيط بعد الإحماء | p99 < 10 ms عند 100 ألف عنصر |

حقق تشغيل LongMemEval المحلي الكامل **459/500 (91.8%)** باستخدام Luna عبر Centra واسترجاع معجمي فقط. تشغيل LoCoMo جارٍ، وتأهيله ما زال معلقًا. نجح أيضًا سيناريو slice-7 الحقيقي والمحدد. لا تثبت هذه النتائج نجاح CI المستضاف أو تأهيل الإصدار. راجع [الحالة المقاسة والأدلة](docs/evaluation/results.md) و[منهجية التقييم](docs/evaluation/methodology.md).

## التطوير وفحص الوثائق

من جذر المستودع، استخدم السيناريو المحدد وفحوص الوثائق أدناه. تحتاج أدوات الوثائق أيضًا إلى Node.js وmdBook:

```sh
cargo test -p hm-sim --test slice7_journey
node docs/tools/catalog.mjs --write
cargo run -p hm-eval -- docs-gate
mdbook build docs
```

يعيد أمر الفهرس توليد الواجهات العامة الموثقة من المصدر الفعلي. يرفض `docs-gate` التغطية القديمة والروابط المحلية المعطلة؛ لكنه لا يصادق على دقة النص أو الروابط الخارجية. اتبع [AGENTS.md](AGENTS.md) و[مواصفات المهام](spec/hypermind-01/spec.kvx).

## خريطة المستودع

| المسار | المحتوى |
| --- | --- |
| `crates/` | نواة Rust والتخزين والإدراك والواجهات وCLI وأدوات التقييم |
| `schemas/` | عقود FlatBuffers وprotobuf الأساسية |
| `sdk/` | مصادر حزم TypeScript وPython وGo |
| `docs/` | mdBook والمراجع المولدة وADR-001–010 |
| `eval/` | أدوات البيانات وتعريفات الاختبارات وتقارير الأدلة |
| `deploy/` | بناء الحاوية من المصدر وملفات النشر |
| `spec/` | المتطلبات والتصميم وسير العمل وحالة المهام |

ابدأ من [فهرس الوثائق](docs/SUMMARY.md) (بالإنجليزية).

## الترخيص

Apache-2.0 وفق تصريح مساحة عمل Rust. راجع [LICENSE](LICENSE). افحص تراخيص النماذج ومجموعات البيانات الأصلية بشكل مستقل؛ ترخيص المشروع لا يحل محل تراخيصها.
