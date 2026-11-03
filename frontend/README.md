# Roubit — Frontend SPA & PWA

> **Progressive Web Application (PWA)** built with **Vue 3**, **TypeScript**, **Vite**, **Pinia**, and **TailwindCSS**. Features offline-first mutation queues, client-side zero-knowledge encryption with Argon2id + AES-256-GCM, real-time SSE notifications, and distraction-free Focus Mode with Web Audio synthesis.

[![Vue 3](https://img.shields.io/badge/Vue.js-3.5%2B-4FC08D?logo=vuedotjs)](https://vuejs.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.0%2B-3178C6?logo=typescript)](https://www.typescriptlang.org/)
[![Vite](https://img.shields.io/badge/Vite-8.3%2B-646CFF?logo=vite)](https://vitejs.dev/)
[![TailwindCSS](https://img.shields.io/badge/TailwindCSS-3.4%2B-38B2AC?logo=tailwind-css)](https://tailwindcss.com/)
[![PWA Ready](https://img.shields.io/badge/PWA-Workbox%20%26%20IndexedDB-5A0FC8?logo=pwa)](https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps)
[![Tests](https://img.shields.io/badge/Vitest-80%2F80%20Passing-success)](https://vitest.dev/)

---

## 🌟 Key Features

1. **Progressive Web App (PWA) & Offline First (Feature 16):**
   - Service Worker precaching of all critical assets via `vite-plugin-pwa` and Workbox.
   - Installable Web App Manifest (`/manifest.webmanifest`) for standalone desktop and mobile home screen installation.
   - Monotonic FIFO Offline Mutation Queue in **IndexedDB** (`roubit_offline_db`) with automatic idempotent drain upon network reconnection.
   - Native **App Badge API** integration dynamically displaying pending uncompleted daily habits.
   - Live network status detection (`Online` / `Offline`) with pending sync counter.

2. **Client-Side Zero-Knowledge Encryption (RF04 / Feature 02 & 09):**
   - Personal reflections encrypted in the browser with **AES-256-GCM** using keys derived via **Argon2id WASM** (`hash-wasm`).
   - Strict zero-knowledge invariant: plaintext never touches network payloads or persistent storage.
   - Volatile memory scrubbing when the cryptographic vault is locked.

3. **Time-Blocking & Integrated Focus Mode (RF01 / Feature 11):**
   - Daily routine schedules with real-time countdowns and active time block tracking.
   - Fullscreen Pomodoro Focus Mode with circular countdown animations, cycle tracking, keyboard shortcuts, and native Web Audio harmonic chime synthesis.

4. **Habits, Streaks & OKR Alignment (RF02 & RF03):**
   - Binary, Numeric, and Timer habit trackers with O(1) streaks and 52-week contribution heatmap.
   - Linked Key Results with automated progress roll-up on habit check-ins.

5. **Real-Time Push Notifications (Feature 14):**
   - Server-Sent Events (SSE) stream listener with Web Notifications API alerts and sound chimes.
   - Interactive in-app toast feed and activity drawer.

---

## 🛠️ Development & Tooling

```bash
# Install dependencies
npm install

# Start development server with proxy to backend (http://127.0.0.1:8080)
npm run dev

# Run Vitest test suites (80 unit and integration tests)
npm test

# Type-check TypeScript code and build production PWA bundle
npm run build

# Preview production build locally
npm run preview
```

---

## 📁 Architecture & Directory Structure

```text
frontend/src/
├── assets/          # Global styles, Tailwind base & glassmorphism utilities
├── components/      # UI components
│   ├── common/      # Network status, modal dialogs, buttons, banners
│   ├── dashboard/   # Routine timeline, time blocks, and Focus Mode Pomodoro
│   ├── habits/      # Habit cards, check-in dialogs, and 52-week heatmap
│   ├── journal/     # Zero-knowledge vault unlock, editor & timeline
│   ├── notifications/# Toast manager and activity feed
│   └── okrs/        # Objectives list, Key Results & progress bars
├── composables/     # Vue composables (usePwa, useModal, useNotification)
├── services/        # WebCrypto (AES-256-GCM/Argon2id), Axios API, Offline Queue (IndexedDB)
├── stores/          # Pinia stores (auth, routine, habit, okr, journal, focus, notification)
├── router/          # Vue Router navigation & auth guards
└── views/           # Top-level route views (Dashboard, Habits, OKRs, Journal, Login)
```
