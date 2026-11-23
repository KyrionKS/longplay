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
    conceal?: boolean;
    onGeometry?: (geometry: { x: number; y: number; r: number }) => void;
    onSide: (index: number) => void;
    onToggle: () => void;
    onRpm: (rpm: number) => void;
    onCover: () => void;
  }

  let {
    album,
    side,
    sideIndex,
    playback,
    grooves,
    conceal = false,
    onGeometry,
    onSide,
    onToggle,
    onRpm,
    onCover,
  }: Props = $props();

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

  const REST_PHI = 1.5;

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
      onGeometry?.({
        x: stageRect.left + centerX,
        y: stageRect.top + centerY,
        r: platterRadius * RECORD_RADIUS_FACTOR,
      });
    };

    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(stage);
    observer.observe(platter);
    return () => observer.disconnect();
  });

  const REFERENCE_MM = 151;
  // The platter is only the carrier: the record has a 3.5 % inset on each side,
  // so its radius is 0.93 × the platter radius. All tonearm dimensions scale
  // against the record, never against the platter.
  const RECORD_RADIUS_FACTOR = 1 - 2 * 0.035;
  const mmToPx = $derived((platterRadius * RECORD_RADIUS_FACTOR) / REFERENCE_MM);
  const pivotDistance = $derived(playback?.pivotDistance ?? 211);
  const armLength = $derived(playback?.armLength ?? 230);
  // The pivot sits high up-right of the spindle, like a real deck.
  const PIVOT_ANGLE = -0.62;
  const pivotX = $derived(centerX + pivotDistance * mmToPx * Math.cos(PIVOT_ANGLE));
  const pivotY = $derived(centerY + pivotDistance * mmToPx * Math.sin(PIVOT_ANGLE));
  const armLengthPx = $derived(armLength * mmToPx);

  let dragging = $state(false);
  let cueHold = $state<{
    phi: number;
    time: number;
    x: number;
    y: number;
    trackIndex: number | null;
  } | null>(null);
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
    if (cueHold) return cueHold.phi;
    if (!album || !side || !playback || !playback.loaded || loading) return REST_PHI;
    if (atEnd) return REST_PHI;
    return PIVOT_ANGLE + Math.PI - playback.armAngle;
  });

  let armPhi = $state(REST_PHI);
  let platterAngle = $state(0);
  let platterVelocity = 0;

  $effect(() => {
    const active = playing || dragging || !!cueHold;
    const target = active ? (Math.PI * 2) / (rpmValue === 45 ? 1.333 : 1.8) : 0;
    let frame = 0;
    let last = performance.now();

    const loop = (now: number) => {
      const dt = Math.min(0.05, (now - last) / 1000);
      last = now;
      const tau = target > platterVelocity ? 0.5 : 2.4;
      platterVelocity +=
        (target - platterVelocity) * (1 - Math.exp(-dt / tau));
      untrack(() => {
        if (platterVelocity > 1e-4 || target > 0) {
          platterAngle =
            (platterAngle + platterVelocity * dt) % (Math.PI * 2);
        }
      });
      frame = requestAnimationFrame(loop);
    };

    frame = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(frame);
  });

  $effect(() => {
    if (!cueHold) return;
    if (playback?.loading) return;
    const timer = setTimeout(() => {
      cueHold = null;
    }, 900);
    return () => clearTimeout(timer);
  });

  $effect(() => {
    album?.id;
    sideIndex;
    cueHold = null;
  });

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
  // The headshell is offset against the arm axis; the stylus sits in its middle.
  const shellPhi = $derived(armPhi + (24 * Math.PI) / 180);
  const shellPhiDeg = $derived((shellPhi * 180) / Math.PI);
  // The stylus is the front-centre of the headshell; the body and the tube
  // sit behind it.
  const shellRearX = $derived(
    stylusX - Math.cos(shellPhi) * armLengthPx * 0.16,
  );
  const shellRearY = $derived(
    stylusY - Math.sin(shellPhi) * armLengthPx * 0.16,
  );

  // S-shaped arm tube: two opposite bends between the pivot and the headshell.
  const tubeChordX = $derived(shellRearX - pivotX);
  const tubeChordY = $derived(shellRearY - pivotY);
  const tubeLength = $derived(
    Math.max(1, Math.hypot(tubeChordX, tubeChordY)),
  );
  const tubePerpX = $derived(-tubeChordY / tubeLength);
  const tubePerpY = $derived(tubeChordX / tubeLength);
  const sBend = $derived(armLengthPx * 0.18);
  const control1X = $derived(pivotX + tubeChordX * 0.14 + tubePerpX * sBend);
  const control1Y = $derived(pivotY + tubeChordY * 0.14 + tubePerpY * sBend);
  const control2X = $derived(pivotX + tubeChordX * 0.55 - tubePerpX * sBend * 0.45);
  const control2Y = $derived(pivotY + tubeChordY * 0.55 - tubePerpY * sBend * 0.45);
  const armPath = $derived(
    `M ${pivotX} ${pivotY} C ${control1X} ${control1Y} ${control2X} ${control2Y} ${shellRearX} ${shellRearY}`,
  );
  const restX = $derived(pivotX + Math.cos(REST_PHI) * armLengthPx);
  const restY = $derived(pivotY + Math.sin(REST_PHI) * armLengthPx);
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
    const sameTrack =
      cueTrackIndex !== null && cueTrackIndex === playback?.trackIndex;
    if (!sameTrack) {
      cueHold = {
        phi: manualPhi,
        time: cueTime,
        x: cueX,
        y: cueY,
        trackIndex: cueTrackIndex,
      };
    }
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
        <div
          class="platter-spin"
          style:transform={`rotate(${platterAngle}rad)`}
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
            <div class="record-slot" class:concealed={conceal}>
              {#key `${album.id}:${side.label}`}
                <Vinyl {album} {side} {grooves} />
              {/key}
            </div>
          {:else}
            <div class="mat"></div>
            <div class="spindle"></div>
          {/if}
        </div>
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
          <button class="cover-link" type="button" onclick={onCover}>
            Cover &amp; Rückseite
          </button>
          <span class="time">
            {#if cueHold}
              {formatClock(cueHold.time)} / {formatClock(duration)}
              {#if cueHold.trackIndex !== null && side?.tracks[cueHold.trackIndex]}
                · {side.tracks[cueHold.trackIndex].number}. {side.tracks[cueHold.trackIndex].title}
              {/if}
              · lädt …
            {:else if loading}
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

    {#if dragging || cueHold}
      {@const badgeTrack =
        (dragging ? cueTrackIndex : cueHold?.trackIndex ?? null) !== null
          ? side?.tracks[
              (dragging ? cueTrackIndex : cueHold?.trackIndex) as number
            ] ?? null
          : null}
      <div
        class="cue-badge"
        style:left={`${dragging ? cueX : (cueHold?.x ?? 0)}px`}
        style:top={`${dragging ? cueY : (cueHold?.y ?? 0)}px`}
      >
        <span class="cue-time">{formatClock(dragging ? cueTime : (cueHold?.time ?? 0))}</span>
        {#if badgeTrack}<strong>{badgeTrack.number}. {badgeTrack.title}</strong>{/if}
        {#if !dragging}<span class="cue-loading">lädt …</span>{/if}
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
          <stop offset="0%" stop-color="#0f0f0f" />
          <stop offset="26%" stop-color="#5c5c5c" />
          <stop offset="55%" stop-color="#202020" />
          <stop offset="82%" stop-color="#6f6f6f" />
          <stop offset="100%" stop-color="#111111" />
        </linearGradient>
        <linearGradient id="weightRing" x1="0" y1="0" x2="1" y2="0">
          <stop offset="0%" stop-color="#6a6a6a" />
          <stop offset="35%" stop-color="#e2e2e2" />
          <stop offset="70%" stop-color="#8a8a8a" />
          <stop offset="100%" stop-color="#4a4a4a" />
        </linearGradient>
        <radialGradient id="base" cx="38%" cy="30%" r="70%">
          <stop offset="0%" stop-color="#6f6f6f" />
          <stop offset="55%" stop-color="#2f2f2f" />
          <stop offset="100%" stop-color="#141414" />
        </radialGradient>
        <filter id="armShadow" x="-30%" y="-20%" width="170%" height="150%">
          <feDropShadow dx="4" dy="7" stdDeviation="6" flood-color="#000" flood-opacity="0.55" />
        </filter>
      </defs>

      <g filter="url(#armShadow)">
        <!-- Aluminium base plate -->
        <circle
          cx={pivotX}
          cy={pivotY}
          r={Math.max(48, armLengthPx * 0.15)}
          fill="url(#chrome)"
          opacity="0.32"
        />

        <!-- Gimbal housing -->
        <ellipse
          cx={pivotX}
          cy={pivotY}
          rx={Math.max(34, armLengthPx * 0.115)}
          ry={Math.max(30, armLengthPx * 0.1)}
          fill="url(#base)"
          opacity="0.95"
        />
        <circle
          cx={pivotX}
          cy={pivotY}
          r={Math.max(28, armLengthPx * 0.088)}
          fill="none"
          stroke="rgba(255,255,255,0.16)"
          stroke-width="3"
        />

        <!-- Arm rest -->
        <g transform={`translate(${restX} ${restY})`}>
          <rect x="-4" y="-2" width="8" height={armLengthPx * 0.06} rx="2" fill="#1f1f1f" />
          <path
            d={`M ${-armLengthPx * 0.022} ${-armLengthPx * 0.004} Q 0 ${armLengthPx * 0.018} ${armLengthPx * 0.022} ${-armLengthPx * 0.004}`}
            fill="none"
            stroke="#2a2a2a"
            stroke-width="4"
            stroke-linecap="round"
          />
        </g>

        <!-- Cue lever and anti-skate -->
        <g>
          <circle cx={pivotX - armLengthPx * 0.12} cy={pivotY + armLengthPx * 0.1} r={armLengthPx * 0.03} fill="#181818" stroke="rgba(255,255,255,0.18)" stroke-width="1.5" />
          <line
            x1={pivotX - armLengthPx * 0.12}
            y1={pivotY + armLengthPx * 0.1}
            x2={pivotX - armLengthPx * 0.12}
            y2={pivotY + armLengthPx * 0.075}
            stroke="rgba(255,255,255,0.5)"
            stroke-width="1.6"
          />
          <path
            d={`M ${pivotX + armLengthPx * 0.06} ${pivotY + armLengthPx * 0.1} L ${pivotX + armLengthPx * 0.115} ${pivotY + armLengthPx * 0.165}`}
            stroke="#bdbdbd"
            stroke-width="3.5"
            stroke-linecap="round"
          />
          <rect
            x={pivotX + armLengthPx * 0.1}
            y={pivotY + armLengthPx * 0.15}
            width={armLengthPx * 0.045}
            height={armLengthPx * 0.02}
            rx={armLengthPx * 0.01}
            fill="#111"
            transform={`rotate(32 ${pivotX + armLengthPx * 0.12} ${pivotY + armLengthPx * 0.16})`}
          />
        </g>

        <!-- Straight rear stub -->
        <path
          d={`M${weightX} ${weightY} L${pivotX} ${pivotY}`}
          stroke="#0d0d0d"
          stroke-width="14"
          stroke-linecap="round"
        />
        <path
          d={`M${weightX} ${weightY} L${pivotX} ${pivotY}`}
          stroke="#c9c9c9"
          stroke-width="9"
          stroke-linecap="round"
        />

        <!-- S-shaped tube -->
        <path
          d={armPath}
          fill="none"
          stroke="#0d0d0d"
          stroke-width="16"
          stroke-linecap="round"
        />
        <path
          d={armPath}
          fill="none"
          stroke="#c9c9c9"
          stroke-width="11"
          stroke-linecap="round"
        />
        <path
          d={armPath}
          fill="none"
          stroke="rgba(255,255,255,0.55)"
          stroke-width="2"
          stroke-linecap="round"
        />

        <!-- Aluminium collar before the headshell -->
        <g transform={`translate(${shellRearX} ${shellRearY}) rotate(${shellPhiDeg})`}>
          <rect
            x={-armLengthPx * 0.016}
            y={-armLengthPx * 0.019}
            width={armLengthPx * 0.032}
            height={armLengthPx * 0.038}
            rx={armLengthPx * 0.014}
            fill="url(#chrome)"
            stroke="rgba(0,0,0,0.45)"
          />
        </g>

        <!-- Counterweight with knurled dial -->
        <g transform={`translate(${weightX} ${weightY}) rotate(${phiDeg})`}>
          <rect
            x={-armLengthPx * 0.085}
            y={-armLengthPx * 0.045}
            width={armLengthPx * 0.17}
            height={armLengthPx * 0.09}
            rx={armLengthPx * 0.042}
            fill="url(#weight)"
          />
          <rect
            x={-armLengthPx * 0.085}
            y={-armLengthPx * 0.045}
            width={armLengthPx * 0.17}
            height={armLengthPx * 0.09}
            rx={armLengthPx * 0.042}
            fill="none"
            stroke="rgba(0,0,0,0.5)"
          />
          <rect
            x={armLengthPx * 0.075}
            y={-armLengthPx * 0.048}
            width={armLengthPx * 0.014}
            height={armLengthPx * 0.096}
            rx="2"
            fill="url(#weightRing)"
          />
          <line x1={-armLengthPx * 0.05} y1={-armLengthPx * 0.042} x2={-armLengthPx * 0.05} y2={armLengthPx * 0.042} stroke="rgba(0,0,0,0.5)" stroke-width="1.4" />
          <line x1={-armLengthPx * 0.03} y1={-armLengthPx * 0.042} x2={-armLengthPx * 0.03} y2={armLengthPx * 0.042} stroke="rgba(0,0,0,0.4)" stroke-width="1.2" />
          <line x1={-armLengthPx * 0.01} y1={-armLengthPx * 0.042} x2={-armLengthPx * 0.01} y2={armLengthPx * 0.042} stroke="rgba(0,0,0,0.3)" stroke-width="1" />
        </g>

        <!-- Pivot -->
        <circle cx={pivotX} cy={pivotY} r={Math.max(18, armLengthPx * 0.07)} fill="url(#base)" />
        <circle
          cx={pivotX}
          cy={pivotY}
          r={Math.max(9, armLengthPx * 0.036)}
          fill="#232323"
          stroke="rgba(255,255,255,0.25)"
        />
        <circle cx={pivotX} cy={pivotY} r={Math.max(3, armLengthPx * 0.012)} fill="#0d0d0d" />

        <!-- Headshell with finger lift, stylus centred -->
        <g transform={`translate(${stylusX} ${stylusY}) rotate(${shellPhiDeg})`}>
          <!-- headshell body, stylus at the front-centre -->
          <rect
            x={-armLengthPx * 0.16}
            y={-armLengthPx * 0.031}
            width={armLengthPx * 0.16}
            height={armLengthPx * 0.062}
            rx="4"
            fill="#161616"
            stroke="rgba(255,255,255,0.14)"
          />
          <ellipse cx={-armLengthPx * 0.12} cy="0" rx={armLengthPx * 0.012} ry={armLengthPx * 0.008} fill="#050505" stroke="rgba(255,255,255,0.1)" />
          <ellipse cx={-armLengthPx * 0.07} cy="0" rx={armLengthPx * 0.012} ry={armLengthPx * 0.008} fill="#050505" stroke="rgba(255,255,255,0.1)" />
          <ellipse cx={-armLengthPx * 0.022} cy="0" rx={armLengthPx * 0.01} ry={armLengthPx * 0.007} fill="#050505" stroke="rgba(255,255,255,0.1)" />
          <!-- black finger lift at the front corner -->
          <path
            d={`M ${-armLengthPx * 0.012} ${-armLengthPx * 0.03} Q ${armLengthPx * 0.012} ${-armLengthPx * 0.06} ${armLengthPx * 0.03} ${-armLengthPx * 0.035}`}
            fill="none"
            stroke="#101010"
            stroke-width="4"
            stroke-linecap="round"
          />
          <path
            d={`M ${-armLengthPx * 0.012} ${-armLengthPx * 0.03} Q ${armLengthPx * 0.012} ${-armLengthPx * 0.06} ${armLengthPx * 0.03} ${-armLengthPx * 0.035}`}
            fill="none"
            stroke="rgba(255,255,255,0.22)"
            stroke-width="1"
            stroke-linecap="round"
          />
          <!-- gold cartridge behind the stylus -->
          <rect
            x={-armLengthPx * 0.085}
            y={-armLengthPx * 0.02}
            width={armLengthPx * 0.06}
            height={armLengthPx * 0.04}
            rx="2"
            fill="#c8a24a"
            stroke="rgba(0,0,0,0.45)"
          />
          <rect
            x={-armLengthPx * 0.032}
            y={-armLengthPx * 0.009}
            width={armLengthPx * 0.03}
            height={armLengthPx * 0.018}
            fill="#1a1a1a"
          />
          <!-- cantilever and stylus tip at the very front -->
          <line
            x1={-armLengthPx * 0.004}
            y1="0"
            x2={0}
            y2="0"
            stroke="#d8d8d8"
            stroke-width="1.6"
          />
          <circle r={Math.max(3.4, armLengthPx * 0.013)} fill="rgba(0,0,0,0.35)" />
          <circle r={Math.max(1.9, armLengthPx * 0.0075)} fill="#f6f6f6" />
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

  .platter-spin {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    will-change: transform;
  }

  .record-slot {
    position: absolute;
    inset: 0;
  }

  .record-slot.concealed {
    visibility: hidden;
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
    background-image:
      url("/textures/metal.jpg"),
      conic-gradient(
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
    background-size: 260px 260px, auto;
    background-blend-mode: overlay, normal;
    -webkit-mask: radial-gradient(circle at 50% 50%, transparent 0 95.2%, #000 95.6%);
    mask: radial-gradient(circle at 50% 50%, transparent 0 95.2%, #000 95.6%);
    opacity: 0.62;
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

  .cover-link {
    margin-top: 1px;
    font-size: 8px;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: rgba(232, 207, 122, 0.72);
  }

  .cover-link:hover {
    color: var(--brass-light);
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

  .cue-loading {
    margin-left: 8px;
    font-size: 8px;
    letter-spacing: 0.14em;
    color: rgba(232, 207, 122, 0.7);
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
