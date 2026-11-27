import type { RecentView } from "./types";

const MINUTE_MS = 60_000;

/** A gradient "screenshot" for the browser preview, as an SVG data URL. */
function sampleThumbnail(from: string, to: string): string {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="320" height="180"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="${from}"/><stop offset="1" stop-color="${to}"/></linearGradient></defs><rect width="320" height="180" fill="url(#g)"/><rect x="24" y="24" width="150" height="14" rx="7" fill="white" fill-opacity="0.7"/><rect x="24" y="50" width="220" height="10" rx="5" fill="white" fill-opacity="0.4"/><rect x="24" y="68" width="190" height="10" rx="5" fill="white" fill-opacity="0.4"/></svg>`;
  return `data:image/svg+xml;base64,${btoa(svg)}`;
}

function item(partial: Partial<RecentView> & Pick<RecentView, "id" | "kind">): RecentView {
  return {
    origin: "copy",
    createdAtMs: Date.now(),
    preview: null,
    textLength: 0,
    files: [],
    thumbnail: null,
    imageSize: null,
    sensitive: false,
    sourceApp: null,
    ...partial,
  };
}

/** Browser-preview stand-in for recents/mod.rs, with a few sample items. */
export function createPreviewRecents(emit: (event: string, payload: unknown) => void) {
  const now = Date.now();
  let items: RecentView[] = [
    item({
      id: "shot",
      kind: "image",
      origin: "screenshot",
      createdAtMs: now - 2 * MINUTE_MS,
      thumbnail: sampleThumbnail("#3b82f6", "#a855f7"),
      imageSize: [1920, 1080],
    }),
    item({
      id: "text",
      kind: "text",
      createdAtMs: now - 6 * MINUTE_MS,
      preview: "git commit -m \"feat(timer): countdown and Pomodoro widget\" && git push origin dev",
      textLength: 78,
      sourceApp: "WindowsTerminal",
    }),
    item({
      id: "photo",
      kind: "files",
      origin: "shelf",
      createdAtMs: now - 4 * MINUTE_MS,
      thumbnail: sampleThumbnail("#0ea5e9", "#a855f7"),
      files: [{ path: "C:\\Users\\Preview\\Pictures\\Vacances 2026.jpg", name: "Vacances 2026.jpg", exists: true }],
    }),
    item({
      id: "invoice",
      kind: "files",
      createdAtMs: now - 6 * MINUTE_MS,
      files: [
        {
          path: "C:\\Users\\Preview\\Downloads\\facture-WH-60391934-2026-11-mensuel.pdf",
          name: "facture-WH-60391934-2026-11-mensuel.pdf",
          exists: true,
        },
      ],
    }),
    item({ id: "secret", kind: "text", createdAtMs: now - 14 * MINUTE_MS, textLength: 20, sensitive: true, sourceApp: "Bitwarden" }),
    item({
      id: "files",
      kind: "files",
      origin: "shelf",
      createdAtMs: now - 50 * MINUTE_MS,
      files: [
        { path: "C:\\Users\\Preview\\Documents\\Rapport annuel.pdf", name: "Rapport annuel.pdf", exists: true },
        { path: "C:\\Users\\Preview\\Documents\\Budget.xlsx", name: "Budget.xlsx", exists: true },
      ],
    }),
    item({
      id: "address",
      kind: "text",
      createdAtMs: now - 3 * 60 * MINUTE_MS,
      preview: "12 rue des Lilas, 75011 Paris",
      textLength: 29,
      sourceApp: "Vivaldi",
    }),
  ];

  const publish = () => emit("recents:changed", items);

  return {
    list: () => items,
    copy: (id: string) => {
      const found = items.find((candidate) => candidate.id === id);
      if (!found) return;
      items = [{ ...found, createdAtMs: Date.now() }, ...items.filter((candidate) => candidate.id !== id)];
      publish();
    },
    remove: (id: string) => {
      items = items.filter((candidate) => candidate.id !== id);
      publish();
    },
    clear: () => {
      items = [];
      publish();
    },
    addFiles: (paths: string[]) => {
      const files = paths.map((path) => ({ path, name: path.split("\\").pop() ?? path, exists: true }));
      items = [item({ id: `drop-${Date.now()}`, kind: "files", origin: "shelf", files }), ...items];
      publish();
    },
    /** Simulates Win+Shift+S. */
    screenshot: () => {
      const shot = item({
        id: `shot-${Date.now()}`,
        kind: "image",
        origin: "screenshot",
        thumbnail: sampleThumbnail("#f97316", "#ec4899"),
        imageSize: [1280, 720],
      });
      items = [shot, ...items];
      publish();
      emit("recents:screenshot", shot);
    },
  };
}
