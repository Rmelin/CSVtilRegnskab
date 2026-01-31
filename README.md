# Foreningsregnskab

Lokal desktop-app til import af banktransaktioner, kontering og rapportering.

## Stack
- Tauri v2 + Rust (Tauri commands)
- React + Vite + TypeScript
- SQLite via sqlx + migrations

## Struktur
- `src-tauri` Rust backend
- `frontend` React UI
- `src-tauri/migrations` SQLX migrations

## Kørsel (Mac og Linux)

### Forudsætninger
- Rust (stable) + `cargo`
- Node.js 18+
- Tauri CLI (`cargo install tauri-cli`)

### Dev
```
cd frontend
npm install
npm run dev
```

I et andet terminalvindue:
```
cd src-tauri
cargo tauri dev
```

### Build
```
cd frontend
npm install
npm run build
```

```
cd src-tauri
cargo tauri build
```

### Database
SQLite-filer ligger pr. klub i `~/Library/Application Support/forening-regnskab/clubs/<slug>/forening_regnskab.sqlite` (macOS)
eller `~/forening-regnskab/clubs/<slug>/forening_regnskab.sqlite` hvis HOME ikke kan læses.

### Ny Linux-maskine
1) Installer systempakker (Debian/Ubuntu):
```
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev libgtk-3-dev \
  libayatana-appindicator3-dev libwebkit2gtk-4.1-dev librsvg2-dev
```
2) Installer Rust:
```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```
3) Installer Node 18+ (fx via nvm) og afhængigheder:
```
cd frontend
npm install
```
4) Installer Tauri CLI:
```
cargo install tauri-cli
```
5) Start appen:
```
cd src-tauri
cargo tauri dev
```
6) Vælg/opret klub i UI ved første start.

## PDF-rapport
PDF genereres via headless Chrome/Chromium og gemmes i systemets temp-mappe. Rapporten indeholder totaler,
budgetgrupper og et bilag med alle transaktioner.

## Saldo-graf
- Grafen bruger `Dato` (booking_date) og `Saldo` fra CSV (ikke Valør).
- År vælges i UI, og der genereres ét punkt pr. dag for hele året.
- Dage uden transaktioner bruger seneste kendte saldo.
- Startsaldo findes som seneste saldo før 01/01, ellers 0.

## Tests
Rust tests ligger i `src-tauri/src/parsing.rs` og `src-tauri/src/matcher.rs`.
Kør tests:
```
cd src-tauri
cargo test
```
