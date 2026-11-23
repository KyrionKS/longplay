export type Rpm = "33" | "45";
export type DiscFormat = "lp12" | "single7" | "unknown";
export type AlbumSource = "release" | "playlist";
export type SortKey = "added" | "artist" | "album" | "year";

export interface Palette {
  vinyl: string;
  vinylLight: string;
  label: string;
  accent: string;
  dark: string;
}

export interface Track {
  id: string;
  number: number;
  title: string;
  artist: string;
  duration: number;
  pregap: number;
  path: string;
  offset: number;
  side: string;
}

export interface Side {
  label: string;
  rpm: Rpm;
  matrix: string | null;
  tracks: Track[];
}

export interface Album {
  id: string;
  title: string;
  albumArtist: string;
  year: number | null;
  genre: string | null;
  label: string | null;
  addedAt: number;
  cover: string | null;
  palette: Palette;
  format: DiscFormat;
  source: AlbumSource;
  sides: Side[];
  folder: string;
  trackCount: number;
  duration: number;
}

export interface ScanSummary {
  albums: number;
  tracks: number;
  issues: string[];
  elapsedMs: number;
}

export type GrooveKind = "leadIn" | "gap" | "track" | "runOut";

export interface GrooveBand {
  kind: GrooveKind;
  trackIndex: number | null;
  start: number;
  end: number;
  startNorm: number;
  endNorm: number;
  startTime: number;
  endTime: number;
}

export interface PlaybackState {
  loaded: boolean;
  loading: boolean;
  playing: boolean;
  position: number;
  duration: number;
  side: string;
  sideIndex: number;
  albumId: string;
  rpm: string;
  grooves: GrooveBand[];
  recordRadius: number;
  armAngle: number;
  stylusRadius: number;
  pivotDistance: number;
  armLength: number;
  trackIndex: number;
  trackNumber: number;
  trackTitle: string;
  issues: string[];
  error: string | null;
}
