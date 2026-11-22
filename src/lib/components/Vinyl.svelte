<script lang="ts">
  import type { Album, GrooveBand, Side } from "$lib/types";
  import { formatDuration } from "$lib/library.svelte";

  let {
    album,
    side,
    grooves = [],
  }: { album: Album; side: Side; grooves?: GrooveBand[] } = $props();

  const sideDuration = $derived(
    side.tracks.reduce((total, track) => total + track.duration + track.pregap, 0),
  );
  const durationLabel = $derived(
    sideDuration >= 60 ? `${Math.round(sideDuration / 60)} MIN` : formatDuration(sideDuration),
  );
  const rpm = $derived(side.rpm === "45" ? "45" : "33⅓");


  let canvas: HTMLCanvasElement | undefined = $state();
  let host: HTMLDivElement | undefined = $state();
  let size = $state(0);
  let lastKey = "";

  $effect(() => {
    if (!host) return;
    const observer = new ResizeObserver((entries) => {
      size = entries[0]?.contentRect.width ?? host!.clientWidth;
    });
    observer.observe(host);
    size = host.clientWidth;
    return () => observer.disconnect();
  });

  $effect(() => {
    if (!canvas || size <= 0) return;
    const key = `${size}|${grooves.length}|${grooves
      .map((band) => `${band.startNorm.toFixed(5)}:${band.endNorm.toFixed(5)}`)
      .join(",")}`;
    if (key === lastKey) return;
    lastKey = key;
    drawGrooves(canvas, size, grooves);
  });

  function drawGrooves(target: HTMLCanvasElement, width: number, bands: GrooveBand[]) {
    const dpr = Math.min(window.devicePixelRatio || 1, 3);
    const pixels = Math.max(1, Math.round(width * dpr));
    if (target.width !== pixels || target.height !== pixels) {
      target.width = pixels;
      target.height = pixels;
    }
    const context = target.getContext("2d");
    if (!context) return;

    context.setTransform(dpr, 0, 0, dpr, 0, 0);
    context.clearRect(0, 0, width, width);
    const center = width / 2;
    const tau = Math.PI * 2;

    for (const band of bands) {
      const outer = band.startNorm * center;
      const inner = band.endNorm * center;
      const bandWidth = outer - inner;
      if (bandWidth <= 0.15) continue;

      if (band.kind === "track") {
        for (let radius = outer - 0.35; radius > inner; radius -= 0.8) {
          const alpha = 0.07 + 0.045 * (0.5 + 0.5 * Math.sin(radius * 1.9));
          context.beginPath();
          context.arc(center, center, radius, 0, tau);
          context.strokeStyle = `rgba(255,255,255,${alpha.toFixed(3)})`;
          context.lineWidth = 0.6;
          context.stroke();
        }
        for (const [edge, alpha] of [
          [outer, 0.2],
          [inner, 0.14],
        ] as const) {
          context.beginPath();
          context.arc(center, center, edge, 0, tau);
          context.strokeStyle = `rgba(255,255,255,${alpha})`;
          context.lineWidth = 1;
          context.stroke();
        }
      } else if (band.kind === "gap") {
        context.beginPath();
        context.arc(center, center, (outer + inner) / 2, 0, tau);
        context.strokeStyle = "rgba(0,0,0,0.38)";
        context.lineWidth = Math.max(1.4, bandWidth);
        context.stroke();
      } else {
        for (let radius = outer; radius > inner; radius -= 2.4) {
          context.beginPath();
          context.arc(center, center, radius, 0, tau);
          context.strokeStyle = "rgba(255,255,255,0.03)";
          context.lineWidth = 0.5;
          context.stroke();
        }
      }
    }
  }
</script>

<div
  class="vinyl"
  class:single={side.rpm === "45"}
  bind:this={host}
  style:--vinyl={album.palette.vinyl}
  style:--light={album.palette.vinylLight}
  style:--label={album.palette.label}
  style:--ink={album.palette.dark}
>
  <div class="disc-layer" aria-hidden="true">
    <div class="grooves"></div>
    <canvas class="grooves-canvas" bind:this={canvas} aria-hidden="true"></canvas>

    <div class="label">
      <div class="top">
        <span class="cat">{side.matrix ?? "LONGPLAY"}</span>
        <span class="title">{album.title}</span>
      </div>
      <div class="bottom">
        <span class="artist">{album.albumArtist}</span>
        <span class="meta">SEITE {side.label} · {rpm} RPM</span>
        <span class="meta dim">{side.tracks.length} TRACKS · {durationLabel}</span>
      </div>
    </div>
  </div>

  <div class="sheen" aria-hidden="true"></div>
  <div class="edge" aria-hidden="true"></div>
  <div class="spindle" aria-hidden="true"></div>
</div>

<style>
  .vinyl {
    position: absolute;
    inset: 3.5%;
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
    container-type: inline-size;
    animation: vinyl-in 620ms cubic-bezier(0.2, 0.85, 0.2, 1) both;
  }

  .vinyl.single {
    inset: 23%;
  }

  .vinyl.single .label {
    width: 46%;
  }

  .vinyl.single .title {
    font-size: 2.9cqw;
  }

  .vinyl.single .artist {
    font-size: 1.9cqw;
  }

  .vinyl.single .cat,
  .vinyl.single .meta {
    font-size: 1.4cqw;
  }

  .disc-layer {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    will-change: transform;
  }

  .grooves-canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    border-radius: 50%;
    pointer-events: none;
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
    opacity: 0.3;
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
    font-size: 1.18cqw;
    letter-spacing: 0.18em;
    opacity: 0.62;
    white-space: nowrap;
  }

  .title {
    font-family: var(--font-display);
    font-size: 2.35cqw;
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
    font-size: 1.6cqw;
    font-style: italic;
    opacity: 0.82;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .meta {
    font-size: 1.18cqw;
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

  .vinyl.single .spindle {
    width: 7%;
  }

  @keyframes vinyl-in {
    from {
      opacity: 0;
      transform: rotateY(74deg) scale(0.94);
      filter: brightness(0.55);
    }
    to {
      opacity: 1;
      transform: rotateY(0deg) scale(1);
      filter: brightness(1);
    }
  }
</style>
