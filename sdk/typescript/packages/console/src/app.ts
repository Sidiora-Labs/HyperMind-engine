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

async function load(event: Event): Promise<void> {
  event.preventDefault();
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
}

document.getElementById("console-connection")?.addEventListener("submit", (event) => {
  void load(event);
});
