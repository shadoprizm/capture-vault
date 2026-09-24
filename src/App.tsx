import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
  type PointerEvent as ReactPointerEvent,
} from "react";
import { startDrag } from "@crabnebula/tauri-plugin-drag";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
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
  title: string;
  description: string;
  note: string;
  tags: string[];
  favorite: boolean;
  ocrText: string;
  enrichmentStatus:
    | "pending"
    | "processing"
    | "complete"
    | "partial"
    | "no_text"
    | "failed";
};

type LibraryFilter = "all" | "favorites";

type ShortcutModifiers =
  | "Control+Alt"
  | "Control+Shift"
  | "Alt+Shift"
  | "Super+Shift"
  | "Super+Alt"
  | "Control+Super"
  | "Control+Alt+Shift";

type ShortcutBinding = {
  enabled: boolean;
  modifiers: ShortcutModifiers;
  key: string;
};

type ShortcutSettings = Record<CaptureMode, ShortcutBinding>;

type VisionSettings = {
  enabled: boolean;
  endpoint: string;
  model: string;
};

type AnalysisSettings = {
  ocrAvailable: boolean;
  ocrError: string | null;
  vision: VisionSettings;
  visionError: string | null;
};

type DragGesture = {
  captureId: string;
  pointerId: number;
  startX: number;
  startY: number;
  started: boolean;
};

type StorageLocationUpdate = {
  path: string;
  captures: CaptureRecord[];
};

const isTauriRuntime = "__TAURI_INTERNALS__" in window;
const DRAG_THRESHOLD = 5;
const SHORTCUT_STORAGE_KEY = "capture-vault.shortcuts.v1";
const SHORTCUT_MODIFIERS: ShortcutModifiers[] = [
  "Control+Alt",
  "Control+Shift",
  "Alt+Shift",
  "Super+Shift",
  "Super+Alt",
  "Control+Super",
  "Control+Alt+Shift",
];
const SHORTCUT_KEYS = [
  ..."ABCDEFGHIJKLMNOPQRSTUVWXYZ".split(""),
  ..."0123456789".split(""),
  ...Array.from({ length: 12 }, (_, index) => `F${index + 1}`),
];
const DEFAULT_SHORTCUTS: ShortcutSettings = {
  area: { enabled: true, modifiers: "Control+Alt", key: "A" },
  screen: { enabled: true, modifiers: "Control+Alt", key: "F" },
};

let shortcutOperation = Promise.resolve();

function queueShortcutOperation<T>(operation: () => Promise<T>) {
  const queued = shortcutOperation.catch(() => undefined).then(operation);
  shortcutOperation = queued.then(
    () => undefined,
    () => undefined,
  );
  return queued;
}

function cloneShortcuts(settings: ShortcutSettings): ShortcutSettings {
  return {
    area: { ...settings.area },
    screen: { ...settings.screen },
  };
}

function isShortcutBinding(value: unknown): value is ShortcutBinding {
  if (!value || typeof value !== "object") return false;
  const binding = value as Partial<ShortcutBinding>;
  return (
    typeof binding.enabled === "boolean" &&
    SHORTCUT_MODIFIERS.includes(binding.modifiers as ShortcutModifiers) &&
    typeof binding.key === "string" &&
    SHORTCUT_KEYS.includes(binding.key)
  );
}

function loadShortcutSettings(): ShortcutSettings {
  try {
    const saved = JSON.parse(localStorage.getItem(SHORTCUT_STORAGE_KEY) ?? "null") as Partial<ShortcutSettings> | null;
    if (saved && isShortcutBinding(saved.area) && isShortcutBinding(saved.screen)) {
      return cloneShortcuts(saved as ShortcutSettings);
    }
  } catch {
    // Invalid or inaccessible local settings fall back to the documented defaults.
  }
  return cloneShortcuts(DEFAULT_SHORTCUTS);
}

function shortcutAccelerator(binding: ShortcutBinding) {
  return `${binding.modifiers}+${binding.key}`;
}

function shortcutLabel(binding: ShortcutBinding) {
  return shortcutAccelerator(binding).replace("Control", "Ctrl").split("+").join(" + ");
}

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

function captureLabel(capture: CaptureRecord) {
  return capture.title || capture.note || fileName(capture.filePath);
}

