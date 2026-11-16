// Desktop replacement for src/lib/feed-server.ts.
//
// In the browser build the feed XML is fetched by a TanStack Start server
// function, so the request never hits CORS. The desktop build has no server
// runtime either, but the shell process exposes /proxy?url=... on the app
// origin, which performs the request server-side inside the binary.
const FETCH_TIMEOUT_MS = 15000;

const PROXY_ORIGIN = import.meta.env["VITE_FEED_PROXY"] ?? "";

export async function fetchFeedXml(options: { data: string }): Promise<string> {
  const target = typeof options?.data === "string" ? options.data : "";
  if (!target) throw new Error("A feed URL is required.");

  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), FETCH_TIMEOUT_MS);
  try {
    const response = await fetch(`${PROXY_ORIGIN}/proxy?url=${encodeURIComponent(target)}`, {
      signal: controller.signal,
    });
    if (!response.ok) throw new Error(`The feed service returned ${response.status}.`);
    const xml = await response.text();
    if (!xml.trim()) throw new Error("The feed returned an empty response.");
    return xml;
  } finally {
    clearTimeout(timer);
  }
}
