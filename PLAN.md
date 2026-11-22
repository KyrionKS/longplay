# Longplay — Implementierungsplan

Ein skeuomorpher Plattenspieler für die eigene Musiksammlung auf macOS.
Arbeitstitel: **Longplay** (Namenskollision mit longplay.app beachten, finaler Name vor Phase 5).

**Status 2026-11-22:** Phase 0 bis 2 umgesetzt. Nächster Schritt: Phase 3 (Turntable-Interaktion).

---

## 1. Ziel

Die Musikbibliothek liegt als Dateien auf dem Mac oder NAS. Longplay zeigt sie als
Schallplatten in einer Holzkiste und spielt sie auf einem Direct-Drive-Plattenspieler ab:
Tonarm von Hand aufsetzen, 33⅓/45 umschalten, Alben wie echte LPs in Seite A und B teilen
und wenden. Knistern, Rumpeln und Nadelphysik gehören dazu — subtil, nicht als Effektgerät.
Scratchen ist bewusst kein Feature; im Mittelpunkt steht das Vinyl-Erlebnis.

**Leitplanken**

- Lokale Dateien, keine Accounts, kein Netzwerk, keine Telemetrie.
- Der Rust-Kern macht die Arbeit; die Oberfläche ist eine dünne Schicht.
- Nativ genug für macOS: eigene Titlebar, Menübar, Media-Keys, Tastatur.
- Deutsche UI.
- Keine DRM-Inhalte, keine Cloud-Bibliothek.

## 2. Scope

### V1 — drin

| Bereich | Umfang |
|---|---|
| Bibliothek | Ordner hinzufügen, rekursiv scannen, Alben gruppieren, Cover, Farbpalette, Suche, Sortierung, inkrementeller Rescan |
| CUE | `.cue` lesen, Single-File- und Multi-File-Alben, Pregaps, Seitenhinweise |
| Seiten | A/B pro Album (generisch N Seiten), Standard = manuell wenden, Auto-Flip optional |
| Audio | FLAC/ALAC/AAC/MP3/WAV/AIFF, Varispeed, Cueing, Pregap-Stille, Song-für-Song-Laden |
| Vinyl | Crackle, Rumble, Nadel-Fall, Bürste, Wow/Flutter, Innenrillen-Charakter, Übersprechen |
| Optik | Holzkiste, Sleeves, Plattenspieler, Tonarm, Label, Gatefold, White-Label-Playlists, 7″-Singles |
| Mac | Menübar-Mini-Player, Media-Keys/Now Playing, Shortcuts, Einstellungen, `.app`/DMG |

### V1 — draußen

Apple Music/MusicKit, Spotify, Streaming, App Store, Notarisierung, CD-Rip, EQ,
Visualizer, Lyrics/Liner-Notes-Generierung, Netzwerkfunktionen, 2LP mit mehr als zwei
Seiten (Datenmodell kann es, UI erst später).

## 3. Entscheidungen

| Thema | Entscheidung |
|---|---|
| Sprache | Rust für alles außer UI |
| Hülle | Tauri 2 (WKWebView), Svelte 5 + TypeScript |
| Musikquelle | lokale Dateien, mehrere Wurzelordner |
| Pausen | Album 1,5 s; Playlist 0 s; CUE-Pregap hat Vorrang |
| Seiten | Standard manuell; `autoFlip` optional, Default aus |
| RPM | Album 33⅓, Single 45; Umschalter verändert Tonhöhe |
| Vinyl-Charakter | Regler 0–100 %, Default 30 % |
| Liner Notes | nur echte Metadaten, kein erfundener Text |
| Name | Arbeitstitel Longplay |
| Plattformen | macOS primär, Linux nur Entwicklung |

## 4. Architektur

