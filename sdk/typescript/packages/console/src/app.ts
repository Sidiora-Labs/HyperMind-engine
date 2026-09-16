import { buildOverview, renderOverview } from "./overview.js";
import { restTransport } from "./transport.js";

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
  try {
    mount.innerHTML = renderOverview(buildOverview(await transport.callTool("inspect", {})));
  } catch (error) {
    mount.textContent = error instanceof Error ? error.message : "the console could not read the daemon";
  }
}

document.getElementById("console-connection")?.addEventListener("submit", (event) => {
  void load(event);
});
