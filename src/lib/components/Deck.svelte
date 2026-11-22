<script lang="ts">
  import type { Album, GrooveBand, PlaybackState, Side } from "$lib/types";
  import { formatClock } from "$lib/playback.svelte";
  import Vinyl from "./Vinyl.svelte";

  interface Props {
    album: Album | null;
    side: Side | null;
    sideIndex: number;
    playback: PlaybackState | null;
    grooves: GrooveBand[];
    onSide: (index: number) => void;
    onToggle: () => void;
    onRpm: (rpm: number) => void;
  }

  let { album, side, sideIndex, playback, grooves, onSide, onToggle, onRpm }: Props = $props();

  const playing = $derived(!!playback?.playing && !playback.loading);
  const loading = $derived(!!playback?.loading);
  const rpmValue = $derived(
    playback?.rpm === "45" || (!playback && side?.rpm === "45") ? 45 : 33,
  );
  const rpmLabel = $derived(rpmValue === 45 ? "45" : "33⅓");
  const position = $derived(playback?.position ?? 0);
  const duration = $derived(playback?.duration ?? 0);
</script>

<section class="deck" aria-label="Plattenspieler">
  <header class="brand" data-tauri-drag-region>
    <h1>LONGPLAY</h1>
    <p>DIRECT DRIVE · MODEL 01</p>
  </header>

  <div class="stage">
    <div class="platter-wrap">
      <div class="platter">
        <div class="rim"></div>
        <svg class="strobe" viewBox="0 0 400 400" aria-hidden="true">
          <circle
            cx="200"
            cy="200"
            r="192"
            fill="none"
            stroke="rgba(239,230,207,0.24)"
            stroke-width="3.4"
            stroke-dasharray="2.2 10"
          />
        </svg>

        {#if album && side}
          <Vinyl {album} {side} {grooves} spinning={playing} />
        {:else}
          <div class="mat"></div>
          <div class="spindle"></div>
        {/if}
      </div>

      {#if album && side}
        <div class="now">
          <span class="now-title">{album.albumArtist} — {album.title}</span>
          <div class="sides" role="group" aria-label="Seite">
            {#each album.sides as diskSide, index}
              <button
                class="side-tab"
                class:active={index === sideIndex}
                type="button"
                disabled={loading}
                onclick={() => onSide(index)}
                title={`Seite ${diskSide.label} auflegen`}
              >
                {diskSide.label}
              </button>
            {/each}
            <span class="rpm-note">{rpmLabel} RPM</span>
          </div>
          <span class="time">
            {#if loading}
              Platte wird aufgelegt …
            {:else}
              {formatClock(position)} / {formatClock(duration)}
              {#if playback?.trackTitle}
                · {playback.trackNumber}. {playback.trackTitle}
              {/if}
            {/if}
          </span>
        </div>
      {:else}
        <p class="deck-hint">Platte aus der Kiste auflegen</p>
      {/if}
    </div>

    <svg class="arm" viewBox="0 0 300 400" aria-hidden="true">
      <defs>
        <linearGradient id="chrome" x1="0" y1="0" x2="1" y2="0">
          <stop offset="0%" stop-color="#6d6d6d" />
          <stop offset="22%" stop-color="#e9e9e9" />
          <stop offset="52%" stop-color="#8f8f8f" />
          <stop offset="78%" stop-color="#d8d8d8" />
          <stop offset="100%" stop-color="#5a5a5a" />
        </linearGradient>
        <linearGradient id="weight" x1="0" y1="0" x2="1" y2="0">
          <stop offset="0%" stop-color="#3d3d3d" />
          <stop offset="30%" stop-color="#b8b8b8" />
          <stop offset="70%" stop-color="#6c6c6c" />
          <stop offset="100%" stop-color="#303030" />
        </linearGradient>
        <radialGradient id="base" cx="38%" cy="30%" r="70%">
          <stop offset="0%" stop-color="#9a9a9a" />
          <stop offset="55%" stop-color="#4e4e4e" />
          <stop offset="100%" stop-color="#232323" />
        </radialGradient>
        <filter id="armShadow" x="-30%" y="-20%" width="170%" height="150%">
          <feDropShadow dx="4" dy="7" stdDeviation="6" flood-color="#000" flood-opacity="0.55" />
        </filter>
      </defs>

      <g filter="url(#armShadow)">
        <ellipse cx="214" cy="104" rx="46" ry="40" fill="url(#base)" opacity="0.95" />

        <g class="moving" style:transform={album ? "rotate(22deg)" : "rotate(0deg)"}>
          <g transform="rotate(-14 196 62)">
            <rect x="180" y="26" width="30" height="62" rx="13" fill="url(#weight)" />
            <rect
              x="180"
              y="26"
              width="30"
              height="62"
              rx="13"
              fill="none"
              stroke="rgba(0,0,0,0.45)"
            />
          </g>

          <path d="M214 96 L242 330" stroke="#0f0f0f" stroke-width="9" stroke-linecap="round" />
          <path
            d="M214 96 L242 330"
            stroke="url(#chrome)"
            stroke-width="5.5"
            stroke-linecap="round"
          />

          <circle cx="214" cy="98" r="24" fill="url(#base)" />
          <circle cx="214" cy="98" r="13" fill="#232323" stroke="rgba(255,255,255,0.25)" />
          <circle cx="214" cy="98" r="4" fill="#0d0d0d" />
          <circle cx="210" cy="94" r="2" fill="rgba(255,255,255,0.6)" />

          <g transform="rotate(4 242 330)">
            <rect x="232" y="318" width="20" height="42" rx="2.5" fill="#1c1c1c" />
            <rect x="235" y="321" width="14" height="26" rx="1.5" fill="#b5342a" />
            <rect x="237" y="349" width="10" height="9" rx="1" fill="#9a9a9a" />
          </g>
        </g>

        <path
          d="M258 286 l16 -10 l8 12 l-16 10 z"
          fill="#2b2b2b"
          stroke="rgba(255,255,255,0.14)"
        />
      </g>
    </svg>
  </div>

  <div class="controls">
    <button
      class="start"
      class:ready={!!album && !loading}
      class:playing
      type="button"
      disabled={!album || loading}
      onclick={onToggle}
      title={playing ? "Wiedergabe anhalten" : "Platte starten"}
    >
      <span class="primary">{playing ? "STOP" : "START"}</span>
      <span class="sub">{playing ? "START" : "STOP"}</span>
    </button>

    <div class="rpm" role="group" aria-label="Drehzahl">
      <button
        class="speed"
        class:active={rpmValue === 33}
        type="button"
        disabled={!album || loading}
        onclick={() => onRpm(33)}>33</button
      >
      <button
        class="speed"
        class:active={rpmValue === 45}
        type="button"
        disabled={!album || loading}
        onclick={() => onRpm(45)}>45</button
      >
    </div>

    <button
      class="brush"
      type="button"
      aria-disabled={!album}
      title={album ? "Anti-Static-Bürste" : "Noch keine Platte aufgelegt"}
    >
      ANTI-STATIC
    </button>
  </div>
</section>

<style>
  .deck {
    position: relative;
    height: 100%;
    min-width: 0;
    padding: 18px 30px 26px;
    display: flex;
    flex-direction: column;
    background:
      radial-gradient(70% 52% at 52% 16%, rgba(255, 210, 155, 0.11), transparent 70%),
      repeating-linear-gradient(
        0deg,
        rgba(0, 0, 0, 0.12) 0 1px,
        rgba(255, 255, 255, 0.012) 1px 3px,
        transparent 3px 7px
      ),
      repeating-linear-gradient(0deg, rgba(255, 190, 120, 0.035) 0 1px, transparent 1px 23px),
      linear-gradient(90deg, rgba(0, 0, 0, 0.5), transparent 6%),
      linear-gradient(180deg, #4a2d1c, #301b10 65%, #24140c);
    border-left: 1px solid rgba(0, 0, 0, 0.55);
    box-shadow: -14px 0 30px rgba(0, 0, 0, 0.28) inset;
  }

  .brand {
    flex: 0 0 auto;
    padding: 6px 0 0 8px;
  }

  .brand h1 {
    margin: 0;
    font-family: var(--font-display);
    font-size: 21px;
    font-weight: 400;
    letter-spacing: 0.28em;
    color: rgba(239, 230, 207, 0.62);
    text-shadow:
      0 -1px 0 rgba(0, 0, 0, 0.65),
      0 1px 0 rgba(255, 220, 170, 0.1);
  }

  .brand p {
    margin: 4px 0 0;
    font-size: 9px;
    letter-spacing: 0.24em;
    color: rgba(239, 230, 207, 0.3);
  }

  .stage {
    position: relative;
    flex: 1 1 auto;
    min-height: 0;
    display: flex;
    align-items: center;
  }

  .platter-wrap {
    width: min(78%, 540px);
    margin-left: 2%;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 14px;
  }

  .platter {
    position: relative;
    width: 100%;
    aspect-ratio: 1;
    border-radius: 50%;
    background:
      radial-gradient(circle at 42% 34%, rgba(255, 255, 255, 0.11), transparent 42%),
      radial-gradient(circle at 50% 50%, #2c2c2c 0 62%, #1a1a1a 63% 100%);
    box-shadow:
      0 24px 48px rgba(0, 0, 0, 0.6),
      0 2px 0 rgba(255, 255, 255, 0.08) inset,
      0 -2px 0 rgba(0, 0, 0, 0.6) inset;
  }

  .strobe {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }

  .rim {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background: conic-gradient(
      from 210deg,
      #585858,
      #c9c9c9 12%,
      #6a6a6a 26%,
      #ededed 40%,
      #5f5f5f 54%,
      #b5b5b5 68%,
      #565656 82%,
      #d6d6d6 92%,
      #585858
    );
    -webkit-mask: radial-gradient(circle at 50% 50%, transparent 0 95.2%, #000 95.6%);
    mask: radial-gradient(circle at 50% 50%, transparent 0 95.2%, #000 95.6%);
    opacity: 0.55;
  }

  .mat {
    position: absolute;
    inset: 6%;
    border-radius: 50%;
    background:
      radial-gradient(circle at 40% 30%, rgba(255, 255, 255, 0.055), transparent 55%),
      repeating-radial-gradient(
        circle at 50% 50%,
        rgba(255, 255, 255, 0.018) 0 1px,
        transparent 1px 4px
      ),
      #222;
    box-shadow:
      0 0 0 1px rgba(0, 0, 0, 0.6) inset,
      0 0 40px rgba(0, 0, 0, 0.55) inset;
  }

  .spindle {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 3.2%;
    aspect-ratio: 1;
    border-radius: 50%;
    transform: translate(-50%, -50%);
    background: radial-gradient(circle at 35% 30%, #f2f2f2, #8a8a8a 55%, #3a3a3a);
    box-shadow:
      0 1px 3px rgba(0, 0, 0, 0.7),
      0 0 0 1px rgba(0, 0, 0, 0.5);
  }

  .deck-hint {
    margin: 0;
    font-family: var(--font-display);
    font-size: 12px;
    font-style: italic;
    color: rgba(239, 230, 207, 0.36);
  }

  .now {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 7px;
    max-width: 100%;
  }

  .now-title {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-display);
    font-size: 12px;
    color: rgba(239, 230, 207, 0.6);
  }

  .sides {
    display: flex;
    align-items: center;
    gap: 7px;
  }

  .side-tab {
    width: 24px;
    height: 20px;
    border-radius: 3px;
    font-size: 9px;
    letter-spacing: 0.08em;
    color: rgba(239, 230, 207, 0.45);
    background: linear-gradient(180deg, #2a1a0e, #17100a);
    box-shadow:
      0 1px 0 rgba(255, 220, 170, 0.08) inset,
      0 2px 4px rgba(0, 0, 0, 0.5);
  }

  .side-tab.active {
    color: var(--brass-light);
    box-shadow:
      0 0 0 1px rgba(201, 162, 39, 0.45) inset,
      0 0 10px rgba(201, 162, 39, 0.16);
  }

  .rpm-note {
    margin-left: 6px;
    font-size: 8px;
    letter-spacing: 0.16em;
    color: rgba(239, 230, 207, 0.32);
  }

  .time {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 9px;
    letter-spacing: 0.12em;
    color: rgba(239, 230, 207, 0.38);
    font-variant-numeric: tabular-nums;
  }

  .arm {
    position: absolute;
    right: 1%;
    top: 3%;
    height: 92%;
    width: auto;
    overflow: visible;
    pointer-events: none;
  }

  .moving {
    transform-origin: 214px 98px;
    transform-box: view-box;
    transition: transform 700ms cubic-bezier(0.2, 0.8, 0.2, 1);
  }

  .controls {
    flex: 0 0 auto;
    display: flex;
    align-items: center;
    gap: 22px;
    padding: 0 8px 0 2%;
  }

  .start {
    width: 62px;
    height: 62px;
    border-radius: 50%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1px;
    color: #241708;
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.12em;
    background: radial-gradient(circle at 36% 28%, #f6f0e2, #cdc6b6 48%, #8e8878 100%);
    box-shadow:
      0 1px 0 rgba(255, 255, 255, 0.75) inset,
      0 -2px 6px rgba(0, 0, 0, 0.35) inset,
      0 6px 12px rgba(0, 0, 0, 0.55);
    opacity: 0.55;
  }

  .start.ready {
    opacity: 0.92;
  }

  .start.playing {
    box-shadow:
      0 1px 0 rgba(255, 255, 255, 0.75) inset,
      0 -2px 6px rgba(0, 0, 0, 0.35) inset,
      0 0 18px rgba(232, 207, 122, 0.35),
      0 6px 12px rgba(0, 0, 0, 0.55);
  }

  .start .primary {
    font-size: 10px;
    letter-spacing: 0.14em;
  }

  .start .sub {
    font-size: 7px;
    opacity: 0.65;
  }

  .rpm {
    display: flex;
    gap: 9px;
  }

  .speed {
    width: 30px;
    height: 30px;
    border-radius: 5px;
    font-size: 10px;
    letter-spacing: 0.06em;
    color: rgba(239, 230, 207, 0.42);
    background: linear-gradient(180deg, #2a1a0e, #17100a);
    box-shadow:
      0 1px 0 rgba(255, 220, 170, 0.08) inset,
      0 2px 4px rgba(0, 0, 0, 0.5);
    opacity: 0.85;
  }

  .speed.active {
    color: var(--brass-light);
    box-shadow:
      0 0 0 1px rgba(201, 162, 39, 0.45) inset,
      0 0 10px rgba(201, 162, 39, 0.16);
    opacity: 1;
  }

  .brush {
    margin-left: auto;
    margin-right: 4%;
    padding: 9px 26px;
    border-radius: 5px;
    font-size: 9px;
    letter-spacing: 0.18em;
    color: rgba(239, 230, 207, 0.5);
    background: linear-gradient(180deg, #5a3720, #3a2112 60%, #2a1710);
    box-shadow:
      0 1px 0 rgba(255, 220, 170, 0.14) inset,
      0 3px 8px rgba(0, 0, 0, 0.45);
    opacity: 0.8;
  }

  .brush[aria-disabled="true"] {
    cursor: default;
  }
</style>
