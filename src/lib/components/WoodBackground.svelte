<script lang="ts">
  // Statische Holzfläche. Wird später durch Texturen ersetzt, bleibt aber der
  // Hintergrund für Crate und Deck.
</script>

<div class="wood" aria-hidden="true">
  <div class="texture"></div>
  <div class="grain"></div>
  <div class="light"></div>
  <div class="vignette"></div>
</div>

<style>
  .wood {
    position: absolute;
    inset: 0;
    background-color: var(--wood-mid);
    background-image:
      radial-gradient(120% 70% at 42% -8%, rgba(255, 214, 160, 0.16), transparent 58%),
      linear-gradient(180deg, rgba(32, 16, 7, 0.5), rgba(18, 9, 4, 0.72));
  }

  .texture {
    position: absolute;
    inset: 0;
    background-image: url("/textures/wood.jpg");
    background-size: 960px 960px;
    background-position: center;
    filter: saturate(0.85) brightness(0.72) contrast(1.06);
    opacity: 0.92;
  }

  .grain {
    position: absolute;
    inset: 0;
    opacity: 0.5;
    mix-blend-mode: overlay;
    background-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='180' height='180'><filter id='n'><feTurbulence type='fractalNoise' baseFrequency='0.85' numOctaves='4' stitchTiles='stitch'/><feColorMatrix type='saturate' values='0'/></filter><rect width='180' height='180' filter='url(%23n)' opacity='0.35'/></svg>");
    background-size: 180px 180px;
  }

  .light {
    position: absolute;
    inset: -10% 20% 35% 8%;
    background: radial-gradient(
      60% 55% at 50% 0%,
      rgba(255, 226, 180, 0.18),
      rgba(255, 210, 150, 0.05) 45%,
      transparent 70%
    );
    filter: blur(6px);
  }

  .vignette {
    position: absolute;
    inset: 0;
    background:
      radial-gradient(120% 90% at 50% 45%, transparent 45%, rgba(0, 0, 0, 0.55) 100%),
      linear-gradient(180deg, rgba(0, 0, 0, 0.35), transparent 12%, transparent 88%, rgba(0, 0, 0, 0.3));
  }
</style>
