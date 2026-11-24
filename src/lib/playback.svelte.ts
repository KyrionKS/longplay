import { invoke } from "@tauri-apps/api/core";
import type { PlaybackState } from "./types";

class PlaybackStore {
  state = $state<PlaybackState | null>(null);
  error = $state<string | null>(null);
}

export const playback = new PlaybackStore();

export async function loadSide(
  albumId: string,
  sideIndex: number,
  autoplay = false,
): Promise<void> {
  try {
    playback.state = await invoke<PlaybackState>("playback_load", {
      albumId,
      sideIndex,
      autoplay,
    });
    playback.error = playback.state.error;
  } catch (error) {
    playback.error = `Platte konnte nicht geladen werden: ${String(error)}`;
  }
}

export async function refreshPlayback(): Promise<void> {
  try {
    playback.state = await invoke<PlaybackState>("playback_state");
    playback.error = playback.state.error;
  } catch {
    // The backend is not ready yet; keep the last known state.
  }
}

export async function togglePlayback(): Promise<void> {
  try {
    playback.state = await invoke<PlaybackState>("playback_toggle");
  } catch (error) {
    playback.error = String(error);
  }
}

export async function pausePlayback(): Promise<void> {
  playback.state = await invoke<PlaybackState>("playback_pause");
}

export async function playPlayback(): Promise<void> {
  try {
    playback.state = await invoke<PlaybackState>("playback_play");
  } catch (error) {
    playback.error = String(error);
  }
}

export async function stopPlayback(): Promise<void> {
  playback.state = await invoke<PlaybackState>("playback_stop");
}

export async function seekPlayback(seconds: number): Promise<void> {
  playback.state = await invoke<PlaybackState>("playback_seek", { seconds });
}

export async function cuePlayback(seconds: number): Promise<void> {
  try {
    playback.state = await invoke<PlaybackState>("playback_cue", { seconds });
  } catch (error) {
    playback.error = String(error);
  }
}

export async function setRpm(rpm: number): Promise<void> {
  try {
    playback.state = await invoke<PlaybackState>("playback_set_rpm", { rpm });
  } catch (error) {
    playback.error = String(error);
  }
}

export function formatClock(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) return "0:00";
  const total = Math.floor(seconds);
  return `${Math.floor(total / 60)}:${String(total % 60).padStart(2, "0")}`;
}
