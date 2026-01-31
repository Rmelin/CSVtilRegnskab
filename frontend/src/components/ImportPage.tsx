import { useRef, useState, type ChangeEvent } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { readFile } from "@tauri-apps/plugin-fs";
import { isTauri } from "@tauri-apps/api/core";
import { importCsv } from "../api/tauri";

type ImportPageProps = {
  activeYear?: number;
};

export default function ImportPage({ activeYear }: ImportPageProps) {
  const [path, setPath] = useState<string | null>(null);
  const [preview, setPreview] = useState<string[][]>([]);
  const [status, setStatus] = useState<string>("");
  const fileInputRef = useRef<HTMLInputElement | null>(null);

  const decodeBuffer = (buffer: ArrayBuffer) => {
    const utf8 = new TextDecoder("utf-8").decode(buffer);
    if (utf8.includes("\ufffd")) {
      return new TextDecoder("iso-8859-1").decode(buffer);
    }
    return utf8;
  };

  const parsePreview = (text: string) => {
    const lines = text.split(/\r?\n/).filter(Boolean).slice(0, 21);
    const rows = lines.map((line) =>
      line
        .split(";")
        .map((cell) => cell.replace(/^"|"$/g, "").trim())
    );
    setPreview(rows);
  };

  const pickFile = async () => {
    if (!isTauri()) {
      fileInputRef.current?.click();
      return;
    }
    try {
      const selection = await open({
        filters: [{ name: "CSV", extensions: ["csv"] }]
      });
      if (typeof selection === "string") {
        setPath(selection);
        const data = await readFile(selection);
        parsePreview(decodeBuffer(new Uint8Array(data).buffer));
      }
    } catch (error) {
      setStatus(`Kunne ikke aabne fil: ${String(error)}`);
    }
  };

  const onFileChange = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;
    const buffer = await file.arrayBuffer();
    const text = decodeBuffer(buffer);
    setPath(file.name);
    parsePreview(text);
  };

  const onImport = async () => {
    if (!path) return;
    if (!isTauri()) {
      setStatus("Import kan kun koeres i desktop-appen.");
      return;
    }
    try {
      setStatus("Importer...");
      const summary = await importCsv(path);
      setStatus(`Importeret ${summary.imported} nye, ${summary.duplicates} dubletter.`);
    } catch (error) {
      setStatus(`Import fejlede: ${String(error)}`);
    }
  };

  const header = preview[0] ?? [];
  const body = preview.slice(1);

  return (
    <section className="panel">
      <header className="panel-header">
        <div>
          <h2>Import fra bank</h2>
          <p>Vælg CSV-fil, se preview og importér transaktioner.</p>
          {activeYear ? <p>Arbejdsår: {activeYear}</p> : null}
        </div>
        <div className="button-row">
          <button type="button" className="primary" onClick={pickFile}>
            Vælg CSV
          </button>
          <button type="button" className="secondary" disabled={!path} onClick={onImport}>
            Importér
          </button>
        </div>
      </header>
      <div className="panel-body">
        <div className="meta-row">
          <span className="label">Valgt fil</span>
          <span>{path ?? "Ingen fil valgt"}</span>
        </div>
        <input
          ref={fileInputRef}
          type="file"
          accept=".csv"
          onChange={onFileChange}
          hidden
        />
        {status ? <p className="status">{status}</p> : null}
        {preview.length > 0 ? (
          <div className="table-wrap">
            <table>
              <thead>
                <tr>
                  {header.map((cell, index) => (
                    <th key={`${cell}-${index}`}>{cell || "(tom)"}</th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {body.map((row, index) => (
                  <tr key={index}>
                    {row.map((cell, cellIndex) => (
                      <td key={cellIndex}>{cell}</td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <p className="empty">Ingen preview endnu. Vælg en CSV-fil.</p>
        )}
      </div>
    </section>
  );
}
