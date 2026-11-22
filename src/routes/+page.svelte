<script lang="ts">
  import "../lib/theme.css";
  import WoodBackground from "$lib/components/WoodBackground.svelte";
  import Crate from "$lib/components/Crate.svelte";
  import Deck from "$lib/components/Deck.svelte";
</script>

<main class="app">
  <WoodBackground />

  <div class="drag-strip" data-tauri-drag-region></div>

  <div class="panes">
    <Crate />
    <Deck />
  </div>
</main>

<style>
  .app {
    position: fixed;
    inset: 0;
    display: flex;
    flex-direction: column;
    background: var(--wood-mid);
  }

  .drag-strip {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 28px;
    z-index: 10;
    background: linear-gradient(180deg, rgba(0, 0, 0, 0.5), transparent);
  }

  .panes {
    position: relative;
    z-index: 1;
    flex: 1 1 auto;
    min-height: 0;
    display: flex;
    padding-top: 28px;
  }

  .panes > :global(.crate) {
    flex: 0 0 40%;
  }

  .panes > :global(.deck) {
    flex: 1 1 60%;
  }
</style>