function enrichmentLabel(status: CaptureRecord["enrichmentStatus"]) {
  switch (status) {
    case "processing":
      return "Analyzing locally…";
    case "complete":
      return "Semantic analysis complete";
    case "partial":
      return "OCR ready; optional vision analysis is off or unavailable";
    case "no_text":
      return "No readable text found";
    case "failed":
      return "Analysis needs another try";
    default:
      return "Ready for local analysis";
  }
}

type ShortcutEditorProps = {
  description: string;
  label: string;
  value: ShortcutBinding;
  onChange: (value: ShortcutBinding) => void;
};

function ShortcutEditor({ description, label, value, onChange }: ShortcutEditorProps) {
  return (
    <div className={`shortcut-editor ${value.enabled ? "" : "disabled"}`}>
      <div className="shortcut-editor-copy">
        <strong>{label}</strong>
        <p>{description}</p>
      </div>
      <label className="shortcut-toggle">
        <input
          type="checkbox"
          checked={value.enabled}
          onChange={(event) => onChange({ ...value, enabled: event.currentTarget.checked })}
        />
        <span>{value.enabled ? "Enabled" : "Disabled"}</span>
      </label>
      <div className="shortcut-selectors" aria-label={`${label} shortcut`}>
        <select
          value={value.modifiers}
          disabled={!value.enabled}
          onChange={(event) =>
            onChange({ ...value, modifiers: event.currentTarget.value as ShortcutModifiers })
          }
          aria-label={`${label} modifiers`}
        >
          {SHORTCUT_MODIFIERS.map((modifiers) => (
            <option value={modifiers} key={modifiers}>
              {modifiers.replace("Control", "Ctrl").split("+").join(" + ")}
            </option>
          ))}
        </select>
        <span aria-hidden="true">+</span>
        <select
          value={value.key}
          disabled={!value.enabled}
          onChange={(event) => onChange({ ...value, key: event.currentTarget.value })}
          aria-label={`${label} key`}
        >
          {SHORTCUT_KEYS.map((key) => (
            <option value={key} key={key}>
              {key}
            </option>
          ))}
        </select>
      </div>
    </div>
  );
}

