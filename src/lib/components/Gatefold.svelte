<script lang="ts">
  import type { Album } from "$lib/types";
  import { coverUrl, formatDuration } from "$lib/library.svelte";
  import { formatClock } from "$lib/playback.svelte";

  let {
    album,
    onClose,
    flipped = $bindable(false),
  }: { album: Album; onClose: () => void; flipped?: boolean } = $props();

  const cover = $derived(coverUrl(album));
  const playlist = $derived(album.source === "playlist");
  const formatLabel = $derived(
    playlist
      ? "12″ Test Pressing"
      : album.format === "single7"
        ? "7″ Single"
        : "12″ LP",
  );
  const rpm = $derived(album.sides[0]?.rpm === "45" ? "45" : "33⅓");
  const handRpm = $derived(rpm === "33⅓" ? "33 1/3" : rpm);
  const catalog = $derived(
    playlist
      ? `TP-${album.id.slice(0, 6).toUpperCase()}`
      : (album.sides[0]?.matrix?.replace(/-[A-D]$/, "") ??
        `LP-${album.id.slice(0, 6).toUpperCase()}`),
  );
  const barcode = $derived(
    album.id
      .toUpperCase()
      .replace(/[^0-9A-F]/g, "")
      .slice(0, 12)
      .replace(/(.{4})/g, "$1 ")
      .trim(),
  );
  const totalTracks = $derived(album.sides.reduce((sum, side) => sum + side.tracks.length, 0));
  const manyTracks = $derived(totalTracks > 12);

  const bars = $derived.by(() => {
    const hex = album.id.replace(/[^0-9a-f]/gi, "").toLowerCase() || "a1b2c3";
    return Array.from({ length: 48 }, (_, index) => {
      const digit = parseInt(hex[index % hex.length], 16);
      return { weight: 1 + (digit % 3), ink: index % 2 === 0 };
    });
  });

  function sideDuration(index: number): number {
    const side = album.sides[index];
    return side.tracks.reduce((sum, track) => sum + track.duration + track.pregap, 0);
  }
</script>

