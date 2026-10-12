import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
type Capture = { id: string; title: string; createdAt: string };
type HostedPage = { captures: HostedCapture[]; nextCursor: string | null };
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
    [connected, setConnected] = useState(false),
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
            In your website account, open “Connect your desktop app” and create a
            device key. Copy the Library ID and Device key into the fields below,
            then select Connect hosted library. Keys are cleared when you leave
            Settings and expire after 90 days.
          </p>
          <label>
            Library ID
            <input
              value={workspace}
              placeholder="Library ID from your website account"
              disabled={busy}
              onChange={(e) => {
                setWorkspace(e.target.value);
                setHosted([]);
                setNextCursor(null);
                setConnected(false);
                setNotice("");
                setError("");
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
              placeholder="Paste your device key here"
              disabled={busy}
              onChange={(e) => {
                setToken(e.target.value);
                setHosted([]);
                setNextCursor(null);
                setConnected(false);
                setNotice("");
                setError("");
              }}
              autoComplete="off"
              spellCheck={false}
            />
          </label>
          <div className="settings-actions">
            <button
              className="primary-action"
              disabled={busy || !workspace.trim() || !token.trim()}
              onClick={() => void action(async () => {
                setConnected(false);
                setHosted([]);
                setNextCursor(null);
                const page = await invoke<HostedPage>("cloud_list", { connection, cursor: null });
                setHosted(page.captures);
                setNextCursor(page.nextCursor);
                setConnected(true);
                setNotice("Connected to your hosted library. Choose a screenshot to back up, or restore a hosted copy below.");
              })}
            >
              {connected ? "Check connection" : "Connect hosted library"}
            </button>
          </div>
          <p role="status">{connected ? "Connected · key kept only while Settings is open" : "Enter your Library ID and Device key to connect."}</p>
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
              disabled={busy || !selected || !connected}
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
              disabled={busy || !connected}
              onClick={() =>
                void action(async () => {
                  const page = await invoke<HostedPage>("cloud_list", { connection, cursor: null });
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
                setConnected(false);
                setError("");
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
                  const page = await invoke<HostedPage>("cloud_list", { connection, cursor: nextCursor });
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
