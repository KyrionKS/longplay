<script lang="ts">
  import type { Album } from "$lib/types";
  import { coverUrl } from "$lib/library.svelte";

  let { album, onselect }: { album: Album; onselect: () => void } = $props();

  const cover = $derived(coverUrl(album));
  const testPressing = $derived(album.source === "playlist");
  const trackCount = $derived(album.sides.reduce((sum, side) => sum + side.tracks.length, 0));
</script>

<button
  class="sleeve"
  class:test={testPressing}
  type="button"
  onclick={onselect}
  title={`${album.title} — ${album.albumArtist}`}
  data-album-id={album.id}
  style:--vinyl={album.palette.vinyl}
  style:--light={album.palette.vinylLight}
  style:--label={album.palette.label}
>
  <span class="disc" aria-hidden="true"></span>
  <span class="paper-edge" aria-hidden="true"></span>

  <span class="art">
    {#if testPressing}
      {#if cover}
        <img src={cover} alt="" draggable="false" />
        <span class="tp-badge">Test Pressing</span>
      {:else}
        <span class="test-cover">
          <span class="tp-stamp">Test Pressing</span>
          <span class="tp-hand">{album.title}</span>
          <span class="tp-meta">{album.albumArtist}</span>
          <span class="tp-meta dim">{trackCount} Tracks · 33 1/3</span>
          <span class="tp-line" aria-hidden="true"></span>
        </span>
      {/if}
    {:else if cover}
      <img src={cover} alt="" draggable="false" />
    {:else}
      <span class="fallback" style:color={album.palette.label}>
        <span class="fallback-title">{album.title}</span>
        <span class="fallback-artist">{album.albumArtist}</span>
      </span>
    {/if}
  </span>

  <span class="wear" aria-hidden="true"></span>
  <span class="spine" aria-hidden="true"></span>
  <span class="grain" aria-hidden="true"></span>
  <span class="shine" aria-hidden="true"></span>
  <span class="caption">{album.title}</span>
</button>

<style>
  .sleeve {
    position: relative;
    aspect-ratio: 1;
    border-radius: 2px;
    text-align: left;
    isolation: isolate;
    transition:
      transform 180ms cubic-bezier(0.2, 0.8, 0.2, 1),
      filter 180ms ease;
    filter: drop-shadow(0 10px 14px rgba(0, 0, 0, 0.45));
  }

  .sleeve:hover {
    transform: translateY(-4px);
    filter: drop-shadow(0 16px 22px rgba(0, 0, 0, 0.55));
  }

  .sleeve:focus-visible {
    outline: 2px solid var(--brass-light);
    outline-offset: 4px;
  }

  .disc {
    position: absolute;
    left: 4%;
    right: 4%;
    top: -7%;
    aspect-ratio: 1;
    border-radius: 50%;
    background:
      repeating-radial-gradient(
        circle at 50% 50%,
        rgba(255, 255, 255, 0.055) 0 1px,
        rgba(0, 0, 0, 0.08) 1px 3px,
        transparent 3px 5px
      ),
      radial-gradient(circle at 38% 30%, var(--light), var(--vinyl) 52%, #141414 98%);
    box-shadow:
      0 2px 6px rgba(0, 0, 0, 0.6),
      inset 0 0 0 1px rgba(255, 255, 255, 0.08);
  }

  .paper-edge {
    position: absolute;
    inset: 1% -1.6% -1.6% 1%;
    border-radius: 1px;
    background-color: #e6dcc2;
    background-image: url("/textures/paper-soft.jpg");
    background-size: 240px 240px;
    box-shadow: 2px 3px 10px rgba(0, 0, 0, 0.35);
    z-index: 0;
  }

  .art {
    position: absolute;
    inset: 0;
    overflow: hidden;
    z-index: 1;
    background: #1c120a;
    box-shadow:
      0 0 0 1px rgba(0, 0, 0, 0.65),
      0 1px 0 rgba(255, 255, 255, 0.18) inset,
      0 0 22px rgba(0, 0, 0, 0.35) inset;
  }

  .art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .fallback {
    position: absolute;
    inset: 0;
    padding: 12% 10%;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    gap: 6px;
    background:
      linear-gradient(160deg, var(--vinyl), #0d0d0d 90%),
      #111;
    font-family: var(--font-display);
  }

  .fallback-title {
    font-size: 15px;
    line-height: 1.15;
  }

  .fallback-artist {
    font-size: 11px;
    font-style: italic;
    opacity: 0.8;
  }

  .test-cover {
    position: absolute;
    inset: 0;
    padding: 9% 9% 10%;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    justify-content: center;
    gap: 1.5%;
    color: #23262e;
    background-color: #efe7d2;
    background-image: url("/textures/paper-soft.jpg");
    background-size: 220px 220px;
    background-blend-mode: multiply;
    box-shadow: 0 0 0 1px rgba(80, 62, 36, 0.18) inset;
  }

  .tp-stamp {
    align-self: flex-end;
    margin-bottom: 4%;
    padding: 0.35em 0.5em;
    border: 1px solid rgba(150, 38, 38, 0.55);
    border-radius: 1px;
    color: rgba(150, 38, 38, 0.72);
    font-family: var(--font-display);
    font-size: 7px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    transform: rotate(-9deg);
  }

  .tp-hand {
    font-family: var(--font-hand);
    font-size: 17px;
    line-height: 1.02;
    transform: rotate(-1.6deg);
  }

  .tp-meta {
    font-family: var(--font-hand);
    font-size: 10px;
    opacity: 0.82;
    transform: rotate(-1.2deg);
  }

  .tp-meta.dim {
    opacity: 0.6;
  }

  .tp-line {
    width: 62%;
    height: 1px;
    margin-top: 5%;
    background: rgba(40, 42, 48, 0.28);
    transform: rotate(-1.5deg);
  }

  .tp-badge {
    position: absolute;
    top: 7%;
    right: 6%;
    padding: 0.45em 0.6em;
    border: 1px solid rgba(160, 40, 40, 0.62);
    border-radius: 1px;
    color: rgba(160, 40, 40, 0.8);
    background: rgba(244, 238, 218, 0.72);
    font-family: var(--font-display);
    font-size: 7px;
    letter-spacing: 0.14em;
    line-height: 1;
    text-transform: uppercase;
    transform: rotate(-7deg);
    white-space: nowrap;
  }

  .wear {
    position: absolute;
    inset: 0;
    z-index: 2;
    pointer-events: none;
    background:
      radial-gradient(60% 55% at 50% 50%, transparent 58%, rgba(0, 0, 0, 0.27) 63%, transparent 72%),
      radial-gradient(45% 40% at 50% 50%, transparent 42%, rgba(255, 255, 255, 0.12) 47%, transparent 56%),
      radial-gradient(circle at 0 0, rgba(255, 248, 230, 0.55), transparent 13%),
      radial-gradient(circle at 100% 0, rgba(255, 248, 230, 0.35), transparent 11%),
      radial-gradient(circle at 0 100%, rgba(255, 248, 230, 0.3), transparent 10%),
      radial-gradient(circle at 100% 100%, rgba(255, 248, 230, 0.42), transparent 12%),
      radial-gradient(circle at 50% 50%, transparent 46%, rgba(255, 255, 255, 0.05) 49%, transparent 53%);
    mix-blend-mode: overlay;
    opacity: 0.85;
  }

  .spine {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 4%;
    z-index: 2;
    background: linear-gradient(
      90deg,
      rgba(0, 0, 0, 0.38),
      rgba(255, 255, 255, 0.05) 42%,
      transparent 100%
    );
    box-shadow: inset -1px 0 0 rgba(0, 0, 0, 0.18);
    pointer-events: none;
  }

  .grain {
    position: absolute;
    inset: 0;
    z-index: 3;
    pointer-events: none;
    opacity: 0.1;
    mix-blend-mode: multiply;
    background-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='120' height='120'><filter id='n'><feTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='3' stitchTiles='stitch'/><feColorMatrix type='saturate' values='0'/></filter><rect width='120' height='120' filter='url(%23n)' opacity='0.55'/></svg>");
    background-size: 120px 120px;
  }

  .shine {
    position: absolute;
    inset: 0;
    z-index: 4;
    pointer-events: none;
    background: linear-gradient(
      118deg,
      rgba(255, 255, 255, 0.14) 0%,
      rgba(255, 255, 255, 0.03) 18%,
      transparent 34%
    );
    mix-blend-mode: screen;
  }

  .caption {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 5;
    padding: 22px 8px 7px;
    font-size: 9px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: rgba(239, 230, 207, 0.92);
    background: linear-gradient(180deg, transparent, rgba(0, 0, 0, 0.78));
    opacity: 0;
    transform: translateY(4px);
    transition:
      opacity 160ms ease,
      transform 160ms ease;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .sleeve:hover .caption,
  .sleeve:focus-visible .caption {
    opacity: 1;
    transform: translateY(0);
  }
</style>
