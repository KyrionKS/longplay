<script lang="ts">
  import type { Album, Side } from "$lib/types";
  import { formatDuration } from "$lib/library.svelte";

  let { album, side }: { album: Album; side: Side } = $props();

  const minutes = $derived(Math.round(album.duration / 60));
  const rpm = $derived(side.rpm === "45" ? "45" : "33⅓");
</script>

<div
  class="vinyl"
  style:--vinyl={album.palette.vinyl}
  style:--light={album.palette.vinylLight}
  style:--label={album.palette.label}
  style:--ink={album.palette.dark}
>
  <div class="grooves" aria-hidden="true"></div>
  <div class="sheen" aria-hidden="true"></div>
  <div class="edge" aria-hidden="true"></div>

  <div class="label">
    <div class="top">
      <span class="cat">{side.matrix ?? "LONGPLAY"}</span>
      <span class="title">{album.title}</span>
    </div>
    <div class="bottom">
      <span class="artist">{album.albumArtist}</span>
      <span class="meta">SEITE {side.label} · {rpm} RPM</span>
      <span class="meta dim">
        {side.tracks.length} TRACKS · {minutes > 0 ? `${minutes} MIN` : formatDuration(album.duration)}
      </span>
    </div>
  </div>

  <div class="spindle" aria-hidden="true"></div>
</div>

<style>
  .vinyl {
    position: absolute;
    inset: 6%;
    border-radius: 50%;
    background:
      radial-gradient(circle at 36% 28%, rgba(255, 255, 255, 0.14), transparent 32%),
      radial-gradient(circle at 68% 76%, rgba(0, 0, 0, 0.4), transparent 55%),
      radial-gradient(
        circle at 50% 50%,
        color-mix(in srgb, var(--light) 60%, var(--vinyl)) 0%,
        var(--vinyl) 52%,
        #0a0a0a 82%,
        #191919 100%
      );
    box-shadow:
      0 0 0 1px rgba(0, 0, 0, 0.65),
      0 18px 34px rgba(0, 0, 0, 0.55),
      0 0 0 3px rgba(255, 255, 255, 0.04) inset;
    animation: settle 520ms cubic-bezier(0.2, 0.9, 0.2, 1) both;
  }

  .grooves {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background:
      repeating-radial-gradient(
        circle at 50% 50%,
        rgba(255, 255, 255, 0.045) 0 0.6px,
        rgba(0, 0, 0, 0.10) 0.6px 2.4px,
        transparent 2.4px 3.6px
      );
    mask: radial-gradient(circle at 50% 50%, transparent 0 34%, #000 35.5% 100%);
    opacity: 0.75;
  }

  .sheen {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background:
      radial-gradient(46% 24% at 30% 22%, rgba(255, 255, 255, 0.13), transparent 70%),
      radial-gradient(40% 18% at 74% 82%, rgba(255, 255, 255, 0.05), transparent 70%);
    mix-blend-mode: screen;
    pointer-events: none;
  }

  .edge {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    box-shadow:
      0 0 0 1px rgba(255, 255, 255, 0.10) inset,
      0 0 0 3px rgba(0, 0, 0, 0.35) inset;
    pointer-events: none;
  }

  .label {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 36%;
    aspect-ratio: 1;
    transform: translate(-50%, -50%);
    border-radius: 50%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-start;
    padding: 11% 12%;
    text-align: center;
    color: var(--ink);
    background:
      radial-gradient(circle at 38% 30%, rgba(255, 255, 255, 0.85), transparent 55%),
      radial-gradient(circle at 50% 50%, var(--label), color-mix(in srgb, var(--label) 78%, #b9a97f) 100%);
    box-shadow:
      0 2px 8px rgba(0, 0, 0, 0.5),
      0 0 0 1px rgba(0, 0, 0, 0.35) inset,
      0 0 0 4px rgba(0, 0, 0, 0.12) inset;
  }

  .top,
  .bottom {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1px;
    width: 100%;
  }

  .bottom {
    position: absolute;
    left: 14%;
    right: 14%;
    bottom: 13%;
  }

  .label::before {
    content: "";
    position: absolute;
    inset: 6%;
    border-radius: 50%;
    border: 1px solid rgba(0, 0, 0, 0.18);
  }

  .cat {
    font-family: var(--font-display);
    font-size: 5.5px;
    letter-spacing: 0.18em;
    opacity: 0.62;
    white-space: nowrap;
  }

  .title {
    font-family: var(--font-display);
    font-size: 11px;
    line-height: 1.08;
    letter-spacing: 0.01em;
    max-width: 100%;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .artist {
    font-family: var(--font-display);
    font-size: 7.5px;
    font-style: italic;
    opacity: 0.82;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .meta {
    font-size: 5.5px;
    letter-spacing: 0.15em;
    text-transform: uppercase;
    opacity: 0.75;
    white-space: nowrap;
  }

  .meta.dim {
    opacity: 0.55;
  }

  .spindle {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 3.4%;
    aspect-ratio: 1;
    border-radius: 50%;
    transform: translate(-50%, -50%);
    background: radial-gradient(circle at 35% 30%, #fdfdfd, #9a9a9a 55%, #3a3a3a);
    box-shadow:
      0 1px 3px rgba(0, 0, 0, 0.75),
      0 0 0 1px rgba(0, 0, 0, 0.45);
  }

  @keyframes settle {
    from {
      opacity: 0;
      transform: scale(0.965);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }
</style>
