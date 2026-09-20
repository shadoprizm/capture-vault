import { useEffect, useMemo, useState } from "react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Icon } from "./components/Icon";
import "./App.css";

type CaptureMode = "area" | "screen";

type CaptureRecord = {
  id: string;
  createdAt: string;
  filePath: string;
  width: number;
  height: number;
  captureMode: CaptureMode;
  note: string;
  tags: string[];
  favorite: boolean;
};

type LibraryFilter = "all" | "favorites";

const isTauriRuntime = "__TAURI_INTERNALS__" in window;

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

function fileName(path: string) {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] ?? path;
}

function formatCaptureDate(value: string) {
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  }).format(new Date(value));
}

function imageSource(capture: CaptureRecord) {
  return isTauriRuntime ? convertFileSrc(capture.filePath) : "";
}

function App() {
  const [captures, setCaptures] = useState<CaptureRecord[]>([]);
  const [filter, setFilter] = useState<LibraryFilter>("all");
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(true);
  const [capturing, setCapturing] = useState<CaptureMode | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [draftNote, setDraftNote] = useState("");
  const [draftTags, setDraftTags] = useState("");
  const [saving, setSaving] = useState(false);

  const selected = captures.find((capture) => capture.id === selectedId) ?? null;

  const visibleCaptures = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return captures.filter((capture) => {
      if (filter === "favorites" && !capture.favorite) return false;
      if (!needle) return true;
      return [
        capture.note,
        capture.captureMode,
        fileName(capture.filePath),
        ...capture.tags,
      ].some((value) => value.toLowerCase().includes(needle));
    });
  }, [captures, filter, query]);

  useEffect(() => {
    async function loadLibrary() {
      if (!isTauriRuntime) {
        setNotice("Browser preview: run npm run tauri dev to capture your screen.");
        setLoading(false);
        return;
      }

      try {
        setCaptures(await invoke<CaptureRecord[]>("list_captures"));
      } catch (loadError) {
        setError(errorMessage(loadError));
      } finally {
        setLoading(false);
      }
    }

    void loadLibrary();
  }, []);

  useEffect(() => {
    setDraftNote(selected?.note ?? "");
    setDraftTags(selected?.tags.join(", ") ?? "");
  }, [selectedId, selected?.note, selected?.tags]);

  async function capture(mode: CaptureMode) {
    if (!isTauriRuntime) {
      setNotice("Screen capture is available in the desktop app.");
      return;
    }

    setCapturing(mode);
    setError(null);
    const appWindow = getCurrentWindow();

    try {
      await appWindow.hide();
      await new Promise((resolve) => window.setTimeout(resolve, 180));
      const created = await invoke<CaptureRecord>("capture_screen", { mode });
      setCaptures((current) => [created, ...current]);
      setSelectedId(created.id);
      setNotice("Screenshot saved to your private library.");
    } catch (captureError) {
      const message = errorMessage(captureError);
      if (!message.toLowerCase().includes("cancel")) setError(message);
    } finally {
      await appWindow.show();
      await appWindow.setFocus();
      setCapturing(null);
    }
  }

  async function updateMetadata(
    capture: CaptureRecord,
    favorite = capture.favorite,
  ) {
    if (!isTauriRuntime) return;
    setSaving(true);
    setError(null);

    try {
      const updated = await invoke<CaptureRecord>("update_capture_metadata", {
        id: capture.id,
        note: draftNote,
        tags: draftTags.split(","),
        favorite,
      });
      setCaptures((current) =>
        current.map((item) => (item.id === updated.id ? updated : item)),
      );
      setNotice(favorite !== capture.favorite ? "Favorite updated." : "Details saved.");
    } catch (saveError) {
      setError(errorMessage(saveError));
    } finally {
      setSaving(false);
    }
  }

  async function toggleFavorite(capture: CaptureRecord) {
    if (!isTauriRuntime) return;
    setSaving(true);
    try {
      const updated = await invoke<CaptureRecord>("update_capture_metadata", {
        id: capture.id,
        note: capture.note,
        tags: capture.tags,
        favorite: !capture.favorite,
      });
      setCaptures((current) =>
        current.map((item) => (item.id === updated.id ? updated : item)),
      );
    } catch (favoriteError) {
      setError(errorMessage(favoriteError));
    } finally {
      setSaving(false);
    }
  }

  async function removeCapture(capture: CaptureRecord) {
    if (!window.confirm("Delete this screenshot permanently?")) return;
    if (!isTauriRuntime) return;

    try {
      await invoke("delete_capture", { id: capture.id });
      setCaptures((current) => current.filter((item) => item.id !== capture.id));
      setSelectedId(null);
      setNotice("Screenshot deleted.");
    } catch (deleteError) {
      setError(errorMessage(deleteError));
    }
  }

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark" aria-hidden="true">
            <Icon name="focus" size={22} />
          </div>
          <div>
            <strong>CaptureVault</strong>
            <span>Private by default</span>
          </div>
        </div>

        <nav className="library-nav" aria-label="Screenshot library">
          <button
            className={filter === "all" ? "active" : ""}
            onClick={() => setFilter("all")}
          >
            <Icon name="grid" />
            Library
            <span>{captures.length}</span>
          </button>
          <button
            className={filter === "favorites" ? "active" : ""}
            onClick={() => setFilter("favorites")}
          >
            <Icon name="star" />
            Favorites
            <span>{captures.filter((capture) => capture.favorite).length}</span>
          </button>
        </nav>

        <div className="privacy-card">
          <Icon name="lock" size={18} />
          <div>
            <strong>Stored locally</strong>
            <p>Your screenshots never leave this device.</p>
          </div>
        </div>

        <p className="version">Early preview · v0.1.0</p>
      </aside>

      <main className="workspace">
        <header className="topbar">
          <div>
            <p className="eyebrow">Screenshot library</p>
            <h1>{filter === "all" ? "Your captures" : "Favorites"}</h1>
          </div>

          <div className="capture-actions">
            <button
              className="secondary-action"
              onClick={() => void capture("screen")}
              disabled={capturing !== null}
            >
              <Icon name="monitor" />
              {capturing === "screen" ? "Capturing…" : "Full screen"}
            </button>
            <button
              className="primary-action"
              onClick={() => void capture("area")}
              disabled={capturing !== null}
            >
              <Icon name="crop" />
              {capturing === "area" ? "Select an area…" : "Capture area"}
            </button>
          </div>
        </header>

        <section className="library-toolbar" aria-label="Library controls">
          <label className="search-box">
            <Icon name="search" size={18} />
            <input
              value={query}
              onChange={(event) => setQuery(event.currentTarget.value)}
              placeholder="Search notes, tags, and captures"
              aria-label="Search captures"
            />
            {query && (
              <button onClick={() => setQuery("")} aria-label="Clear search">
                <Icon name="close" size={16} />
              </button>
            )}
          </label>
          <span className="result-count">
            {visibleCaptures.length} {visibleCaptures.length === 1 ? "capture" : "captures"}
          </span>
        </section>

        {error && (
          <div className="message error-message" role="alert">
            <Icon name="warning" size={18} />
            <span>{error}</span>
            <button onClick={() => setError(null)} aria-label="Dismiss error">
              <Icon name="close" size={16} />
            </button>
          </div>
        )}

        {notice && (
          <div className="message notice-message" role="status">
            <Icon name="check" size={18} />
            <span>{notice}</span>
            <button onClick={() => setNotice(null)} aria-label="Dismiss message">
              <Icon name="close" size={16} />
            </button>
          </div>
        )}

        {loading ? (
          <div className="loading-grid" aria-label="Loading captures">
            {Array.from({ length: 6 }).map((_, index) => (
              <div className="loading-card" key={index} />
            ))}
          </div>
        ) : visibleCaptures.length > 0 ? (
          <section className="capture-grid" aria-label="Captures">
            {visibleCaptures.map((capture) => (
              <article
                className="capture-card"
                key={capture.id}
                onClick={() => setSelectedId(capture.id)}
              >
                <div className="capture-preview">
                  <img src={imageSource(capture)} alt={capture.note || "Screenshot"} />
                  <button
                    className={`favorite-button ${capture.favorite ? "selected" : ""}`}
                    onClick={(event) => {
                      event.stopPropagation();
                      void toggleFavorite(capture);
                    }}
                    aria-label={capture.favorite ? "Remove from favorites" : "Add to favorites"}
                  >
                    <Icon name="star" size={17} filled={capture.favorite} />
                  </button>
                  <span className="mode-pill">
                    {capture.captureMode === "area" ? "Area" : "Screen"}
                  </span>
                </div>
                <div className="capture-card-copy">
                  <div>
                    <strong>{capture.note || fileName(capture.filePath)}</strong>
                    <span>{formatCaptureDate(capture.createdAt)}</span>
                  </div>
                  <span className="dimensions">
                    {capture.width} × {capture.height}
                  </span>
                </div>
                {capture.tags.length > 0 && (
                  <div className="tag-row">
                    {capture.tags.slice(0, 3).map((tag) => (
                      <span key={tag}>{tag}</span>
                    ))}
                  </div>
                )}
              </article>
            ))}
          </section>
        ) : (
          <section className="empty-state">
            <div className="empty-illustration">
              <div className="empty-window">
                <span />
                <span />
                <span />
                <Icon name={query || filter === "favorites" ? "search" : "focus"} size={36} />
              </div>
            </div>
            <p className="eyebrow">{query ? "No matches" : "A fresh vault"}</p>
            <h2>
              {query
                ? "Try a different search"
                : filter === "favorites"
                  ? "No favorites yet"
                  : "Capture something worth keeping"}
            </h2>
            <p>
              {query
                ? "Search looks through filenames, notes, tags, and capture types."
                : "Choose an area or a full display. Your screenshot will appear here automatically."}
            </p>
            {!query && filter === "all" && (
              <button className="primary-action" onClick={() => void capture("area")}>
                <Icon name="crop" />
                Capture your first area
              </button>
            )}
          </section>
        )}
      </main>

      {selected && (
        <div className="detail-backdrop" onClick={() => setSelectedId(null)}>
          <aside
            className="detail-panel"
            aria-label="Capture details"
            onClick={(event) => event.stopPropagation()}
          >
            <header>
              <div>
                <p className="eyebrow">Capture details</p>
                <h2>{formatCaptureDate(selected.createdAt)}</h2>
              </div>
              <button
                className="icon-button"
                onClick={() => setSelectedId(null)}
                aria-label="Close details"
              >
                <Icon name="close" />
              </button>
            </header>

            <img
              className="detail-image"
              src={imageSource(selected)}
              alt={selected.note || "Selected screenshot"}
            />

            <dl className="capture-facts">
              <div>
                <dt>Type</dt>
                <dd>{selected.captureMode === "area" ? "Selected area" : "Full screen"}</dd>
              </div>
              <div>
                <dt>Size</dt>
                <dd>{selected.width} × {selected.height}</dd>
              </div>
              <div>
                <dt>File</dt>
                <dd title={selected.filePath}>{fileName(selected.filePath)}</dd>
              </div>
            </dl>

            <label className="field">
              <span>Note</span>
              <textarea
                value={draftNote}
                onChange={(event) => setDraftNote(event.currentTarget.value)}
                placeholder="Why did you save this?"
                rows={4}
              />
            </label>

            <label className="field">
              <span>Tags</span>
              <input
                value={draftTags}
                onChange={(event) => setDraftTags(event.currentTarget.value)}
                placeholder="research, design, reference"
              />
              <small>Separate tags with commas.</small>
            </label>

            <div className="detail-actions">
              <button className="danger-action" onClick={() => void removeCapture(selected)}>
                <Icon name="trash" />
                Delete
              </button>
              <button
                className="primary-action"
                disabled={saving}
                onClick={() => void updateMetadata(selected)}
              >
                <Icon name="check" />
                {saving ? "Saving…" : "Save details"}
              </button>
            </div>
          </aside>
        </div>
      )}
    </div>
  );
}

export default App;