```
┌──────────────────────────── Frontend (Svelte 5) ────────────────────────────┐
│  Crate  ·  Turntable  ·  Gatefold  ·  Settings  ·  MiniPlayer               │
│  Stores: library, playback, physics (rAF, feste Substeps)                   │
│  Rendering: SVG/CSS für Szene, Canvas pro Platte, CSS-Transform für Rotation │
└───────────────────────────────┬─────────────────────────────────────────────┘
                                │ Tauri IPC (Commands + Events)
┌───────────────────────────────▼─────────────────────────────────────────────┐
│                              Rust-Kern                                      │
│  library/  scanner · metadata · artwork · palette · cue · db                │
│  playback/ timeline · queue · tonearm · state machine                       │
│  audio/    engine · decoder · resampler · noise · clock                     │
│  platform/ now_playing · tray · settings                                    │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Modulverantwortung

- `library::scanner`: Ordner rekursiv lesen, Dateien erkennen, Alben bilden.
- `library::metadata`: `lofty`-Tags, Dauer, Cover, Comment.
- `library::cue`: CUE parsen, Tracks und Pregaps, Seitenhinweise.
- `library::palette`: Farben aus Cover für Vinyl, Label, Sleeve.
- `library::db`: SQLite, inkrementeller Rescan über `mtime`, Suche (FTS5).
- `playback::timeline`: Album → Seiten → Tracks mit absoluten Zeiten.
- `playback::tonearm`: Zeit ↔ Radius ↔ Winkel, Cueing, Auto-Return.
- `playback::state`: Zustandsmaschine für Teller, Nadel, Seite.
- `audio::engine`: cpal-Stream, Callback, Pegel, Limiter.
- `audio::decoder`: symphonia im Worker-Thread, Ringpuffer.
- `audio::resampler`: fraktionaler Catmull-Rom, vorzeichenbehaftete Rate.
- `audio::noise`: prozedurale Vinyl-Geräusche.
- `platform`: Menübar, Now Playing, Media-Keys.

## 5. Datenmodell

```rust
struct Album {
    id: String,              // stabil: blake3 aus Wurzel + Album-Schlüssel
    title: String,
    album_artist: String,
    year: Option<u16>,
    genre: Option<String>,
    label: Option<String>,
    added_at: i64,
    cover: Option<PathBuf>,  // gecachte, skalierte Kopie
    palette: Palette,
    format: DiscFormat,      // Lp12 | Single7 | Unknown
    sides: Vec<Side>,
}

struct Side {
    label: char,             // 'A', 'B', …
    rpm: Rpm,                // 33 | 45
    matrix: Option<String>,  // z. B. LP-001-A
    tracks: Vec<Track>,
}

struct Track {
    id: String,
    number: u32,
    title: String,
    artist: String,
    duration: f64,
    pregap: f64,             // Stille vor diesem Track
    source: TrackSource,     // File{path,offset,len} | Cue{path,start,end}
    side_index: usize,
}