function App() {
  const [captures, setCaptures] = useState<CaptureRecord[]>([]);
  const [filter, setFilter] = useState<LibraryFilter>("all");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [shortcutSettings, setShortcutSettings] = useState<ShortcutSettings>(loadShortcutSettings);
  const [draftShortcuts, setDraftShortcuts] = useState<ShortcutSettings>(() =>
    cloneShortcuts(shortcutSettings),
  );
  const [savingShortcuts, setSavingShortcuts] = useState(false);
  const [analysisSettings, setAnalysisSettings] = useState<AnalysisSettings | null>(null);
  const [draftVision, setDraftVision] = useState<VisionSettings | null>(null);
  const [savingVision, setSavingVision] = useState(false);
  const [storageLocation, setStorageLocation] = useState("");
  const [changingStorage, setChangingStorage] = useState(false);
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(true);
  const [capturing, setCapturing] = useState<CaptureMode | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [draftTitle, setDraftTitle] = useState("");
  const [draftDescription, setDraftDescription] = useState("");
  const [draftNote, setDraftNote] = useState("");
  const [draftTags, setDraftTags] = useState("");
  const [saving, setSaving] = useState(false);
  const [copyingId, setCopyingId] = useState<string | null>(null);
  const [analyzingId, setAnalyzingId] = useState<string | null>(null);
  const [draggingId, setDraggingId] = useState<string | null>(null);
  const dragGesture = useRef<DragGesture | null>(null);
  const suppressCardClick = useRef(false);
  const selectedIdRef = useRef<string | null>(null);
  const captureInProgressRef = useRef(false);
  const shortcutSettingsRef = useRef(shortcutSettings);

  const selected = captures.find((capture) => capture.id === selectedId) ?? null;

  shortcutSettingsRef.current = shortcutSettings;

  const visibleCaptures = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return captures.filter((capture) => {
      if (filter === "favorites" && !capture.favorite) return false;
      if (!needle) return true;
      return [
        capture.title,
        capture.description,
        capture.note,
        capture.ocrText,
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
        const [savedCaptures, savedLocation] = await Promise.all([
          invoke<CaptureRecord[]>("list_captures"),
          invoke<string>("get_storage_location"),
        ]);
        setCaptures(savedCaptures);
        setStorageLocation(savedLocation);
      } catch (loadError) {
        setError(errorMessage(loadError));
      } finally {
        setLoading(false);
      }
    }

    void loadLibrary();
  }, []);

  useEffect(() => {
    if (!isTauriRuntime) return;
    void invoke<AnalysisSettings>("get_analysis_settings")
      .then((analysis) => {
        setAnalysisSettings(analysis);
        setDraftVision(analysis.vision);
      })
      .catch((analysisError) => setError(`Could not load local analysis settings: ${errorMessage(analysisError)}`));
  }, []);

  useEffect(() => {
    selectedIdRef.current = selectedId;
  }, [selectedId]);

  useEffect(() => {
    if (!isTauriRuntime) return;

    void queueShortcutOperation(() => invoke("configure_shortcuts", {
      settings: shortcutSettingsRef.current,
    })).catch((shortcutError) => {
      setError(`Global shortcuts are unavailable: ${errorMessage(shortcutError)}`);
    });
  }, []);

  useEffect(() => {
    if (!isTauriRuntime) return;

    let disposed = false;
    let stopEnriched: (() => void) | undefined;
    let stopFailed: (() => void) | undefined;
    let stopCreated: (() => void) | undefined;
    let stopCaptureFailed: (() => void) | undefined;

    void Promise.all([
      listen<CaptureRecord>("capture-created", ({ payload }) => {
        setCaptures((current) => {
          const existing = current.find((item) => item.id === payload.id);
          return [existing ?? payload, ...current.filter((item) => item.id !== payload.id)];
        });
        setSelectedId(payload.id);
        setNotice("Screenshot saved to your private library.");
      }),
      listen<string>("capture-failed", ({ payload }) => setError(payload)),
      listen<CaptureRecord>("capture-enriched", ({ payload }) => {
        setCaptures((current) => current.some((capture) => capture.id === payload.id)
          ? current.map((capture) => (capture.id === payload.id ? payload : capture))
          : [payload, ...current]);
        setAnalyzingId((current) => (current === payload.id ? null : current));
        if (selectedIdRef.current === payload.id) {
          setDraftTitle((current) => current || payload.title);
          setDraftDescription((current) => current || payload.description);
        }
      }),
      listen<string>("capture-enrichment-failed", ({ payload }) => {
        setError(payload);
        setAnalyzingId(null);
      }),
    ])
      .then(([unlistenCreated, unlistenCaptureFailed, unlistenEnriched, unlistenFailed]) => {
        if (disposed) {
          unlistenCreated();
          unlistenCaptureFailed();
          unlistenEnriched();
          unlistenFailed();
          return;
        }
        stopCreated = unlistenCreated;
        stopCaptureFailed = unlistenCaptureFailed;
        stopEnriched = unlistenEnriched;
        stopFailed = unlistenFailed;
      })
      .catch((listenError) => setError(errorMessage(listenError)));

    return () => {
      disposed = true;
      stopCreated?.();
      stopCaptureFailed?.();
      stopEnriched?.();
      stopFailed?.();
    };
  }, []);

  useEffect(() => {
    setDraftTitle(selected?.title ?? "");
    setDraftDescription(selected?.description ?? "");
    setDraftNote(selected?.note ?? "");
    setDraftTags(selected?.tags.join(", ") ?? "");
  }, [selectedId]);

  async function saveShortcutSettings() {
    const next = cloneShortcuts(draftShortcuts);
    const enabled = (["area", "screen"] as CaptureMode[])
      .filter((mode) => next[mode].enabled)
      .map((mode) => shortcutAccelerator(next[mode]));

    if (new Set(enabled).size !== enabled.length) {
      setError("Area capture and full screen cannot use the same shortcut.");
      return;
    }

    setSavingShortcuts(true);
    setError(null);
    const previous = cloneShortcuts(shortcutSettingsRef.current);

    try {
      await queueShortcutOperation(() => invoke("configure_shortcuts", { settings: next }));
      localStorage.setItem(SHORTCUT_STORAGE_KEY, JSON.stringify(next));
      shortcutSettingsRef.current = next;
      setShortcutSettings(next);
      setDraftShortcuts(cloneShortcuts(next));
      setNotice("Global shortcuts updated.");
    } catch (shortcutError) {
      shortcutSettingsRef.current = previous;
      setError(
        `Could not register those shortcuts. Another app may already use one: ${errorMessage(shortcutError)}`,
      );
    } finally {
      setSavingShortcuts(false);
    }
  }

  async function saveVisionSettings() {
    if (!draftVision) return;
    setSavingVision(true);
    setError(null);
    try {
      const saved = await invoke<VisionSettings>("set_vision_settings", { settings: draftVision });
      setAnalysisSettings((current) => current && { ...current, vision: saved, visionError: null });
      setDraftVision(saved);
      setNotice(saved.enabled ? "Local AI titles and descriptions enabled." : "Local AI titles and descriptions disabled.");
    } catch (visionError) {
      setError(errorMessage(visionError));
    } finally {
      setSavingVision(false);
    }
  }

  function updateDraftShortcut(mode: CaptureMode, binding: ShortcutBinding) {
    setDraftShortcuts((current) => ({ ...current, [mode]: binding }));
  }

  async function chooseStorageLocation() {
    if (!isTauriRuntime) {
      setNotice("Storage folders can be changed in the desktop app.");
      return;
    }

    let selected: string | string[] | null;
    try {
      selected = await open({
        directory: true,
        multiple: false,
        defaultPath: storageLocation || undefined,
        title: "Choose screenshot storage folder",
      });
    } catch (dialogError) {
      setError(errorMessage(dialogError));
      return;
    }
    if (typeof selected !== "string") return;

    setChangingStorage(true);
    setError(null);
    try {
      const updated = await invoke<StorageLocationUpdate>("set_storage_location", {
        path: selected,
      });
      setStorageLocation(updated.path);
      setCaptures(updated.captures);
      setNotice(
        updated.captures.length > 0
          ? `Storage moved. ${updated.captures.length} ${updated.captures.length === 1 ? "capture" : "captures"} migrated.`
          : "Screenshot storage location updated.",
      );
    } catch (storageError) {
      setError(errorMessage(storageError));
    } finally {
      setChangingStorage(false);
    }
  }

  async function capture(mode: CaptureMode, trigger: "button" | "shortcut" = "button") {
    if (!isTauriRuntime) {
      setNotice("Screen capture is available in the desktop app.");
      return;
    }
    if (captureInProgressRef.current) return;

    captureInProgressRef.current = true;
    setCapturing(mode);
    setError(null);
    const appWindow = getCurrentWindow();
    let wasVisible = true;

    try {
      wasVisible = await appWindow.isVisible();
      await appWindow.hide();
      await new Promise((resolve) => window.setTimeout(resolve, 180));
      const created = await invoke<CaptureRecord>("capture_screen", { mode });
      setCaptures((current) => {
        const existing = current.find((item) => item.id === created.id);
        return [existing ?? created, ...current.filter((item) => item.id !== created.id)];
      });
      setSelectedId(created.id);
      setNotice("Screenshot saved to your private library.");
    } catch (captureError) {
      const message = errorMessage(captureError);
      if (!message.toLowerCase().includes("cancel")) setError(message);
    } finally {
      if (wasVisible) await appWindow.show();
      if (trigger === "button") await appWindow.setFocus();
      captureInProgressRef.current = false;
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
        title: draftTitle,
        description: draftDescription,
        note: draftNote,
        tags: draftTags.split(","),
        favorite,
      });
      setCaptures((current) =>
        current.map((item) => (item.id === updated.id ? updated : item)),
      );
      setNotice(favorite !== capture.favorite ? "Favorite updated." : "Details saved.");
      setSelectedId(null);
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
        title: capture.title,
        description: capture.description,
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

  async function analyzeCapture(capture: CaptureRecord) {
    if (!isTauriRuntime) return;

    setAnalyzingId(capture.id);
    setError(null);
    setCaptures((current) =>
      current.map((item) =>
        item.id === capture.id ? { ...item, enrichmentStatus: "processing" } : item,
      ),
    );

    try {
      const updated = await invoke<CaptureRecord>("enrich_capture", { id: capture.id });
      setCaptures((current) =>
        current.map((item) => (item.id === updated.id ? updated : item)),
      );
      setDraftTitle((current) => current || updated.title);
      setDraftDescription((current) => current || updated.description);
      setNotice(
        updated.enrichmentStatus === "partial"
          ? "OCR is searchable, but the vision service was unavailable."
          : "Semantic title, description, and searchable text are ready.",
      );
    } catch (analysisError) {
      setCaptures((current) =>
        current.map((item) =>
          item.id === capture.id ? { ...item, enrichmentStatus: "failed" } : item,
        ),
      );
      setError(errorMessage(analysisError));
    } finally {
      setAnalyzingId(null);
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

  async function copyCapture(capture: CaptureRecord) {
    if (!isTauriRuntime) {
      setNotice("Copying screenshots is available in the desktop app.");
      return;
    }

    setCopyingId(capture.id);
    setError(null);

    try {
      await invoke("copy_capture", { id: capture.id });
      setNotice("Copied as an image and PNG file. Paste it wherever you need it.");
    } catch (copyError) {
      setError(errorMessage(copyError));
    } finally {
      setCopyingId(null);
    }
  }

  function beginNativeDrag(capture: CaptureRecord) {
    if (!isTauriRuntime) return;

    setDraggingId(capture.id);
    setError(null);

    void startDrag(
      {
        item: [capture.filePath],
        icon: capture.filePath,
        mode: "copy",
      },
      ({ result }) => {
        setDraggingId((current) => (current === capture.id ? null : current));
        if (result === "Dropped") {
          setNotice("Screenshot dropped as a PNG file.");
        }
      },
    ).catch((dragError) => {
      setDraggingId((current) => (current === capture.id ? null : current));
      setError(errorMessage(dragError));
    });
  }

  function startDragGesture(
    event: ReactPointerEvent<HTMLElement>,
    capture: CaptureRecord,
  ) {
    if (!isTauriRuntime || !event.isPrimary || event.button !== 0) return;
    if ((event.target as HTMLElement).closest("button, input, textarea")) return;

    suppressCardClick.current = false;
    dragGesture.current = {
      captureId: capture.id,
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      started: false,
    };
  }

  function continueDragGesture(
    event: ReactPointerEvent<HTMLElement>,
    capture: CaptureRecord,
  ) {
    const gesture = dragGesture.current;
    if (!gesture || gesture.captureId !== capture.id || gesture.pointerId !== event.pointerId) {
      return;
    }
    if ((event.buttons & 1) === 0) {
      dragGesture.current = null;
      return;
    }
    if (gesture.started) return;

    const distance = Math.hypot(event.clientX - gesture.startX, event.clientY - gesture.startY);
    if (distance < DRAG_THRESHOLD) return;

    gesture.started = true;
    suppressCardClick.current = true;
    event.preventDefault();
    beginNativeDrag(capture);
  }

  function endDragGesture(event: ReactPointerEvent<HTMLElement>) {
    if (dragGesture.current?.pointerId === event.pointerId) {
      dragGesture.current = null;
    }
  }

  function captureCardKeyDown(
    event: ReactKeyboardEvent<HTMLElement>,
    capture: CaptureRecord,
  ) {
    if (event.target !== event.currentTarget) return;

    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "c") {
      event.preventDefault();
      void copyCapture(capture);
      return;
    }

    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      setSelectedId(capture.id);
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
            <strong>CaptureRecall</strong>
            <span>Private by default</span>
          </div>
        </div>

        <nav className="library-nav" aria-label="Screenshot library">
          <button
            className={!settingsOpen && filter === "all" ? "active" : ""}
            onClick={() => {
              setFilter("all");
              setSettingsOpen(false);
            }}
          >
            <Icon name="grid" />
            Library
            <span>{captures.length}</span>
          </button>
          <button
            className={!settingsOpen && filter === "favorites" ? "active" : ""}
            onClick={() => {
              setFilter("favorites");
              setSettingsOpen(false);
            }}
          >
            <Icon name="star" />
            Favorites
            <span>{captures.filter((capture) => capture.favorite).length}</span>
          </button>
          <button
            className={settingsOpen ? "active" : ""}
            onClick={() => {
              setDraftShortcuts(cloneShortcuts(shortcutSettingsRef.current));
              setSelectedId(null);
              setSettingsOpen(true);
            }}
          >
            <Icon name="settings" />
            Settings
          </button>
        </nav>

        <div className="privacy-card">
          <Icon name="lock" size={18} />
          <div>
            <strong>Stored locally</strong>
            <p>Your screenshots stay local by default.</p>
          </div>
        </div>

        <p className="version">Mac preview · v0.3.1</p>
      </aside>

      <main className="workspace">
        <header className="topbar">
          <div>
            <p className="eyebrow">{settingsOpen ? "Preferences" : "Screenshot library"}</p>
            <h1>
              {settingsOpen ? "Settings" : filter === "all" ? "Your captures" : "Favorites"}
            </h1>
          </div>

          {!settingsOpen && <div className="capture-actions">
            <button
              className="secondary-action"
              onClick={() => void capture("screen")}
              disabled={capturing !== null}
            >
              <Icon name="monitor" />
              {capturing === "screen" ? "Capturing…" : "Full screen"}
              {shortcutSettings.screen.enabled && (
                <kbd className="button-shortcut">{shortcutLabel(shortcutSettings.screen)}</kbd>
              )}
            </button>
            <button
              className="primary-action"
              onClick={() => void capture("area")}
              disabled={capturing !== null}
            >
              <Icon name="crop" />
              {capturing === "area" ? "Select an area…" : "Capture area"}
              {shortcutSettings.area.enabled && (
                <kbd className="button-shortcut">{shortcutLabel(shortcutSettings.area)}</kbd>
              )}
            </button>
          </div>}
        </header>

        <nav className="mobile-nav" aria-label="Screenshot library">
          <button
            className={!settingsOpen && filter === "all" ? "active" : ""}
            onClick={() => {
              setFilter("all");
              setSettingsOpen(false);
            }}
          >
            <Icon name="grid" size={17} />
            Library
          </button>
          <button
            className={!settingsOpen && filter === "favorites" ? "active" : ""}
            onClick={() => {
              setFilter("favorites");
              setSettingsOpen(false);
            }}
          >
            <Icon name="star" size={17} />
            Favorites
          </button>
          <button
            className={settingsOpen ? "active" : ""}
            onClick={() => {
              setDraftShortcuts(cloneShortcuts(shortcutSettingsRef.current));
              setSelectedId(null);
              setSettingsOpen(true);
            }}
          >
            <Icon name="settings" size={17} />
            Settings
          </button>
        </nav>

        {!settingsOpen && <section className="library-toolbar" aria-label="Library controls">
          <label className="search-box">
            <Icon name="search" size={18} />
            <input
              value={query}
              onChange={(event) => setQuery(event.currentTarget.value)}
              placeholder="Search titles, OCR text, notes, and tags"
              aria-label="Search captures"
            />
            {query && (
              <button onClick={() => setQuery("")} aria-label="Clear search">
                <Icon name="close" size={16} />
              </button>
            )}
          </label>
          <div className="library-status">
            <span className="transfer-tip">
              <Icon name="grip" size={15} />
              Copy or drag any capture into another app
            </span>
            <span className="result-count">
              {visibleCaptures.length} {visibleCaptures.length === 1 ? "capture" : "captures"}
            </span>
          </div>
        </section>}

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

        {settingsOpen ? (
          <section className="settings-page" aria-labelledby="settings-title">
            <div className="settings-intro">
              <span className="settings-icon" aria-hidden="true">
                <Icon name="settings" size={22} />
              </span>
              <div>
                <h2 id="settings-title">CaptureRecall preferences</h2>
                <p>
                  Manage where screenshots are kept and how captures are started from any app.
                </p>
              </div>
            </div>

            <h3 className="settings-section-title">Storage</h3>
            <div className="settings-card storage-setting">
              <span className="storage-setting-icon" aria-hidden="true">
                <Icon name="folder" size={20} />
              </span>
              <div className="storage-setting-copy">
                <strong>Screenshot folder</strong>
                <p>Existing images are moved when you choose a new folder.</p>
                <code title={storageLocation}>
                  {storageLocation || "Available in the desktop app"}
                </code>
              </div>
              <button
                className="secondary-action"
                type="button"
                disabled={changingStorage}
                onClick={() => void chooseStorageLocation()}
              >
                {changingStorage ? "Moving…" : "Choose folder"}
              </button>
            </div>

            <h3 className="settings-section-title">Image analysis</h3>
            <div className="settings-card analysis-setting">
              <span className="storage-setting-icon" aria-hidden="true">
                <Icon name="sparkles" size={20} />
              </span>
              <div className="storage-setting-copy">
                <strong>{analysisSettings?.ocrAvailable ? "OCR is on" : "OCR is unavailable"}</strong>
                <p>
                  {analysisSettings?.ocrAvailable
                    ? "Every new capture is scanned for searchable text on this device."
                    : analysisSettings?.ocrError ?? "Checking the bundled OCR models…"}
                </p>
                <label className="vision-toggle">
                  <input
                    type="checkbox"
                    checked={draftVision?.enabled ?? false}
                    disabled={!analysisSettings?.ocrAvailable || savingVision}
                    onChange={(event) => setDraftVision((current) => current && {
                      ...current, enabled: event.currentTarget.checked,
                    })}
                  />
                  Generate titles and descriptions with AI
                </label>
                <p>Optional. The complete image is sent to the service at this address. A tunnel can forward it to another computer. Only use a service you trust.</p>
                {analysisSettings?.visionError && <p className="analysis-error">{analysisSettings.visionError}</p>}
                <div className="vision-fields">
                  <label>
                    Service URL on this Mac
                    <input
                      type="url"
                      value={draftVision?.endpoint ?? ""}
                      disabled={!analysisSettings?.ocrAvailable || savingVision}
                      onChange={(event) => setDraftVision((current) => current && {
                        ...current, endpoint: event.currentTarget.value,
                      })}
                      placeholder="http://127.0.0.1:8083/v1/chat/completions"
                    />
                  </label>
                  <label>
                    Vision model name
                    <input
                      type="text"
                      value={draftVision?.model ?? ""}
                      disabled={!analysisSettings?.ocrAvailable || savingVision}
                      onChange={(event) => setDraftVision((current) => current && {
                        ...current, model: event.currentTarget.value,
                      })}
                    />
                  </label>
                </div>
                <button
                  className="secondary-action"
                  type="button"
                  disabled={!analysisSettings?.ocrAvailable || savingVision || !draftVision}
                  onClick={() => void saveVisionSettings()}
                >
                  {savingVision ? "Saving…" : "Save analysis settings"}
                </button>
              </div>
            </div>

            <h3 className="settings-section-title">Global shortcuts</h3>

            <div className="settings-card">
              <ShortcutEditor
                label="Capture an area"
                description="Choose a region using the secure desktop capture picker."
                value={draftShortcuts.area}
                onChange={(binding) => updateDraftShortcut("area", binding)}
              />
              <ShortcutEditor
                label="Capture full screen"
                description="Capture the full display without opening CaptureRecall first."
                value={draftShortcuts.screen}
                onChange={(binding) => updateDraftShortcut("screen", binding)}
              />
            </div>

            <div className="settings-note">
              <Icon name="lock" size={17} />
              <p>
                Shortcuts are registered only while CaptureRecall is running. Ubuntu may reserve
                some combinations for system actions.
              </p>
            </div>

            <div className="settings-actions">
              <button
                className="secondary-action"
                type="button"
                disabled={savingShortcuts}
                onClick={() => setDraftShortcuts(cloneShortcuts(DEFAULT_SHORTCUTS))}
              >
                Restore defaults
              </button>
              <button
                className="primary-action"
                type="button"
                disabled={savingShortcuts}
                onClick={() => void saveShortcutSettings()}
              >
                <Icon name="check" size={17} />
                {savingShortcuts ? "Applying…" : "Save shortcuts"}
              </button>
            </div>
          </section>
        ) : loading ? (
          <div className="loading-grid" aria-label="Loading captures">
            {Array.from({ length: 6 }).map((_, index) => (
              <div className="loading-card" key={index} />
            ))}
          </div>
        ) : visibleCaptures.length > 0 ? (
          <section className="capture-grid" aria-label="Captures">
            <p id="capture-transfer-instructions" className="sr-only">
              Press Control or Command C to copy a focused screenshot. Drag a card to copy the PNG
              into another app or folder.
            </p>
            {visibleCaptures.map((capture) => (
              <article
                className={`capture-card ${draggingId === capture.id ? "dragging" : ""}`}
                key={capture.id}
                onClick={() => {
                  if (suppressCardClick.current) {
                    suppressCardClick.current = false;
                    return;
                  }
                  setSelectedId(capture.id);
                }}
                onKeyDown={(event) => captureCardKeyDown(event, capture)}
                onPointerDown={(event) => startDragGesture(event, capture)}
                onPointerMove={(event) => continueDragGesture(event, capture)}
                onPointerUp={endDragGesture}
                onPointerCancel={endDragGesture}
                tabIndex={0}
                aria-describedby="capture-transfer-instructions"
                aria-label={`${captureLabel(capture)}, screenshot`}
              >
                <div className="capture-preview">
                  <img
                    src={imageSource(capture)}
                    alt={capture.title || capture.description || capture.note || "Screenshot"}
                    draggable={false}
                  />
                  <button
                    className="copy-button"
                    type="button"
                    disabled={copyingId === capture.id}
                    onClick={(event) => {
                      event.stopPropagation();
                      void copyCapture(capture);
                    }}
                    aria-label={`Copy ${fileName(capture.filePath)}`}
                    title="Copy image and file"
                  >
                    <Icon name={copyingId === capture.id ? "check" : "copy"} size={17} />
                  </button>
                  <button
                    className={`favorite-button ${capture.favorite ? "selected" : ""}`}
                    type="button"
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
                  {capture.enrichmentStatus === "processing" && (
                    <span className="enrichment-pill">
                      <Icon name="sparkles" size={13} />
                      Analyzing
                    </span>
                  )}
                  <span className="drag-hint" aria-hidden="true">
                    <Icon name="grip" size={14} />
                    {draggingId === capture.id ? "Dragging…" : "Drag to share"}
                  </span>
                </div>
                <div className="capture-card-copy">
                  <div>
                    <strong>{captureLabel(capture)}</strong>
                    <span>{formatCaptureDate(capture.createdAt)}</span>
                  </div>
                  <span className="dimensions">
                    {capture.width} × {capture.height}
                  </span>
                </div>
                {capture.description && (
                  <p className="capture-card-description">{capture.description}</p>
                )}
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
                ? "Search looks through titles, descriptions, OCR text, notes, tags, and filenames."
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

            <div
              className={`detail-image-shell ${draggingId === selected.id ? "dragging" : ""}`}
              onPointerDown={(event) => startDragGesture(event, selected)}
              onPointerMove={(event) => continueDragGesture(event, selected)}
              onPointerUp={endDragGesture}
              onPointerCancel={endDragGesture}
              title="Drag this PNG into another app or folder"
            >
              <img
                className="detail-image"
                src={imageSource(selected)}
                alt={selected.title || selected.description || selected.note || "Selected screenshot"}
                draggable={false}
              />
              <span className="detail-drag-hint" aria-hidden="true">
                <Icon name="grip" size={15} />
                {draggingId === selected.id ? "Dragging…" : "Drag PNG to share"}
              </span>
            </div>

            <div className="detail-transfer">
              <button
                className="primary-action"
                type="button"
                disabled={copyingId === selected.id}
                onClick={() => void copyCapture(selected)}
              >
                <Icon name={copyingId === selected.id ? "check" : "copy"} />
                {copyingId === selected.id ? "Copied" : "Copy screenshot"}
              </button>
              <p>Paste inline or as a file into email, documents, chats, cloud apps, and folders.</p>
            </div>

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

            <section className="enrichment-card" aria-label="Local screenshot analysis">
              <div className="enrichment-card-copy">
                <span className={`enrichment-icon ${selected.enrichmentStatus}`}>
                  <Icon name="sparkles" size={18} />
                </span>
                <div>
                  <strong>{enrichmentLabel(selected.enrichmentStatus)}</strong>
                  <p>
                    OCR runs on this device. Optional vision analysis only runs after you opt in;
                    re-analysis never replaces your note or edits.
                  </p>
                </div>
              </div>
              <button
                className="secondary-action compact-action"
                type="button"
                disabled={
                  selected.enrichmentStatus === "processing" || analyzingId === selected.id
                }
                onClick={() => void analyzeCapture(selected)}
              >
                <Icon name="sparkles" size={16} />
                {selected.enrichmentStatus === "complete" ||
                selected.enrichmentStatus === "no_text"
                  ? "Re-analyze"
                  : selected.enrichmentStatus === "processing"
                    ? "Analyzing…"
                    : "Analyze"}
              </button>
            </section>

            <label className="field">
              <span>Title</span>
              <input
                value={draftTitle}
                onChange={(event) => setDraftTitle(event.currentTarget.value)}
                placeholder="A clear name for this screenshot"
                maxLength={140}
              />
              <small>Optional vision suggestion after you explicitly opt in.</small>
            </label>

            <label className="field">
              <span>Description</span>
              <textarea
                value={draftDescription}
                onChange={(event) => setDraftDescription(event.currentTarget.value)}
                placeholder="What is shown in this screenshot?"
                rows={3}
              />
              <small>Optional vision suggestion based on the image’s interface and context.</small>
            </label>

            {selected.ocrText && (
              <details className="ocr-details">
                <summary>
                  <span>
                    <Icon name="text" size={16} />
                    Detected text
                  </span>
                  <span>{selected.ocrText.length.toLocaleString()} characters</span>
                </summary>
                <pre>{selected.ocrText}</pre>
              </details>
            )}

            <label className="field">
              <span>Note</span>
              <textarea
                value={draftNote}
                onChange={(event) => setDraftNote(event.currentTarget.value)}
                placeholder="Why did you save this?"
                rows={4}
              />
              <small>Your personal context. Automatic analysis never changes it.</small>
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