<div class="scrim" role="dialog" aria-modal="true" aria-label="Plattenhülle">
  <button class="close" type="button" onclick={onClose} title="Schließen (Esc)">×</button>

  <div class="stage">
    <button
      class="sleeve"
      class:flipped
      class:many={manyTracks}
      class:test={playlist}
      type="button"
      onclick={() => (flipped = !flipped)}
      title={flipped ? "Vorderseite zeigen" : "Rückseite zeigen"}
    >
      <div class="sleeve-inner">
        <div class="face front">
          {#if playlist}
            <div class="test-front">
              <span class="test-stamp">Test Pressing</span>
              <span class="test-hand">{album.title}</span>
              <span class="test-hand small">{album.albumArtist}</span>
              <span class="test-note">{totalTracks} Tracks · {handRpm} RPM · {catalog}</span>
            </div>
          {:else if cover}
            <img src={cover} alt="" draggable="false" />
          {:else}
            <div
              class="cover-fallback"
              style:--vinyl={album.palette.vinyl}
              style:--ink={album.palette.dark}
            >
              <span class="fallback-artist">{album.albumArtist}</span>
              <span class="fallback-title">{album.title}</span>
            </div>
          {/if}
          <span class="sleeve-edge" aria-hidden="true"></span>
          <span class="front-wear" aria-hidden="true"></span>
        </div>

        <div class="face back" class:test={playlist}>
          <header class="back-head">
            <span class="back-artist">{album.albumArtist}</span>
            <h2>{album.title}</h2>
            <div class="back-rule" style:background={album.palette.accent}></div>
            <p class="back-sub">
              {formatLabel} · {rpm} RPM · Stereo
              {#if album.label && !playlist} · {album.label}{/if}
              {#if album.year} · {album.year}{/if}
            </p>
          </header>

          <div class="sides">
            {#each album.sides as side, index}
              <section class="side">
                <div class="side-head">
                  <span>Seite {side.label}</span>
                  <span>{formatDuration(sideDuration(index))}</span>
                </div>
                <ol class="tracks">
                  {#each side.tracks as track, trackIndex}
                    <li>
                      <span class="num">{side.label}{trackIndex + 1}</span>
                      <span class="ttl">{track.title}</span>
                      <span class="dots"></span>
                      <span class="dur">{formatClock(track.duration)}</span>
                    </li>
                  {/each}
                </ol>
              </section>
            {/each}
          </div>

          <footer class="back-foot">
            <div class="foot-left">
              <span class="cat">{catalog}</span>
              <span class="small">Gesamtspielzeit {formatClock(album.duration)}</span>
              <span class="small">Hergestellt mit Longplay · Direct Drive</span>
            </div>
            <div class="barcode-wrap">
              {#if playlist}
                <span class="promo">Promo · Not for Sale</span>
              {:else}
                <div class="barcode" aria-hidden="true">
                  {#each bars as bar, index}
                    <span
                      class:ink={bar.ink}
                      style:flex={bar.weight}
                      data-index={index}
                    ></span>
                  {/each}
                </div>
                <span class="barcode-text">{barcode}</span>
              {/if}
            </div>
          </footer>
        </div>
      </div>
    </button>

    <div class="bar">
      <span class="caption">{album.albumArtist} — {album.title}</span>
      <button class="turn" type="button" onclick={() => (flipped = !flipped)}>
        {flipped ? "Vorderseite" : "Rückseite"} anzeigen
      </button>
    </div>
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 60;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    background: rgba(8, 4, 2, 0.86);
    backdrop-filter: blur(5px);
    animation: scrim-in 180ms ease both;
  }

  .stage {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    max-width: 100%;
    max-height: 100%;
  }

  .sleeve {
    width: min(80vw, 80vh, 1000px);
    aspect-ratio: 1;
    cursor: pointer;
    perspective: 1800px;
    filter: drop-shadow(0 30px 60px rgba(0, 0, 0, 0.6));
    animation: sleeve-in 260ms cubic-bezier(0.2, 0.9, 0.2, 1) both;
  }

  .sleeve-inner {
    position: relative;
    width: 100%;
    height: 100%;
    transform-style: preserve-3d;
    transition: transform 720ms cubic-bezier(0.2, 0.8, 0.2, 1);
  }

  .sleeve.flipped .sleeve-inner {
    transform: rotateY(180deg);
  }

  .face {
    position: absolute;
    inset: 0;
    border-radius: 2px;
    overflow: hidden;
    backface-visibility: hidden;
    -webkit-backface-visibility: hidden;
    box-shadow:
      0 0 0 1px rgba(0, 0, 0, 0.5),
      0 0 40px rgba(0, 0, 0, 0.25) inset;
    background: #f4eeda;
  }

  .front img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .sleeve-edge {
    position: absolute;
    inset: 0;
    box-shadow:
      0 0 0 1px rgba(255, 255, 255, 0.08) inset,
      0 0 60px rgba(0, 0, 0, 0.2) inset;
    pointer-events: none;
  }

  .front-wear {
    position: absolute;
    inset: 0;
    pointer-events: none;
    background:
      radial-gradient(60% 55% at 50% 50%, transparent 58%, rgba(0, 0, 0, 0.22) 63%, transparent 72%),
      radial-gradient(circle at 0 0, rgba(255, 248, 230, 0.5), transparent 9%),
      radial-gradient(circle at 100% 0, rgba(255, 248, 230, 0.35), transparent 8%),
      radial-gradient(circle at 0 100%, rgba(255, 248, 230, 0.32), transparent 8%),
      radial-gradient(circle at 100% 100%, rgba(255, 248, 230, 0.42), transparent 9%);
    mix-blend-mode: overlay;
    opacity: 0.9;
  }

  .cover-fallback {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    gap: 1vmin;
    padding: 9%;
    background:
      linear-gradient(160deg, var(--vinyl), #0d0d0d 92%),
      #111;
    color: #f4eeda;
    font-family: var(--font-display);
  }

  .fallback-artist {
    font-size: 1.8vmin;
    font-style: italic;
    opacity: 0.8;
  }

  .fallback-title {
    font-size: 4.4vmin;
    line-height: 1.05;
  }

  .test-front {
    width: 100%;
    height: 100%;
    padding: 9%;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    justify-content: center;
    gap: 1.5vmin;
    color: #23262e;
    background-color: #efe7d2;
    background-image: url("/textures/paper-soft.jpg");
    background-size: 420px 420px;
    background-blend-mode: multiply;
  }

  .test-stamp {
    align-self: flex-end;
    margin-bottom: 4vmin;
    padding: 0.7vmin 1.1vmin;
    border: 1px solid rgba(150, 38, 38, 0.58);
    border-radius: 1px;
    color: rgba(150, 38, 38, 0.76);
    font-family: var(--font-display);
    font-size: 2.3vmin;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    transform: rotate(-10deg);
  }

  .test-hand {
    font-family: var(--font-hand);
    font-size: 7.6vmin;
    line-height: 1.02;
    transform: rotate(-1.8deg);
  }

  .test-hand.small {
    font-size: 3.4vmin;
    opacity: 0.85;
    transform: rotate(-1.2deg);
  }

  .test-note {
    margin-top: 2vmin;
    font-family: var(--font-hand);
    font-size: 2.4vmin;
    opacity: 0.62;
    transform: rotate(-1deg);
  }

  .back.test {
    background-color: #efe7d2;
    background-image: url("/textures/paper-soft.jpg");
    background-size: 420px 420px;
    background-blend-mode: multiply;
    color: #23262e;
  }

  .back.test .back-head h2,
  .back.test .back-artist,
  .back.test .ttl,
  .back.test .num,
  .back.test .dur,
  .back.test .side-head {
    font-family: var(--font-hand);
  }

  .back.test .back-rule {
    display: none;
  }

  .promo {
    padding: 0.9vmin 1.4vmin;
    border: 1px solid rgba(150, 38, 38, 0.58);
    border-radius: 1px;
    color: rgba(150, 38, 38, 0.78);
    font-family: var(--font-display);
    font-size: 1.5vmin;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    transform: rotate(-4deg);
  }

  .back {
    transform: rotateY(180deg);
    color: #1a1410;
    padding: 5.6% 6%;
    display: flex;
    flex-direction: column;
    gap: 2.2vmin;
    background:
      radial-gradient(90% 80% at 70% 8%, rgba(255, 255, 255, 0.4), transparent 62%),
      linear-gradient(180deg, #efe6cf, #e2d6b8);
  }

  .back::after {
    content: "";
    position: absolute;
    inset: 0;
    pointer-events: none;
    opacity: 0.08;
    mix-blend-mode: multiply;
    background-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='140' height='140'><filter id='n'><feTurbulence type='fractalNoise' baseFrequency='0.85' numOctaves='3' stitchTiles='stitch'/><feColorMatrix type='saturate' values='0'/></filter><rect width='140' height='140' filter='url(%23n)' opacity='0.6'/></svg>");
    background-size: 140px 140px;
  }

  .back-head h2 {
    margin: 0.6vmin 0 0;
    font-family: var(--font-display);
    font-size: 5.6vmin;
    font-weight: 400;
    line-height: 1.02;
  }

  .back-artist {
    font-size: 1.5vmin;
    letter-spacing: 0.22em;
    text-transform: uppercase;
    opacity: 0.6;
  }

  .back-rule {
    height: 2px;
    margin: 1.8vmin 0 1.4vmin;
    opacity: 0.75;
  }

  .back-sub {
    margin: 0;
    font-size: 1.3vmin;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    opacity: 0.5;
  }

  .sides {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 4vmin;
    align-content: center;
    overflow: hidden;
  }

  .side {
    min-width: 0;
  }

  .side-head {
    display: flex;
    justify-content: space-between;
    padding-bottom: 1vmin;
    border-bottom: 1px solid rgba(26, 20, 16, 0.22);
    font-size: 1.35vmin;
    letter-spacing: 0.2em;
    text-transform: uppercase;
    opacity: 0.65;
  }

  .tracks {
    list-style: none;
    margin: 1.4vmin 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.8vmin;
  }

  .tracks li {
    display: flex;
    align-items: baseline;
    gap: 0.8vmin;
    font-size: 1.95vmin;
    line-height: 1.28;
  }

  .many .tracks li {
    font-size: 1.5vmin;
  }

  .num {
    width: 4vmin;
    flex: 0 0 4vmin;
    text-align: right;
    opacity: 0.55;
    font-variant-numeric: tabular-nums;
  }

  .ttl {
    flex: 1 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .dots {
    flex: 0 0 7%;
    border-bottom: 1px dotted rgba(26, 20, 16, 0.35);
    transform: translateY(-0.4vmin);
  }

  .dur {
    flex: 0 0 auto;
    font-variant-numeric: tabular-nums;
    opacity: 0.75;
  }

  .back-foot {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 2vmin;
    padding-top: 1.4vmin;
    border-top: 1px solid rgba(26, 20, 16, 0.2);
  }

  .foot-left {
    display: flex;
    flex-direction: column;
    gap: 0.5vmin;
  }

  .cat {
    font-size: 1.2vmin;
    letter-spacing: 0.2em;
  }

  .small {
    font-size: 1vmin;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    opacity: 0.5;
  }

  .barcode-wrap {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 0.6vmin;
  }

  .barcode {
    display: flex;
    width: 20vmin;
    height: 5.5vmin;
    background: #efe6cf;
  }

  .barcode span {
    background: transparent;
  }

  .barcode span.ink {
    background: #1a1410;
  }

  .barcode-text {
    font-size: 1vmin;
    letter-spacing: 0.18em;
    font-variant-numeric: tabular-nums;
    opacity: 0.6;
  }

  .bar {
    display: flex;
    align-items: center;
    gap: 18px;
    color: rgba(239, 230, 207, 0.65);
    font-size: 10px;
    letter-spacing: 0.08em;
  }

  .caption {
    max-width: 60vw;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-display);
  }

  .turn {
    padding: 6px 16px;
    border-radius: 3px;
    font-size: 8.5px;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: #2a1a0e;
    background: linear-gradient(180deg, #d9b64a, #b8912a 55%, #8f6d17);
    box-shadow:
      0 1px 0 rgba(255, 250, 220, 0.6) inset,
      0 2px 6px rgba(0, 0, 0, 0.5);
  }

  .close {
    position: fixed;
    top: 22px;
    right: 26px;
    width: 34px;
    height: 34px;
    border-radius: 50%;
    font-size: 20px;
    line-height: 1;
    color: #2a1a0e;
    background: radial-gradient(circle at 35% 28%, #f0d88f, #c9a227 55%, #7d5f14 100%);
    box-shadow:
      0 1px 0 rgba(255, 240, 190, 0.55) inset,
      0 2px 8px rgba(0, 0, 0, 0.55);
  }

  @keyframes scrim-in {
    from {
      opacity: 0;
    }
  }

  @keyframes sleeve-in {
    from {
      opacity: 0;
      transform: translateY(12px) scale(0.97);
    }
  }
</style>
