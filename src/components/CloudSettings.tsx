import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
type Capture = { id: string; title: string; createdAt: string };
type HostedCapture = {
  id: string;
  metadata: { title: string; createdAt: string };
};
export function CloudSettings<T extends Capture>({
  captures,
  onRestore,
}: {
  captures: T[];
  onRestore: (capture: T) => void;
}) {
  const [workspace, setWorkspace] = useState(""),
    [token, setToken] = useState(""),
    [selected, setSelected] = useState(""),
    [hosted, setHosted] = useState<HostedCapture[]>([]),
    [nextCursor, setNextCursor] = useState<string | null>(null),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [notice, setNotice] = useState("");
  const connection = { workspace: workspace.trim(), token: token.trim() };
  async function action(work: () => Promise<void>) {
    setBusy(true);
    setError("");
    setNotice("");
    try {
      await work();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <>
      <h3 className="settings-section-title">Optional hosted vault</h3>
      <div className="settings-card cloud-setting">
        <div>
          <strong>Back up only what you choose</strong>
          <p>
            The free local library needs no account. Personal and Team add paid
            hosted storage at capturerecall.com. A backup sends the selected
            image, OCR text, notes and tags to your library. Nothing uploads
            automatically.
          </p>
          <p>
            Open <code>https://capturerecall.com/account/</code> in your browser
            to subscribe and create a device key. Keys stay in memory for while
            this Settings screen is open and expire after 90 days.
          </p>
          <label>
            Library ID
            <input
              value={workspace}
              onChange={(e) => {
                setWorkspace(e.target.value);
                setHosted([]);
                setNextCursor(null);
              }}
              autoComplete="off"
              spellCheck={false}
            />
          </label>
          <label>
            Device key
            <input
              type="password"
              value={token}
              onChange={(e) => {
                setToken(e.target.value);
                setHosted([]);
                setNextCursor(null);
              }}
              autoComplete="off"
              spellCheck={false}
            />
          </label>
          <label>
            Screenshot to back up
            <select
              value={selected}
              onChange={(e) => setSelected(e.target.value)}
            >
              <option value="">Choose one screenshot</option>
              {captures.map((c) => (
                <option value={c.id} key={c.id}>
                  {c.title || c.createdAt}
                </option>
              ))}
            </select>
          </label>
          <div className="settings-actions">
            <button
              className="primary-action"
              disabled={busy || !selected || !workspace || !token}
              onClick={() =>
                void action(async () => {
                  await invoke("cloud_upload_capture", {
                    connection,
                    id: selected,
                  });
                  setNotice(
                    "Selected screenshot and metadata backed up. Repeating this action creates another hosted copy.",
                  );
                })
              }
            >
              Back up selected screenshot
            </button>
            <button
              className="secondary-action"
              disabled={busy || !workspace || !token}
              onClick={() =>
                void action(async () => {
                  const page = await invoke<{
                    captures: HostedCapture[];
                    nextCursor: string | null;
                  }>("cloud_list", { connection, cursor: null });
                  setHosted(page.captures);
                  setNextCursor(page.nextCursor);
                  setNotice(
                    "Hosted library loaded. Restoring creates a new local copy and preserves its notes, tags and OCR text.",
                  );
                })
              }
            >
              Browse hosted library
            </button>
            <button
              className="secondary-action"
              disabled={busy || !token}
              onClick={() => {
                setToken("");
                setHosted([]);
                setNextCursor(null);
                setNotice(
                  "Disconnected from this app. Revoke the device key on the website to invalidate it everywhere.",
                );
              }}
            >
              Disconnect
            </button>
          </div>
          {busy && <p role="status">Contacting your hosted vault…</p>}
          {error && (
            <p role="alert" className="cloud-error">
              {error}
            </p>
          )}
          {notice && <p role="status">{notice}</p>}
          {nextCursor && (
            <button
              className="secondary-action"
              disabled={busy}
              onClick={() =>
                void action(async () => {
                  const page = await invoke<{
                    captures: HostedCapture[];
                    nextCursor: string | null;
                  }>("cloud_list", { connection, cursor: nextCursor });
                  setHosted((current) => [...current, ...page.captures]);
                  setNextCursor(page.nextCursor);
                })
              }
            >
              Load older hosted screenshots
            </button>
          )}
          {hosted.map((c) => (
            <div className="cloud-item" key={c.id}>
              <span>
                {c.metadata.title || "Screenshot"} · {c.metadata.createdAt}
              </span>
              <button
                className="secondary-action"
                disabled={busy}
                onClick={() =>
                  void action(async () => {
                    const restored = await invoke<T>("cloud_restore_capture", {
                      connection,
                      id: c.id,
                    });
                    onRestore(restored);
                    setNotice(
                      "Screenshot restored as a new local copy. The hosted original is unchanged.",
                    );
                  })
                }
              >
                Restore local copy
              </button>
            </div>
          ))}
        </div>
      </div>
    </>
  );
}
