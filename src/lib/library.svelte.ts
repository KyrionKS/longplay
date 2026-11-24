import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { Album, ScanSummary, SortKey } from "./types";

class LibraryState {
  albums = $state<Album[]>([]);
  root = $state<string | null>(null);
  loading = $state(true);
  scanning = $state(false);
  error = $state<string | null>(null);
  sort = $state<SortKey>("added");
  query = $state("");
  page = $state(0);
}

export const library = new LibraryState();

export async function loadLibrary(): Promise<void> {
  try {
    const [albums, root] = await Promise.all([
      invoke<Album[]>("library_load"),
      invoke<string | null>("library_root"),
    ]);
    library.albums = albums;
    library.root = root;
    library.error = null;
  } catch (error) {
    library.error = String(error);
  } finally {
    library.loading = false;
  }
}

export async function chooseMusicFolder(): Promise<void> {
  try {
    const picked = await open({
      directory: true,
      multiple: false,
      title: "Mediathek wählen",
    });
    if (!picked || Array.isArray(picked)) return;

    await scan(picked);
  } catch (error) {
    library.error = `Ordnerauswahl fehlgeschlagen: ${String(error)}`;
  }
}

export async function rescan(): Promise<void> {
  if (!library.root) return;
  await scan(library.root);
}

async function scan(folder: string): Promise<void> {
  library.scanning = true;
  library.error = null;
  try {
    await invoke<ScanSummary>("library_scan", { folder });
    library.root = folder;
    await loadLibrary();
  } catch (error) {
    library.error = String(error);
  } finally {
    library.scanning = false;
  }
}

export function coverUrl(album: Album): string | null {
  return album.cover ? convertFileSrc(album.cover) : null;
}

export function formatDuration(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds <= 0) return "–";
  const total = Math.round(seconds);
  const minutes = Math.floor(total / 60);
  return `${minutes}:${String(total % 60).padStart(2, "0")}`;
}