struct Palette { vinyl: Rgb, vinyl_light: Rgb, label: Rgb, accent: Rgb, dark: Rgb }
```

SQLite-Tabellen: `albums`, `sides`, `tracks`, `folders`, `playlists`,
`playlist_items`, `settings`, `scan_errors`, `side_overrides`.

## 6. Bibliothek

### Scan

1. Wurzelordner aus Einstellungen, `walkdir` mit Ignore für versteckte Ordner.
2. Erweiterungen: `flac`, `m4a`, `alac`, `aac`, `mp3`, `wav`, `aiff`, `aif`, `ogg`, `opus`.
3. `lofty` lesen: Titel, Artist, Album, AlbumArtist, Track, Disc, Jahr, Genre, Comment,
   Dauer, Cover (embedded), `mtime`, Dateigröße.
4. Gruppierung: `AlbumArtist + Album`; fehlt eins davon, Ordnerpfad.
5. Sortierung: Disc, Tracknummer, Dateiname.
6. Cover: embedded → `cover.jpg`/`folder.jpg`/`front.jpg` → Platzhalter aus Palette.
7. Palette: dominant + vibrant + dunkel/hell; Vinyl = dominant mit Helligkeitskorrektur,
   Label = cremig abgetönt, Akzent = vibrant.
8. Inkrementell: Datei unverändert (`mtime` + Größe) → Tags aus DB, kein erneutes Parsen.

### CUE

Unterstützt Single-File (`album.flac` + `album.cue`) und Multi-File.

- Encoding: UTF-8, UTF-8-BOM, Fallback Windows-1252.
- Direktiven: `FILE`, `TRACK`, `INDEX 00`, `INDEX 01`, `TITLE`, `PERFORMER`, `SONGWRITER`,
  `REM` (u. a. `REM SIDE`, `REM DISCNUMBER`, `REM DATE`, `REM GENRE`).
- Zeitformat `mm:ss:ff` mit 75 Frames/s.
- Pregap = `INDEX 01 − INDEX 00`; ohne `INDEX 00` = 0.
- Track-Ende = nächster `INDEX 00` oder Dateiende.
- Seiten: `REM SIDE A` > Discnummer > Track-Präfix `A1` > Dateiname > manuell > Heuristik.
- Fehler (fehlende Datei, Offset hinter Dateiende, doppelte Nummern) landen in `scan_errors`,
  das Album bleibt abspielbar.

### Seiten-Heuristik (letzte Stufe)

Track-Grenze, die der halben Gesamtdauer am nächsten liegt; 12″ max. 22 min/Seite,
7″ max. 5 min/Seite. Manuelles Override im UI hat immer Vorrang und wird in
`side_overrides` gespeichert.

## 7. Audio-Engine

### Ausgabe

- `cpal`, Standard-Ausgabegerät, f32 stereo, bevorzugt 48 kHz, 256–512 Frames.
- Gerätewechsel (Kopfhörer) → Stream neu aufbauen, Position halten.
- Linux-Dev: cpal hinter Feature `audio-device`; Tests laufen ohne ALSA-Header.

### Datenfluss

```
Datei → symphonia (Thread) → Track-Puffer 48 kHz (inkl. Pregap) → Resampler (Callback) → Noise-Mix → Limiter → Gerät
```

- **Song für Song:** es wird nur der aktuelle Track gehalten; der nächste wird im
  Hintergrund vorgeladen. Das ist NAS-tauglich und begrenzt den Speicher auf zwei
  Tracks (6 min ≈ 130 MB pro Track bei 48 kHz stereo f32).
- Der Track-Puffer beginnt mit seiner Pregap-Stille; dadurch bleibt die Seite
  lückenlos und der Tonarm kann durch die Pause wandern.
- Auto-Advance: am Track-Ende tauscht der Callback auf den vorbereiteten Puffer,
  ohne Loader und ohne Klick (20 ms Fade). Ist kein nächster Track bereit, endet die Seite.
- Seek innerhalb des Tracks ist sample-genau; ein Seek in einen anderen Track lädt
  diesen kurz nach (Status `loading`).
- Seitenende: letzter Track endet → Nadel hebt ab → Auto-Return → `side-ended`.

### Parameter (Defaults)

| Parameter | Wert |
|---|---|
| Track-Puffer | 1 Track + 1 Prefetch, 48 kHz |
| Buffer | Gerätestandard (CoreAudio) |
| Rate-Clamp | ±3,0 |
| Wow | 0,55 Hz, ±0,15 % |
| Flutter | 6,3 Hz, ±0,05 % |
| Rumble | Brown Noise, LP 40 Hz |
| Crackle | 8 Ereignisse/min, BP 2–8 kHz |
| Übersprechen | −30 dB |
| Innenrille | ab 85 % der Seite, HF-Abfall + Sättigung |
| Vinyl-Charakter | 0,0–1,0, Default 0,3 |

### Verifikation

Offline-Renderer: Script aus `Seek`, `Rate`, `Frames` rendert nach WAV und liefert
Kennzahlen. Tests prüfen:

- Seek-Genauigkeit ±20 ms
- keine Sample-Sprünge > Schwelle an Ratenwechseln/Reversals
- Pregap-Längen exakt (1,5 s bzw. CUE)
- kein Clipping bei Vollaussteuerung
- Seitenwechsel ohne Klick

## 8. Interaktion

### Zustände

```
Empty → Loaded(side) → Cueing → Playing ⇄ Paused
                              ↓
                         SideEnded → Flipping → Loaded(other)
