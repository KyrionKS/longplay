<script lang="ts">
  import { onMount } from "svelte";
  import "../lib/theme.css";
  import WoodBackground from "$lib/components/WoodBackground.svelte";
  import Crate from "$lib/components/Crate.svelte";
  import Deck from "$lib/components/Deck.svelte";
  import Gatefold from "$lib/components/Gatefold.svelte";
  import { addMusicFolder, library, loadLibrary, rescan } from "$lib/library.svelte";
  import {
    loadSide,
    playback,
    refreshPlayback,
    setRpm,
    togglePlayback,
  } from "$lib/playback.svelte";
  import type { Album, SortKey } from "$lib/types";

  let selectedId = $state<string | null>(null);
  let sideIndex = $state(0);
  let gatefoldOpen = $state(false);
  let sleeveFlipped = $state(false);

  const collator = new Intl.Collator("de", { sensitivity: "base" });

  const visible = $derived.by(() => {
    const query = library.query.trim().toLowerCase();
    let albums = library.albums.filter(
      (album) =>
        !query ||
        album.title.toLowerCase().includes(query) ||
        album.albumArtist.toLowerCase().includes(query),
    );

    switch (library.sort) {
      case "artist":
        albums = [...albums].sort(
          (a, b) =>
            collator.compare(a.albumArtist, b.albumArtist) || collator.compare(a.title, b.title),
        );
        break;
      case "album":
        albums = [...albums].sort((a, b) => collator.compare(a.title, b.title));
        break;
      case "year":
        albums = [...albums].sort((a, b) => (b.year ?? 0) - (a.year ?? 0));
        break;
      default:
        break;
    }
    return albums;
  });

  const pageCount = $derived(Math.max(1, Math.ceil(visible.length / 9)));
  const pageAlbums = $derived(visible.slice(library.page * 9, library.page * 9 + 9));

  const selected = $derived(library.albums.find((album) => album.id === selectedId) ?? null);
  const side = $derived(selected?.sides[sideIndex] ?? null);

  const loadedState = $derived(
    playback.state &&
      selected &&
      playback.state.albumId === selected.id &&
      playback.state.sideIndex === sideIndex
      ? playback.state
      : null,
  );

  const sortOrder: SortKey[] = ["added", "artist", "album", "year"];

  $effect(() => {
    if (library.page > pageCount - 1) library.page = pageCount - 1;
  });

  $effect(() => {
    if (selected && sideIndex > selected.sides.length - 1) sideIndex = 0;
  });

  onMount(() => {
    loadLibrary();
    const timer = window.setInterval(async () => {
      await refreshPlayback();
      const state = playback.state;
      if (!selectedId && state?.loaded && state.albumId) {
        selectedId = state.albumId;
        sideIndex = state.sideIndex;
      }
    }, 250);

    const onKey = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA")) {
        return;
      }
      if (gatefoldOpen) {
        if (event.key === "Escape") {
          gatefoldOpen = false;
          return;
        }
        if (
          event.code === "Space" ||
          event.key === "Enter" ||
          event.key === "ArrowRight" ||
          event.key === "ArrowLeft"
        ) {
          event.preventDefault();
          sleeveFlipped = !sleeveFlipped;
          return;
        }
        return;
      }
      if ((event.key === "g" || event.key === "G") && selected) {
        event.preventDefault();
        gatefoldOpen = true;
        sleeveFlipped = false;
        return;
      }
      if (event.code === "Space") {
        event.preventDefault();
        toggle();
      }
      const digit = Number(event.key);
      if (digit >= 1 && digit <= 9) {
        event.preventDefault();
        const album = pageAlbums[digit - 1];
        if (album) select(album);
      }
    };
    window.addEventListener("keydown", onKey);

    return () => {
      window.clearInterval(timer);
      window.removeEventListener("keydown", onKey);
    };
  });

  function cycleSort() {
    const index = sortOrder.indexOf(library.sort);
    library.sort = sortOrder[(index + 1) % sortOrder.length];
    library.page = 0;
  }

  function search(value: string) {
    library.query = value;
    library.page = 0;
  }

  function select(album: Album) {
    selectedId = album.id;
    sideIndex = 0;
    gatefoldOpen = false;
    sleeveFlipped = false;
    void loadSide(album.id, 0, false);
  }

  function changeSide(index: number) {
    if (!selected) return;
    const wasPlaying = !!playback.state?.playing;
    sideIndex = index;
    void loadSide(selected.id, index, wasPlaying);
  }

  function toggle() {
    if (!selected) return;
    void togglePlayback();
  }

  function changeRpm(rpm: number) {
    void setRpm(rpm);
  }
</script>

<main class="app">
  <WoodBackground />

  <div class="drag-strip" data-tauri-drag-region></div>

  <div class="panes">
    <Crate
      albums={pageAlbums}
      total={visible.length}
      page={library.page}
      {pageCount}
      sort={library.sort}
      query={library.query}
      scanning={library.scanning || library.loading}
      onAdd={addMusicFolder}
      onRescan={rescan}
      onSelect={select}
      onPage={(page) => (library.page = page)}
      onSort={cycleSort}
      onQuery={search}
    />
    <Deck
      album={selected}
      {side}
      {sideIndex}
      playback={playback.state}
      grooves={loadedState?.grooves ?? []}
      onSide={changeSide}
      onToggle={toggle}
      onRpm={changeRpm}
      onCover={() => {
        gatefoldOpen = true;
        sleeveFlipped = false;
      }}
    />
  </div>

  {#if library.error || playback.error}
    <div class="error" role="alert">{library.error ?? playback.error}</div>
  {/if}

  {#if gatefoldOpen && selected}
    <Gatefold
      album={selected}
      bind:flipped={sleeveFlipped}
      onClose={() => (gatefoldOpen = false)}
    />
  {/if}
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

  .error {
    position: absolute;
    z-index: 20;
    left: 26px;
    right: 26px;
    bottom: 18px;
    padding: 10px 14px;
    border-radius: 4px;
    font-size: 11px;
    color: #f3dede;
    background: rgba(86, 20, 20, 0.9);
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.45);
  }
</style>
