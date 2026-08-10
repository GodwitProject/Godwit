# Godwit UI

Single-page application for Godwit admin and user console.

## Setup

```bash
cd apps/ui
npm install
```

## Development

```bash
npm run dev
```

The dev server proxies `/api`, `/health`, `/metrics` to the backend at `VITE_API_URL` (default: `http://localhost:3000`).

## Build

```bash
npm run build
npm run preview
```

## Tests

```bash
npm run test
npm run test:watch
```
