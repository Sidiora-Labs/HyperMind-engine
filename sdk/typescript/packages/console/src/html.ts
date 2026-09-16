const ESCAPES: ReadonlyArray<readonly [RegExp, string]> = [
  [/&/g, "&amp;"],
  [/</g, "&lt;"],
  [/>/g, "&gt;"],
  [/"/g, "&quot;"],
  [/'/g, "&#39;"],
];

export function escapeText(value: string): string {
  return ESCAPES.reduce((text, [pattern, replacement]) => text.replace(pattern, replacement), value);
}

export function uriLink(uri: string): string {
  return `<a class="uri" href="#${escapeText(encodeURIComponent(uri))}">${escapeText(uri)}</a>`;
}

export function section(title: string, body: string): string {
  return `<section class="${escapeText(title.toLowerCase().replace(/[^a-z0-9]+/g, "-"))}"><h2>${escapeText(title)}</h2>${body}</section>`;
}
