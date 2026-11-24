Stand: d94028da7a56279249ab90d82f17ed3c099812e8

# Longplay — Codemap

## 1. Zweck + Stack
Skeuomorpher Plattenspieler für die lokale Sammlung: Alben als Platten in einer Holzkiste, Tonarm von Hand aufsetzen, 33⅓/45, Seiten A/B, Knistern (README.md:6-9).
Tauri 2 + SvelteKit mit `adapter-static`/SPA ohne SSR (svelte.config.js:12-14, src/routes/+layout.ts:5); der Rust-Kern hält Index, Audio und Playback-Modell, die Webview ist Präsentationsschicht (src-tauri/src/lib.rs:3-4).
Crates: lofty, walkdir, blake3, rusqlite (bundled), image, symphonia, arc-swap; Edition 2024 (src-tauri/Cargo.toml:6-32); cpal nur außerhalb Linux (src-tauri/Cargo.toml:44).
Frontend: Svelte 5, TypeScript, Vite (package.json:16-31), Port 1430 mit `strictPort` (vite.config.js:18-21).

## 2. Befehle
- `npm run tauri dev`: startet Vite + App (`beforeDevCommand`, `devUrl`) (src-tauri/tauri.conf.json:6-10).
- `npm run dev` allein: 127.0.0.1 oder `TAURI_DEV_HOST`, HMR 1431 nur mit Host (vite.config.js:5,19-31).
- `npm run build` = Vite-Build nach `build/`; Bundle via `npm run tauri build` (package.json:8, src-tauri/tauri.conf.json:6-10).
- `npm run check` = `svelte-kit sync` + `svelte-check` (package.json:11).
- Rust-Tests: `cargo test --manifest-path src-tauri/Cargo.toml` (README.md:98).