```

Teller: `Stopped → SpinningUp → AtSpeed → CoastingDown`.
Nadel: `Rest → Lifted → OnRecord`.

### Tonarm-Geometrie

Spindel im Ursprung, Pivot bei `(d, 0)`, effektive Länge `L`, Rillenradius `r`:

```
cos θ = (d² + L² − r²) / (2·d·L)
r = r_außen − (r_außen − r_innen) · fortschritt
```

Defaults: `d = 211 mm`, `L = 230 mm`, `r_außen = 146 mm`, `r_innen = 60 mm`.
Fortschritt linear in der Zeit, Track-Bänder aus den Dauern.

### Nadelphysik

- 2-DOF-Feder-Dämpfer, Resonanz ~10 Hz, ζ ≈ 0,3.
- Auflagekraft/Antiskating als leichte Einwärts-Vorspannung.
- Aufsetzen: Cantilever 1–2 px, zwei Nachschwingungen, ~120 ms.
- Warp: 0,5–2 Hz, minimale vertikale Bewegung des Arms.

### Cueing (bewusst kein Scratch)

Scratchen ist nicht im Scope. Der Tonarm wird gezogen, um zu cueen:

- Arm anheben, über die Platte bewegen, auf einer Rille absetzen; Audio bleibt bis zum
  Aufsetzen stumm.
- Während des Ziehens folgt der Arm dem Zeiger, die Nadel bleibt angehoben
  (Cantilever-Physik).
- Aufsetzen: kurzer Nadel-Fall-Thump, dann Wiedergabe ab der Position.
- Seitenende: Auslaufrille mit Locked-Groove-Thump, Auto-Return in die Ablage.

## 9. Optik

### Layout (1280 × 900, min. 1100 × 760)

- Links 40 %: Holzkiste mit 3×3 Sleeves, Messingplakette mit Sortierung, Suche, Seitenpunkte.
- Rechts 60 %: Plattenspieler auf Walnuss, eingravierte Modellzeile, Teller, Tonarm,
  Start/Stop, 33/45, Anti-Static.
- Titlebar transparent, Traffic-Lights über dem Holz, Drag-Region.

### Platte

Pro Album einmal auf Canvas vorgerendert, dann nur rotiert:

- Einlaufrille, Track-Bänder mit echten Pausengrößen, Auslaufrille, tote Rille mit Matrix.
- Transluzenz, Kante, Glanz, Staub, deterministische Haarkratzer (Seed = Album-ID).
- Label mit Albumtitel, Künstler, Seite, Trackanzahl, Dauer, RPM, Katalognummer.
- Singles: 7″, größeres Loch, 45 rpm.

### Seitenwechsel

3D-Flip der Platte (CSS `rotateY`), Label wechselt auf B, Rillenlayout wird getauscht,
Tonarm schwenkt in die Ablage. Dauer ~600 ms, unterbrechbar.

### Gatefold

Buch mit Papierstruktur: links Cover + Metadaten (Released, Genre, Label, Laufzeit,
Pressung), rechts Tracklist mit Punktführung, aktueller Track eingekreist, Blöcke
„Seite A“/„Seite B“. Klick auf Track = Seek. Kein erfundener Liner-Text.

### Playlists

White-Label-Testpressung: weißes Label, handgeschriebene Anmutung, Tracklist als Stempel.
Pausen 0.

### Vinyl-Ästhetik (Vorschlag)

Priorisiert nach Wirkung, nicht nach Aufwand:

1. **Tonarm in der Rille:** Winkel folgt der Wiedergabe, Nadel sitzt sichtbar im Band.
2. **Material:** Label-Abrieb am Rand, leicht vergilbtes Papier, Ring-Abdruck des
   Vinyls auf der Hülle, abgestoßene Sleeve-Ecken.
3. **Tote Rille:** Matrix-Nummer und Presswerks-Kürzel eingeritzt, Staub und feine
   Haarkratzer deterministisch pro Album.
4. **Bewegung:** Teller-Anlauf/Auslauf mit Trägheit, 20-ms-Nadel-Absetzen, Warp-Reflex,
   die mit der Drehung wandert.
5. **Licht:** ein beweglicher Glanzstreifen, Staub im Lichtkegel, weicher Schatten
   unter dem Arm.
6. **Hülle → Teller:** Sleeve hebt sich, Platte gleitet heraus und sinkt auf den Teller.
7. **Track-Hover:** Zeiger über einem Rillenband zeigt den Tracktitel als geätzte
   Beschriftung; Klick cueed dorthin.
8. **Ambiente:** 7″-Single kleiner als 12″, großes Loch, 45-Adapter; Gatefold mit
   echten Metadaten.

## 10. macOS

- Fenster: Overlay-Titlebar, `hiddenTitle`, dunkles Theme, Größe persistieren.
- Menübar: Tray-Icon, Menü mit Play/Pause, Next, aktuellem Track; Linksklick toggelt Fenster.
- Now Playing + Media-Keys über `souvlaki`.
- Shortcuts: Space, ←/→, ↑/↓, ⌘L (Crate), Esc, ⌘O (Ordner), ⌘, (Settings).
- Einstellungen: Ordner, Pausen, Auto-Flip, Vinyl-Charakter, Crackle, Ausgabegerät.
- Packaging: `npm run tauri build`, Ad-hoc-Signatur, DMG. Keine Notarisierung in V1.

## 11. Phasen

### Phase 0 — Skelett (erledigt, wenn alles unten grün ist)

- [x] Tauri 2 + SvelteKit + TypeScript scaffold
- [x] Fensterkonfiguration (1280×900, min. 1100×760, Overlay-Titlebar, dunkel)
- [x] Theme-Tokens, Holzhintergrund, Kiste, deutsche Texte
- [x] Rust-Modulskelett mit Tonarm-Geometrie + Tests
- [x] `cargo test`, `npm run check`, `npm run build`
- [x] README, PLAN.md, Git, öffentliches Repo

**Abnahme:** App startet (Linux verifiziert, macOS offen), zeigt die Kiste, Tests grün.

### Phase 1 — Bibliothek

- [x] `cue.rs` + 21 Tests (Single-File, Multi-File, Encoding, Fehlerfälle)
- [x] `metadata.rs`, `scanner.rs`, Gruppierung + Tests
- [x] SQLite-Schema, voller Rescan (inkrementell über `mtime` offen)
- [x] Cover-Cache (512 px JPEG), Palette
- [x] Seiten-Erkennung (CUE, Disc, `A1`-Präfix, Heuristik)
- [ ] Manuelles Seiten-Override im UI (Phase 3)
- [x] Crate-Grid mit echten Daten, Suche, Sortierung, Seiten
- [x] Scan-Status im UI; Fortschritt pro Datei offen

**Abnahme:** Funktional erfüllt (3-Alben-Fixture, End-to-End-Test). Die 50-Alben-
Messung auf einer echten Bibliothek steht noch aus; kaputte Tags landen als Scan-Issue.

### Phase 2 — Audio

- [x] Decoder, Resampler, Offline-Tests (24 Audio-Tests)
- [x] cpal-Ausgabe (macOS/Windows), Linux-Null-Sink für Development
- [x] Timeline inkl. Pregaps, Trackwechsel, Seitenende
- [x] Wow/Flutter, Innenrille, Übersprechen, Crackle, Rumble, Nadel-Fall
- [x] Rate/Volume/Character über Atomare, RPM über IPC

**Abweichung vom Plan:** Statt eines 45-s-Ringpuffers wird **ein Track** (plus
Prefetch des nächsten) geladen. Das hält NAS-Traffic und Speicher klein; ein
Track-Puffer enthält seine Pregap. Seek im Track ist sample-genau, ein
Track-Wechsel lädt kurz nach. 96-kHz-Interfaces werden erst in der Wiedergabe
hochgerechnet, die Puffer bleiben bei 48 kHz.

**Abnahme:** Album läuft, Seek sample-genau, Ratenwechsel ohne Klicks, Seitenende
stoppt. Auf macOS/CPU noch nicht gemessen; Linux-Dev nutzt einen Null-Sink.

**Ausgabe:** Standardgerät, kein Picker. Alle Kanäle werden akzeptiert; Stereo geht
auf die ersten beiden Ausgänge, weitere Kanäle bleiben still. Mono wird heruntergemischt.

### Phase 3 — Turntable

- [x] Rillen-Layout pro Track, Pausen und Auslaufrille (Canvas, vorgezogen)
- [ ] Tonarm folgt dem Rillen-Layout (Winkel aus der Side-Position)
- [ ] Teller-Physik: Anlauf, Auslauf, Start/Stop
- [ ] Tonarm ziehen und cueen (ohne Scratch), Nadel-Fall
- [ ] Auto-Return und Locked-Groove am Seitenende
- [ ] 33/45 (Tonhöhe und Tempo)
- [ ] Seiten-Zustandsmaschine + Flip-Animation
- [ ] Ästhetik-Pass (siehe Abschnitt 9)

**Abnahme:** Armwinkel und Rillenposition stimmen überein, Cueing sitzt auf dem
gewählten Track, Seitenwechsel und Flip laufen rund, 60 fps. Kein Scratch.

### Phase 4 — Optik

- [ ] Texturen (Holz, Metall, Papier), Licht, Vignette
- [ ] Platten-Renderer, Label, Glitzer, Kratzer
- [ ] Tonarm, Knöpfe, Bürste
- [ ] Gatefold, Playlists, Singles
- [ ] Animationen, Reduced Motion

**Abnahme:** Vergleich mit Referenz-Screenshots, 60 fps (Frame-Zeiten gemessen).

### Phase 5 — Mac-Finish

- [ ] Tray, Now Playing, Media-Keys
- [ ] Shortcuts, Settings, Persistenz
- [ ] Fehlerzustände, leere Bibliothek, DRM-Hinweis
- [ ] README, DMG, ad-hoc signiert
- [ ] Namensentscheidung

**Abnahme:** App startet aus dem Finder, Media-Keys und Settings funktionieren.

## 12. Teststrategie

- **Unit:** CUE, Gruppierung, Seiten-Heuristik, Tonarm-Geometrie, Resampler, Palette.
- **Offline-Audio:** deterministische Renderings, Kennzahlen statt Ohren.
- **Property:** Rate/Seek-Kombinationen bleiben im gültigen Bereich.
- **Frontend:** `svelte-check`, Build; später Vitest für Stores.
- **Manuell (Mac):** Cueing, Klang, 60 fps, Media-Keys, Seitenwechsel, Ästhetik.

Regel: Kein „getestet“ ohne ausgeführten Befehl. Von Agenten geschriebene Tests werden
per Mutationscheck geprüft.

## 13. Risiken

| Risiko | Gegenmittel |
|---|---|
| Tonarm trifft die Rille nicht exakt | ein gemeinsames Rillen-Layout für Audio und Optik, Tests auf Winkel/Radius |
| WebView-Performance | Platte vorrendern, Rotation per CSS, Frame-Zeiten messen |
| Tag-Chaos | Fallbacks, Scan-Fehler sichtbar, später manuelles Editieren |
| Optik wirkt billig | Texturen früh festlegen, notfalls stilisiert statt halb fotorealistisch |
| Gerätewechsel | Device-Lost-Handling, Position halten |
| Linux ohne ALSA-Header | cpal hinter Feature, Tests ohne Ausgabe |
| DRM-Dateien | erkennen, überspringen, klarer Hinweis |

## 14. Entwicklung

```bash
# Linux
npm install
npm run tauri dev          # Dev-Port 1430; Velox/Paceboard belegen 1420
cargo test --manifest-path src-tauri/Cargo.toml
npm run check
npm run build

# macOS
npm run tauri build        # .app + DMG
```

Systempakete Linux: `libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev
libayatana-appindicator3-dev libasound2-dev` (letzteres erst ab Phase 2).

## 15. Konventionen

- UI-Texte deutsch, Code/Kommentare englisch.
- Keine `#[allow]`-Unterdrückung, kein `unsafe` ohne dokumentierte Begründung.
- Fehler mit `thiserror`, an der IPC-Grenze in `{ code, message }` übersetzen.
- Beträge in Sekunden als `f64`, Sample-Positionen als `u64`, Winkel in Radiant.
- Jede Phase endet mit grünen Tests und einem Commit.
