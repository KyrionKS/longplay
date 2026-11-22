<script lang="ts">
  import type { Album } from "$lib/types";
  import { coverUrl } from "$lib/library.svelte";

  let { album, onselect }: { album: Album; onselect: () => void } = $props();

  const cover = $derived(coverUrl(album));
</script>

<button
  class="sleeve"
  type="button"
  onclick={onselect}
  title={`${album.title} — ${album.albumArtist}`}
  style:--vinyl={album.palette.vinyl}
  style:--light={album.palette.vinylLight}
  style:--label={album.palette.label}
>
  <span class="disc" aria-hidden="true"></span>

  <span class="art">
    {#if cover}
      <img src={cover} alt="" draggable="false" />
    {:else}
      <span class="fallback" style:color={album.palette.label}>
        <span class="fallback-title">{album.title}</span>
        <span class="fallback-artist">{album.albumArtist}</span>
      </span>
    {/if}
  </span>

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

  .art {
    position: absolute;
    inset: 0;
    overflow: hidden;
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

  .shine {
    position: absolute;
    inset: 0;
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