## 3. Modulkarte
- `src-tauri/src/lib.rs`: `AppState` (DB, Cover-Ordner, Session) (lib.rs:21-25), 12 IPC-Commands inkl. `app_version` (lib.rs:28-142), Aufbau/Setup (lib.rs:145-197).
- `library/scanner.rs`: Ordnerlauf, CUE-/Tag-/Playlist-Alben, Seiten- und Formatlogik (`scan` :71, `collect_files` :135, `group_key` :170, `build_from_files` :187, `build_playlist` :291, `build_from_cue` :402, `assemble` :543, `group_sides` :661, `split_index` :689).
- `library/m3u.rs`: toleranter M3U/M3U8-Parser, liefert nie Fehler (m3u.rs:2-6, `parse` :28).
- `library/model.rs`: Serde-DTOs in camelCase (model.rs:7-27), `DiscFormat` (model.rs:37-45), `AlbumSource` (model.rs:47-53), `Side`/`Rpm` (model.rs:55-84), `Track`/`Palette` (model.rs:86-120).
- `library/db.rs`: SQLite-Cache, Schema/Migration (db.rs:45-84), Schreiben/Lesen (db.rs:87-160), `album`/`folders` (db.rs:162-191).
- `library/cue.rs`: CUE-Sheets inkl. Pregap und Einzeldatei-Offsets (cue.rs:43-60, `parse_bytes` :85).
- `library/metadata.rs`: Tags via lofty (metadata.rs:35) und Dauer (metadata.rs:73); `artwork.rs`: Cover finden/einbetten/cachen (artwork.rs:24-59); `palette.rs`: Farben aus Cover (palette.rs:12-49).
- `audio/session.rs`: lädt je einen Track, prefetcht den nächsten, liefert `PlaybackState` (session.rs:23-45, `tick` :126, `schedule_prefetch` :144, `load` :171, `state` :355); aktiviert den Einlauf in `new` (session.rs:96).
- `audio/cue.rs`: WAV-Einlauf, eingebettet per `include_bytes!` (`samples` :33), minimaler 16-Bit-PCM-RIFF-Parser `parse_wav` :43, `CueVoice` :93 mit `start` :140 und `process` :164, lineare Interpolation `read_sample` :198. Fehlende/kaputte Dateien werden still zu `Sample::empty` (cue.rs:34-39).
- `src-tauri/assets/audio/needle_drop.wav` und `vinyl_static.wav`: echte Mono-Aufnahmen (48 kHz/16 Bit), beide CC0. Nadel: HenKonen (Freesound #682161); Statik: „Vinyl Crackle (Real)“ von beautifuldaymonster1968 (Freesound #434822, CC0), 12 s; Herkunft, Kürzung und Pegel in `CREDITS.md`.
- `audio/engine.rs`: DSP/Queue, `DspState` mit `cue`/`pre_roll` (engine.rs:21-28), `set_needle_pre_roll` (:223), `process_device` (:269), `process` (:311), Pre-Roll-Aufbau (:341-357), Cue-Rendering (:385-394), Musik plus Crackle/Rumble (:437-439); `decode.rs`: Symphonia→48 kHz (decode.rs:54,131).
- `audio/timeline.rs`: Seitenzeit und Track-Spans (timeline.rs:34-97); `audio/groove.rs`: Rillenbänder in mm (groove.rs:1-8, `for_side` :92); `audio/noise.rs`: nur noch Crackle/Rumble, `process` :32, Crackle :41-53, Rumble :55-58; `audio/resample.rs`: Catmull-Rom/Varispeed (resample.rs:7,55).
- `audio/output.rs`: cpal-Stream bzw. Linux-Nullsink (output.rs:14, 96).
- `playback/geometry.rs`: Tonarmgeometrie, `cos(θ)=(d²+L²−r²)/(2dL)` (geometry.rs:6-7), `angle_for_radius` (:110); `playback/mod.rs` ist nur ein Doc-Stub (playback/mod.rs:1).
- `src/routes/+page.svelte`: Layout, Auswahl, Tastatur, Polling, Cue-Sequenz (src/routes/+page.svelte:18-24, 81-137, 150-215).
- `src/lib/components/Crate.svelte`: Kiste, Suche, Sortierung, Pager, 3×3 (Crate.svelte:6-36, 48-52, 139-165).
- `Sleeve.svelte`: quadratische Hülle, Test-Pressing-Stempel (Sleeve.svelte:8, 25-47, 59-60), Wear/Spine/Grain/Shine (:49-53).
- `Deck.svelte`: Teller, Tonarm, Ziehen und Cue (Deck.svelte:57-133, 243-320, 326-440), runder Teller (:814-815).
- `Vinyl.svelte`: Rillen-Canvas, Label, Auflege-Animation `vinyl-in` (Vinyl.svelte:65, 157-219, 542-554); `Gatefold.svelte`: Cover groß, Vorder-/Rückseite (Gatefold.svelte:12-38, 54-136, 68-95); `WoodBackground.svelte`: Holz + SVG-Korn (WoodBackground.svelte:6-10, 23-38).
- `src/lib/types.ts`: IPC-Typen (types.ts:1-92); `library.svelte.ts`/`playback.svelte.ts`: Stores + Invokes (library.svelte.ts:5-32, 34-68, 70-78; playback.svelte.ts:4-9, 11-71, 73-77); `groove.ts`: Radius↔Winkel/Cue (groove.ts:4-13, 22-48).

## 4. Datenfluss
1. Ordnerwahl oder Rescan → `library_scan` mit Ordnerliste (library.svelte.ts:34-68; lib.rs:52-74).
2. `collect_files` trennt Dateien nach `.cue`, `.m3u/.m3u8` und Audio-Endungen (scanner.rs:135-168).
3. Reihenfolge: CUE-Alben, dann Tag-Gruppen, dann Playlists (scanner.rs:83-122); `group_key` nutzt Album-Artist+Album, sonst Pfad (scanner.rs:170-185).
4. `build_from_files` liest Cover (eingebettet vor Ordner) und jüngste mtime (scanner.rs:262-270); `assemble` verwirft Tracks ≤10 ms, baut Format, IDs und Seiten (scanner.rs:549, 566-620).
5. Playlist: `m3u::parse` (m3u.rs:28), relative Pfade ab Playlist-Ordner, `#EXTINF`-Titel/-Dauer vor Tags (scanner.rs:291-355); Label „Test Pressing“ (scanner.rs:389), immer 12″ (scanner.rs:576-577). Cover: `cover.jpg/png` neben der `.m3u` gewinnt, sonst eingebettetes Bild des ersten Tracks, sonst dessen Ordner-Cover (scanner.rs:375-381; artwork.rs:24-35).
6. DB: `replace_albums` schreibt Palette/Format/Sides/Source als JSON (db.rs:106-110), `load_albums` liest sie mit Fallbacks (db.rs:124-140); Spalte `source` default `"release"` inkl. ALTER-Migration (db.rs:62, 72-84).
7. IPC: `library_load` liefert Alben, `playback_*` steuert die Session (lib.rs:34-142); Frontend pollt alle 250 ms (src/routes/+page.svelte:81-90).
8. Audio: `playback_load` holt Album/Seite aus der DB (lib.rs:76-101); Conductor prüft alle 120 ms und prefetcht den Folgetrack (session.rs:116-169); Engine rendert, Ausgabe cpal/Nullsink (engine.rs:311, output.rs:96).
9. Tonarm: Groove-Bänder → Radius, `cuePointForRadius` → Zeit (groove.ts:22-47); Ziehen setzt Nadel und ruft `playback_cue` (Deck.svelte:286-320).
10. Auswahl: Klick auf die Hülle lädt nur die Seite (src/routes/+page.svelte:150-157), kein Hüllen-Flug; die Platte erscheint mit dem alten kurzen `vinyl-in`-Übergang, neu getriggert per `{#key}` (Deck.svelte:363-366; Vinyl.svelte:219, 542-554).
11. Start-Sequenz: ein START-Klick während `loading` wird in `toggle` als `startPending` gemerkt und per `$effect` nachgeholt, sobald die Seite bereit ist (src/routes/+page.svelte:173-196); der Start-Knopf bleibt dazu auch beim Laden aktiv (Deck.svelte:711). An Anfang/Ende `cuePhase="swing"` 1400 ms → `"lower"` 450 ms → `togglePlayback` (src/routes/+page.svelte:202-215); Deck hebt den Arm (`armLift` :123), Ziel `cueTargetPhi` (:116-121), legt bei `"lower"` auf (:180-184). Engine startet dann einen zufälligen Einlauf von 2–4 s: `CueVoice::start` wählt Startposition (bis 40 % der 12-s-Statik-Aufnahme), Rate (0,96–1,04 bzw. 0,98–1,02) und Gain (0,78–1,08 Nadel, 0,78–1,06 Statik) neu (cue.rs:140-161); gerendert werden echter Nadel-Aufschlag plus Oberflächenstatik mit 20 ms Fade-in (cue.rs:164-185; engine.rs:385-394). Die Position bleibt stehen, danach folgt Musik mit prozeduralem Crackle/Rumble (engine.rs:341-357; noise.rs:41-58; aktiviert in session.rs:96).

## 5. Typen und Einstiegspunkte
- `Album`, `Side`, `Track`, `Palette`, `PlaybackState` sind 1:1 camelCase-DTOs (model.rs:7-27, 55-120; session.rs:23-45; types.ts:6-92).
- `AlbumSource = release | playlist` (model.rs:47-53; types.ts:3,43); fehlendes Feld wird `release` (model.rs:20-22, 165-186).
- Rust-Einstieg: `main()` → `longplay_lib::run()` (main.rs:5-6, lib.rs:145); UI-Einstieg: `+page.svelte` mit `loadLibrary()` (src/routes/+page.svelte:81-82).
- `DiscFormat` steuert 7″/12″ und RPM (model.rs:37-45, 62-84; scanner.rs:566-581).

## 6. Konfiguration / ENV
- Fenster 1280×900, Overlay-Titelleiste, Dark, Hintergrund `#1b100a` (tauri.conf.json:15-24).
- Asset-Protokoll für `$APPCACHE/**`; Cover liegen in `app_cache_dir/covers` (tauri.conf.json:29-34; lib.rs:150-152).
- DB: `app_data_dir/library.sqlite3`, WAL (lib.rs:155-156; db.rs:47).
- `LONGPLAY_SCAN_DIR` scannt beim Start (Pfadtrenner via `split_paths`) (lib.rs:160-170; README.md:78-81).
- Keine CSP gesetzt (tauri.conf.json:28); Font Caveat lokal (theme.css:1-7), Texturen in `static/textures/` (README.md:76).
- Ton-Assets sind einkompiliert (`include_bytes!`, cue.rs:35-38), keine Tauri-Resources; Lizenztexte in `src-tauri/assets/audio/CREDITS.md` (README.md:121-130).

## 7. Gotchas
- Linux baut ohne cpal: ein Thread taktet die Engine als Nullsink, es kommt kein Ton heraus (output.rs:5-6, 95-96).
- Der WAV-Parser akzeptiert nur PCM (Format 1) mit 16 Bit; alles andere fällt still auf Stille zurück (cue.rs:43-90).
- Format ist nur Heuristik: ≤2 Tracks und ≤480 s → 7″ (scanner.rs:566-571); echte `RELEASETYPE`/`MEDIA`-Tags fehlen (PLAN.md:501-503).
- Playlist-Tracks werden nicht aus den Release-Gruppen entfernt (`consumed` füllt nur CUE); derselbe Ordner erscheint zusätzlich als Album (scanner.rs:81-108).
- `source` wird als JSON-String gespeichert, nicht als nackter Text (db.rs:110, 139).
- Seitensplit: explizite Hinweise (CUE-Seite, Disc, `A1`-Präfix) gewinnen, sonst balancierte Hälften (scanner.rs:661-687, 689-704).
- Hüllen sind quadratisch (`aspect-ratio: 1`), Platte/Teller rund (Sleeve.svelte:59-60; Deck.svelte:814-815; Vinyl.svelte:199-203).
- Papierkante an Shelf-Hüllen ist entfernt; sichtbar sind nur Wear, Spine, Grain und Shine (Sleeve.svelte:49-53).
- `prefers-reduced-motion` deaktiviert global alle Animationen/Transitions, damit auch `vinyl-in` (theme.css:63-70).
- Start-Sequenz läuft nur an Anfang/Ende (Position ≤0,05 s bzw. letzte 0,08 s); Pause/Play sonst sofort; ein während `loading` geklickter START wartet und startet nach dem Laden selbst (src/routes/+page.svelte:173-215).
- Der Start-Knopf ist während `loading` nicht deaktiviert, damit der erste Klick nicht verschluckt wird (Deck.svelte:711; src/routes/+page.svelte:189-193).
- Der Einlauf kommt nur, wenn die Seite an Position 0 beginnt; Mitten-Cue und Seek überspringen ihn (engine.rs:341-357).
- Playlist-Hüllen zeigen bei fehlendem Cover den Stempel „Test Pressing“, sonst das Cover mit Badge (Sleeve.svelte:8, 25-38; Gatefold.svelte:68-95).

## 8. Tests
- 118 `#[test]`-Attribute in 17 Dateien (18 `#[cfg(test)]`-Blöcke, scanner.rs hat zwei) (z. B. m3u.rs:67-197, scanner.rs:755, db.rs:200).
- m3u: 16 Tests zu BOM, CRLF, `#EXTINF`, Kommentaren, Windows-Pfaden (m3u.rs:80-197).
- scanner: Playlist → Testpressung inkl. fehlender Einträge und Cover (scanner.rs:978-1029); 10 Tests gesamt (scanner.rs:755, 845).
- audio/cue: 3 Tests zu Dekodierung, Variation zwischen Starts und Ende des Einlaufs (cue.rs:224-255); engine: 20 Tests inkl. Pre-Roll (engine.rs:463, 740-793).
- DB nutzt In-Memory-SQLite (db.rs:37-43); Session-Tests schreiben WAV-Dateien (session.rs:449, 545).
- Kein Frontend-Testskript; statischer Check ist `npm run check` (package.json:11).
