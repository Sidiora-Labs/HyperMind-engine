import { buildAccessView, renderAccess } from "./access.js";
import { createActivityFeed, renderActivity } from "./activity.js";
import {
  applyDomainAction,
  compileDomainProfile,
  DomainAction,
  DomainField,
  DomainProfile,
  emptyDomainProfile,
  loadDomainProfile,
  persistDomainProfile,
  renderDomainProfile,
} from "./domain-profile.js";
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
import {
  findResumableSession,
  purgeExpiredSessions,
  renderUploadSession,
  resumeUpload,
  webStorageSessionStore,
} from "./upload-session.js";

interface Session {
  transport: ConsoleTransport;
  actor: number;
}

const KINDS: DomainField["kind"][] = ["text", "number", "date", "identifier"];

let session: Session | undefined;
let profile: DomainProfile = emptyDomainProfile();
let notice = "";

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

async function showAccess(mount: HTMLElement): Promise<void> {
  if (session === undefined) return;
  const panel = document.createElement("div");
  panel.id = "console-access";
  mount.append(panel);
  try {
    const envelope = await session.transport.callTool("inspect", { uri: `hm://${session.actor}/access` });
    panel.innerHTML = renderAccess(buildAccessView(envelope));
  } catch (error) {
    panel.textContent = failure(error);
  }
}

function editor(): string {
  const kinds = KINDS.map((kind) => `<option value="${kind}">${kind}</option>`).join("");
  return [
    `<form id="domain-profile-editor">`,
    `<label for="domain-type-name">Type name</label>`,
    `<input id="domain-type-name" name="type-name" type="text" />`,
    `<label for="domain-type-id">Type id</label>`,
    `<input id="domain-type-id" name="type-id" type="text" />`,
    `<label for="domain-field-name">Field name</label>`,
    `<input id="domain-field-name" name="field-name" type="text" />`,
    `<label for="domain-field-kind">Field kind</label>`,
    `<select id="domain-field-kind" name="field-kind">${kinds}</select>`,
    `<label for="domain-field-required">Field required</label>`,
    `<input id="domain-field-required" name="field-required" type="checkbox" />`,
    `<button type="submit" name="intent" value="add_type">Add type</button>`,
    `<button type="submit" name="intent" value="rename_type">Rename type</button>`,
    `<button type="submit" name="intent" value="delete_type">Delete type</button>`,
    `<button type="submit" name="intent" value="duplicate_type">Duplicate type</button>`,
    `<button type="submit" name="intent" value="add_field">Add field</button>`,
    `<button type="submit" name="intent" value="delete_field">Delete field</button>`,
    `<button type="submit" name="intent" value="store">Store profile</button>`,
    `<button type="submit" name="intent" value="load">Load stored profile</button>`,
    `</form>`,
  ].join("");
}

function paintDomainProfile(): void {
  const panel = document.getElementById("console-domain-profile");
  if (panel === null) return;
  const message = notice === "" ? "" : `<p class="notice">${escapeText(notice)}</p>`;
  panel.innerHTML = renderDomainProfile(profile, compileDomainProfile(profile)) + editor() + message;
}

function showActivity(mount: HTMLElement): void {
  if (session === undefined) return;
  const panel = document.createElement("div");
  panel.id = "console-activity";
  panel.innerHTML = renderActivity(createActivityFeed({ actorKey: String(session.actor) }));
  mount.append(panel);
}

async function showUploadSession(mount: HTMLElement, transport: ConsoleTransport): Promise<void> {
  const panel = document.createElement("div");
  panel.id = "console-upload-session";
  panel.innerHTML = await uploadSection(transport);
  mount.append(panel);
}

async function uploadSection(transport: ConsoleTransport): Promise<string> {
  const store = webStorageSessionStore(window.localStorage);
  await purgeExpiredSessions(store);
  const waiting = await findResumableSession(store);
  if (waiting === null) {
    return section("Upload session", "<p>No upload session is waiting to be resumed.</p>");
  }
  const resumed = await resumeUpload(transport, store, waiting);
  if (!resumed.persisted) {
    return `${renderUploadSession(resumed)}<p>Nothing reached storage: this upload session was written nowhere and will not survive a reload.</p>`;
  }
  if (resumed.degraded) {
    return `${renderUploadSession(resumed)}<p>This upload session is degraded: its record was written but the documents still waiting to be sent were not, so they must be chosen again before the upload can finish.</p>`;
  }
  return renderUploadSession(resumed);
}

function showDomainProfile(mount: HTMLElement): void {
  const panel = document.createElement("div");
  panel.id = "console-domain-profile";
  mount.append(panel);
  paintDomainProfile();
}

function fieldOf(values: FormData): DomainField {
  const kind = String(values.get("field-kind") ?? "text");
  return {
    name: String(values.get("field-name") ?? ""),
    kind: KINDS.includes(kind as DomainField["kind"]) ? (kind as DomainField["kind"]) : "text",
    required: values.get("field-required") !== null,
  };
}

function actionOf(values: FormData, intent: string): DomainAction | undefined {
  const name = String(values.get("type-name") ?? "");
  const id = String(values.get("type-id") ?? "");
  const field = String(values.get("field-name") ?? "");
  switch (intent) {
    case "add_type":
      return { type: "add_type", name };
    case "rename_type":
      return { type: "rename_type", id, name };
    case "delete_type":
      return { type: "delete_type", id };
    case "duplicate_type":
      return { type: "duplicate_type", id };
    case "add_field":
      return { type: "add_field", id, field: fieldOf(values) };
    case "delete_field":
      return { type: "delete_field", id, name: field };
    default:
      return undefined;
  }
}

async function edit(form: HTMLFormElement, intent: string): Promise<void> {
  if (intent === "store") {
    if (session === undefined) {
      notice = "connect to the daemon before storing the domain profile";
      return;
    }
    const provenance = await persistDomainProfile(session.transport, profile);
    notice = `stored domain profile version ${profile.version} as ${provenance.join(", ")}`;
    return;
  }
  if (intent === "load") {
    if (session === undefined) {
      notice = "connect to the daemon before loading the domain profile";
      return;
    }
    const stored = await loadDomainProfile(session.transport);
    if (stored === undefined) {
      notice = "no domain profile has been stored yet";
      return;
    }
    profile = stored;
    notice = `loaded domain profile version ${stored.version}`;
    return;
  }
  const action = actionOf(new FormData(form), intent);
  if (action === undefined) return;
  profile = applyDomainAction(profile, action);
  notice = "";
}

async function runEdit(form: HTMLFormElement, intent: string): Promise<void> {
  try {
    await edit(form, intent);
  } catch (error) {
    notice = failure(error);
  }
  paintDomainProfile();
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
  notice = "";
  const summary = document.createElement("div");
  summary.id = "console-overview";
  summary.innerHTML = renderOverview(overview);
  mount.append(summary);
  showActivity(mount);
  await showSources(mount);
  await showEvidence(mount, values);
  await showAccess(mount);
  showDomainProfile(mount);
  await showUploadSession(mount, transport);
}

async function load(event: Event): Promise<void> {
  event.preventDefault();
  await render();
}

document.getElementById("console-connection")?.addEventListener("submit", (event) => {
  void load(event);
});

document.addEventListener("submit", (event) => {
  const form = event.target;
  if (!(form instanceof HTMLFormElement) || form.id !== "domain-profile-editor") return;
  event.preventDefault();
  const submitter = event.submitter;
  void runEdit(form, submitter instanceof HTMLButtonElement ? submitter.value : "");
});

window.addEventListener("hashchange", () => {
  void render();
});
