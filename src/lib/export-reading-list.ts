import type { RssArticle } from "./rss-reader";
import type { OfflineCache, StoryTags } from "./reading-store";

export function buildReadingListMarkdown(
  articles: RssArticle[],
  savedIds: ReadonlySet<string>,
  storyTags: StoryTags,
  offlineCache: OfflineCache,
): string {
  const saved = articles.filter((article) => savedIds.has(article.id));
  const lines: string[] = ["# Saved reading list", "", `Exported ${new Date().toLocaleString()}`, ""];
  for (const article of saved) {
    const tags = storyTags[article.id] ?? [];
    lines.push(`## ${article.title}`);
    lines.push("");
    lines.push(`- Source: ${article.source}`);
    lines.push(`- Link: ${article.link}`);
    if (tags.length) lines.push(`- Tags: ${tags.join(", ")}`);
    const cached = offlineCache[article.id]?.text;
    if (cached) {
      lines.push("");
      lines.push(cached);
    } else if (article.description) {
      lines.push("");
      lines.push(article.description);
    }
    lines.push("");
    lines.push("---");
    lines.push("");
  }
  return lines.join("\n");
}

export function downloadReadingList(
  articles: RssArticle[],
  savedIds: ReadonlySet<string>,
  storyTags: StoryTags,
  offlineCache: OfflineCache,
): void {
  const contents = buildReadingListMarkdown(articles, savedIds, storyTags, offlineCache);
  const blob = new Blob([contents], { type: "text/markdown" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = "marginalia-reading-list.md";
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  URL.revokeObjectURL(url);
}
