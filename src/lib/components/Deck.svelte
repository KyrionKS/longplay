<script lang="ts">
  import { untrack } from "svelte";
  import type { Album, GrooveBand, PlaybackState, Side } from "$lib/types";
  import { cuePlayback, formatClock, pausePlayback } from "$lib/playback.svelte";
  import { cuePointForRadius, thetaForRadius } from "$lib/groove";
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

  let { album, side, sideIndex, playback, grooves, onSide, onToggle, onRpm }: Props =
    $props();

  const playing = $derived(!!playback?.playing && !playback?.loading);
  const loading = $derived(!!playback?.loading);
  const rpmValue = $derived(
    playback?.rpm === "45" || (!playback && side?.rpm === "45") ? 45 : 33,
  );
  const rpmLabel = $derived(rpmValue === 45 ? "45" : "33⅓");
  const position = $derived(playback?.position ?? 0);
  const duration = $derived(playback?.duration ?? 0);
  const atEnd = $derived(
    !!playback && duration > 0 && !playback.playing && position >= duration - 0.08,
  );

  const REST_PHI = 1.26;

  let stageEl: HTMLDivElement | undefined = $state();
  let platterEl: HTMLDivElement | undefined = $state();
  let stageW = $state(0);
  let stageH = $state(0);
  let centerX = $state(0);
  let centerY = $state(0);
  let platterRadius = $state(0);

  $effect(() => {
    const stage = stageEl;
    const platter = platterEl;
    if (!stage || !platter) return;

    const measure = () => {
      const stageRect = stage.getBoundingClientRect();
      const platterRect = platter.getBoundingClientRect();
      stageW = stageRect.width;
      stageH = stageRect.height;
      centerX = platterRect.left - stageRect.left + platterRect.width / 2;
      centerY = platterRect.top - stageRect.top + platterRect.height / 2;
      platterRadius = platterRect.width / 2;
    };

    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(stage);
    observer.observe(platter);
    return () => observer.disconnect();
  });

  const REFERENCE_MM = 151;
  const mmToPx = $derived((platterRadius * 0.94) / REFERENCE_MM);
  const pivotDistance = $derived(playback?.pivotDistance ?? 211);
  const armLength = $derived(playback?.armLength ?? 230);
  // The pivot sits up-right of the spindle, like a real deck.
  const PIVOT_ANGLE = -0.34;
  const pivotX = $derived(centerX + pivotDistance * mmToPx * Math.cos(PIVOT_ANGLE));
  const pivotY = $derived(centerY + pivotDistance * mmToPx * Math.sin(PIVOT_ANGLE));
  const armLengthPx = $derived(armLength * mmToPx);

  let dragging = $state(false);
  let manualPhi = $state(REST_PHI);
  let cueTime = $state(0);
  let cueTrackIndex = $state<number | null>(null);
  let cueX = $state(0);
  let cueY = $state(0);
  let hoverTrackIndex = $state<number | null>(null);
  let hoverX = $state(0);
  let hoverY = $state(0);

  const cueTrack = $derived(
    cueTrackIndex !== null ? (side?.tracks[cueTrackIndex] ?? null) : null,
  );
  const hoverTrack = $derived(
    hoverTrackIndex !== null ? (side?.tracks[hoverTrackIndex] ?? null) : null,
  );

  const statePhi = $derived.by(() => {
    if (dragging) return manualPhi;
    if (!album || !side || !playback || !playback.loaded || loading) return REST_PHI;
    if (atEnd) return REST_PHI;
    if (!playback.playing && playback.position < 0.05) return REST_PHI;
    return PIVOT_ANGLE + Math.PI - playback.armAngle;
  });

  let armPhi = $state(REST_PHI);

  $effect(() => {
    const target = statePhi;
    const speed = dragging ? 30 : 8;
    let frame = 0;
    let last = performance.now();

    const loop = (now: number) => {
      const dt = Math.min(0.05, (now - last) / 1000);
      last = now;
      untrack(() => {
        if (Math.abs(target - armPhi) > 0.0004) {
          armPhi += (target - armPhi) * (1 - Math.exp(-speed * dt));
        }
      });
      frame = requestAnimationFrame(loop);
    };

    frame = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(frame);
  });

  const stylusX = $derived(pivotX + armLengthPx * Math.cos(armPhi));
  const stylusY = $derived(pivotY + armLengthPx * Math.sin(armPhi));
  const phiDeg = $derived((armPhi * 180) / Math.PI);
  const weightX = $derived(pivotX - Math.cos(armPhi) * armLengthPx * 0.24);
  const weightY = $derived(pivotY - Math.sin(armPhi) * armLengthPx * 0.24);

  function pointerInfo(event: PointerEvent) {
    if (!stageEl || !platterEl || mmToPx <= 0) return null;
    const stage = stageEl.getBoundingClientRect();
    const platter = platterEl.getBoundingClientRect();
    const dx = event.clientX - (platter.left + platter.width / 2);
    const dy = event.clientY - (platter.top + platter.height / 2);
    return {
      x: event.clientX - stage.left,
      y: event.clientY - stage.top,
      radius: Math.hypot(dx, dy) / mmToPx,
    };
  }

  function updateCue(event: PointerEvent) {
    const info = pointerInfo(event);
    if (!info || !playback) return;
    const point = cuePointForRadius(playback.grooves, info.radius);
    if (!point) return;
    cueTime = point.time;
    cueTrackIndex = point.trackIndex;
    cueX = info.x;
    cueY = info.y;
    manualPhi =
      PIVOT_ANGLE + Math.PI - thetaForRadius(point.radius, pivotDistance, armLength);
  }

  function updateHover(event: PointerEvent) {
    if (!playback?.loaded || dragging || loading) {
      hoverTrackIndex = null;
      return;
    }
    const info = pointerInfo(event);
    if (!info) return;
    const point = cuePointForRadius(playback.grooves, info.radius);
    if (!point || point.trackIndex === null) {
      hoverTrackIndex = null;
      return;
    }
    hoverTrackIndex = point.trackIndex;
    hoverX = info.x;
    hoverY = info.y;
  }

  function onPointerDown(event: PointerEvent) {
    if (!album || !side || !playback?.loaded || loading) return;
    dragging = true;
    (event.currentTarget as HTMLElement).setPointerCapture?.(event.pointerId);
    updateCue(event);
    void pausePlayback();
  }

  function onPointerMove(event: PointerEvent) {
    if (dragging) {
      updateCue(event);
    } else {
      updateHover(event);
    }
  }

  function onPointerUp() {
    if (!dragging) return;
    dragging = false;
    hoverTrackIndex = null;
    void cuePlayback(cueTime);
  }

  function onPointerLeave() {
    hoverTrackIndex = null;
  }
