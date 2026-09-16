import {
  EvidenceClass,
  buildAnswerLookup,
  buildEvidencePath,
  classifyEvidence,
  renderAnswerLookup,
  renderEvidencePath,
} from "./evidence.js";
import { escapeText, section } from "./html.js";
import { buildOverview, OverviewView, renderOverview } from "./overview.js";
import { buildSourceDetail, buildSourceIndex, renderSourceDetail, renderSourceIndex } from "./sources.js";
import { ConsoleTransport, restTransport } from "./transport.js";

interface Session {
  transport: ConsoleTransport;
  actor: number;
}

let session: Session | undefined;

function failure(error: unknown): string {
  return error instanceof Error ? error.message : "the console could not read the daemon";
}

async function showSource(mount: HTMLElement, conversation: string): Promise<void> {
  if (session === undefined) return;
  try {
    const envelope = await session.transport.callTool("inspect", {
      uri: `hm://${session.actor}/sources/${conversation}`,
    });
    mount.innerHTML = renderSourceDetail(buildSourceDetail(envelope));
  } catch (error) {
    mount.textContent = failure(error);
  }
}

async function showSources(mount: HTMLElement): Promise<void> {
  if (session === undefined) return;
  const index = document.createElement("div");
  index.id = "console-sources";
  const detail = document.createElement("div");
  detail.id = "console-source";
  mount.append(index, detail);
  try {
    const envelope = await session.transport.callTool("inspect", { uri: `hm://${session.actor}/sources` });
    index.innerHTML = renderSourceIndex(buildSourceIndex(envelope));
  } catch (error) {
    index.textContent = failure(error);
    return;
  }
  for (const link of Array.from(index.querySelectorAll("a[data-conversation]"))) {
    link.addEventListener("click", (event) => {
      event.preventDefault();
      const conversation = (link as HTMLElement).dataset["conversation"];
      if (conversation !== undefined) void showSource(detail, conversation);
    });
  }
}

function selectedLsn(actor: number): number | undefined {
  const selected = decodeURIComponent(window.location.hash.replace(/^#/, ""));
  const matched = new RegExp(`^hm://${actor}/lsn/(\\d+)$`).exec(selected);
  return matched === null ? undefined : Number(matched[1]);
}

function renderEvidenceClasses(classes: EvidenceClass[]): string {
  if (classes.length === 0) return "";
  const rows = classes
    .map(
      (entry) =>
        `<tr><td>${escapeText(String(entry.lsn))}</td><td>${escapeText(String(entry.retrieved))}</td><td>${escapeText(String(entry.included))}</td><td>${escapeText(String(entry.attested))}</td></tr>`,
    )
    .join("");
  return section(
    "Evidence classes",
    `<table><thead><tr><th>lsn</th><th>retrieved</th><th>included</th><th>attested</th></tr></thead><tbody>${rows}</tbody></table>`,
  );
}

async function evidence(
  transport: ConsoleTransport,
  actor: number,
  conversation: string,
  query: string,
): Promise<string> {
  const lsn = selectedLsn(actor);
  if (lsn === undefined) return "";
  const walked = await transport.callTool("inspect", { uri: `hm://${actor}/lsn/${lsn}` });
  const lookup = buildAnswerLookup(await transport.callTool("inspect", { uri: `hm://${actor}/evidence/${lsn}` }));
  let classes: EvidenceClass[] = [];
  if (conversation !== "" && query !== "") {
    const activated = await transport.callTool("activate", { conversation, query, budget_tokens: 4096 });
    if (activated.manifest !== undefined) classes = classifyEvidence(activated.manifest, [lookup]);
  }
  return `${renderEvidencePath(buildEvidencePath(walked))}${renderEvidenceClasses(classes)}${renderAnswerLookup(lookup)}`;
}

async function showEvidence(mount: HTMLElement, values: FormData): Promise<void> {
  if (session === undefined) return;
  const panel = document.createElement("div");
  panel.id = "console-evidence";
  mount.append(panel);
  try {
    panel.innerHTML = await evidence(
      session.transport,
      session.actor,
      String(values.get("conversation") ?? ""),
      String(values.get("query") ?? ""),
    );
  } catch (error) {
    panel.textContent = failure(error);
  }
}

async function render(): Promise<void> {
  const form = document.getElementById("console-connection") as HTMLFormElement | null;
  const mount = document.getElementById("console");
  if (form === null || mount === null) return;
  const values = new FormData(form);
  const transport = restTransport({
    baseUrl: String(values.get("base-url") ?? ""),
    token: String(values.get("token") ?? ""),
    connectionId: String(values.get("connection-id") ?? ""),
  });
  mount.textContent = "";
  let overview: OverviewView;
  try {
    overview = buildOverview(await transport.callTool("inspect", {}));
  } catch (error) {
    mount.textContent = failure(error);
    return;
  }
  session = { transport, actor: overview.actor };
  const summary = document.createElement("div");
  summary.id = "console-overview";
  summary.innerHTML = renderOverview(overview);
  mount.append(summary);
  await showSources(mount);
  await showEvidence(mount, values);
}

async function load(event: Event): Promise<void> {
  event.preventDefault();
  await render();
}

document.getElementById("console-connection")?.addEventListener("submit", (event) => {
  void load(event);
});

window.addEventListener("hashchange", () => {
  void render();
});
