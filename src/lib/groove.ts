import type { GrooveBand } from "./types";

/// Angle at the pivot between pivot→spindle and pivot→stylus.
export function thetaForRadius(
  radiusMm: number,
  pivotDistance = 211,
  armLength = 230,
): number {
  const cosine =
    (pivotDistance * pivotDistance + armLength * armLength - radiusMm * radiusMm) /
    (2 * pivotDistance * armLength);
  return Math.acos(Math.min(1, Math.max(-1, cosine)));
}

export interface CuePoint {
  time: number;
  trackIndex: number | null;
  radius: number;
}

/// Maps a stylus radius to side time using the groove bands.
export function cuePointForRadius(
  bands: GrooveBand[],
  radiusMm: number,
): CuePoint | null {
  if (bands.length === 0) return null;
  const outer = bands[0].start;
  const inner = bands[bands.length - 1].end;
  if (radiusMm > outer + 1e-6 || radiusMm < inner - 1e-6) return null;

  for (const band of bands) {
    if (radiusMm <= band.start + 1e-9 && radiusMm >= band.end - 1e-9) {
      const span = band.start - band.end;
      const progress = span > 1e-9 ? (band.start - radiusMm) / span : 0;
      return {
        time: band.startTime + (band.endTime - band.startTime) * progress,
        trackIndex: band.trackIndex,
        radius: radiusMm,
      };
    }
  }

  return {
    time: bands[bands.length - 1].endTime,
    trackIndex: null,
    radius: radiusMm,
  };
}