</script>

<section class="deck" aria-label="Plattenspieler">
  <header class="brand" data-tauri-drag-region>
    <h1>LONGPLAY</h1>
    <p>DIRECT DRIVE · MODEL 01</p>
  </header>

  <div class="stage" bind:this={stageEl}>
    <div class="platter-wrap">
      <div
        class="platter"
        class:dragging
        role="group"
        aria-label="Plattenteller — ziehen, um die Nadel zu setzen"
        bind:this={platterEl}
        onpointerdown={onPointerDown}
        onpointermove={onPointerMove}
        onpointerup={onPointerUp}
        onpointercancel={onPointerUp}
        onpointerleave={onPointerLeave}
      >
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
          {#key `${album.id}:${side.label}`}
            <Vinyl {album} {side} {grooves} spinning={playing || dragging} />
          {/key}
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

    {#if dragging && cueTrack}
      <div class="cue-badge" style:left={`${cueX}px`} style:top={`${cueY}px`}>
        <span class="cue-time">{formatClock(cueTime)}</span>
        <strong>{cueTrack.number}. {cueTrack.title}</strong>
      </div>
    {:else if hoverTrack && !dragging}
      <div class="hover-badge" style:left={`${hoverX}px`} style:top={`${hoverY}px`}>
        {hoverTrack.number}. {hoverTrack.title}
      </div>
    {/if}

    <svg
      class="arm-layer"
      width={stageW}
      height={stageH}
      viewBox={`0 0 ${stageW} ${stageH}`}
      aria-hidden="true"
    >
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
        <ellipse
          cx={pivotX}
          cy={pivotY}
          rx={Math.max(28, armLengthPx * 0.1)}
          ry={Math.max(25, armLengthPx * 0.09)}
          fill="url(#base)"
          opacity="0.95"
        />

        <path
          d={`M${weightX} ${weightY} L${stylusX} ${stylusY}`}
          stroke="#0d0d0d"
          stroke-width="11"
          stroke-linecap="round"
        />
        <path
          d={`M${weightX} ${weightY} L${stylusX} ${stylusY}`}
          stroke="#c9c9c9"
          stroke-width="7"
          stroke-linecap="round"
        />
        <path
          d={`M${pivotX} ${pivotY} L${stylusX} ${stylusY}`}
          stroke="rgba(255,255,255,0.55)"
          stroke-width="1.4"
          stroke-linecap="round"
        />

        <g transform={`translate(${weightX} ${weightY}) rotate(${phiDeg})`}>
          <rect
            x={-armLengthPx * 0.08}
            y={-armLengthPx * 0.042}
            width={armLengthPx * 0.16}
            height={armLengthPx * 0.084}
            rx={armLengthPx * 0.04}
            fill="url(#weight)"
          />
          <rect
            x={-armLengthPx * 0.08}
            y={-armLengthPx * 0.042}
            width={armLengthPx * 0.16}
            height={armLengthPx * 0.084}
            rx={armLengthPx * 0.04}
            fill="none"
            stroke="rgba(0,0,0,0.45)"
          />
        </g>

        <circle cx={pivotX} cy={pivotY} r={Math.max(16, armLengthPx * 0.062)} fill="url(#base)" />
        <circle
          cx={pivotX}
          cy={pivotY}
          r={Math.max(8, armLengthPx * 0.032)}
          fill="#232323"
          stroke="rgba(255,255,255,0.25)"
        />
        <circle cx={pivotX} cy={pivotY} r={Math.max(3, armLengthPx * 0.012)} fill="#0d0d0d" />

        <g transform={`translate(${stylusX} ${stylusY}) rotate(${phiDeg})`}>
          <rect
            x={-armLengthPx * 0.095}
            y={-armLengthPx * 0.03}
            width={armLengthPx * 0.085}
            height={armLengthPx * 0.06}
            rx="3"
            fill="#1c1c1c"
          />
          <rect
            x={-armLengthPx * 0.08}
            y={-armLengthPx * 0.022}
            width={armLengthPx * 0.05}
            height={armLengthPx * 0.044}
            rx="2"
            fill="#b5342a"
          />
          <rect
            x={-armLengthPx * 0.012}
            y={-armLengthPx * 0.008}
            width={armLengthPx * 0.022}
            height={armLengthPx * 0.016}
            rx="1"
            fill="#cfcfcf"
          />
        </g>
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
    cursor: grab;
    touch-action: none;
    perspective: 1200px;
    background:
      radial-gradient(circle at 42% 34%, rgba(255, 255, 255, 0.11), transparent 42%),
      radial-gradient(circle at 50% 50%, #2c2c2c 0 62%, #1a1a1a 63% 100%);
    box-shadow:
      0 24px 48px rgba(0, 0, 0, 0.6),
      0 2px 0 rgba(255, 255, 255, 0.08) inset,
      0 -2px 0 rgba(0, 0, 0, 0.6) inset;
  }

  .platter.dragging {
    cursor: grabbing;
  }

  .strobe {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
  }

  .rim {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    pointer-events: none;
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
    min-height: 58px;
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

  .arm-layer {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    overflow: visible;
    pointer-events: none;
  }

  .cue-badge,
  .hover-badge {
    position: absolute;
    z-index: 6;
    pointer-events: none;
    transform: translate(14px, 14px);
    padding: 5px 9px 6px;
    border-radius: 3px;
    font-size: 9px;
    letter-spacing: 0.1em;
    white-space: nowrap;
    color: var(--cream);
    background: rgba(18, 10, 5, 0.82);
    box-shadow:
      0 0 0 1px rgba(201, 162, 39, 0.35),
      0 4px 12px rgba(0, 0, 0, 0.5);
  }

  .hover-badge {
    opacity: 0.85;
    font-size: 8px;
    color: rgba(239, 230, 207, 0.72);
  }

  .cue-badge .cue-time {
    margin-right: 8px;
    color: var(--brass-light);
    font-variant-numeric: tabular-nums;
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
