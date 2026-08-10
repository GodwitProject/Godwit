# Godwit UI Revamp — Design Specification

**Date:** 2026-08-10  
**Status:** Approved  
**Scope:** Remplacer `apps/admin/` et le contenu de `apps/ui/` par une unique SPA Vite + React, avec deux espaces : Admin (`/admin/*`) et Console (`/console/*`).

---

## 1. Overview

### 1.1 Contexte

Le dépôt contient actuellement deux applications frontend incomplètes et redondantes :
- `apps/admin/` : dashboard admin Next.js 14 avec shadcn/ui, focalisé sur orgs/teams/users.
- `apps/ui/` : scaffold Next.js 14 pour un dashboard opérationnel (modèles, providers, clés, logs).

Ces deux apps se marchent dessus et ne reflètent pas le périmètre métier réel de Godwit : un proxy LLM multi-providers avec gestion des modèles, des clés API et de la consommation.

### 1.2 Objectifs

- Une **seule application** dans `apps/ui/`.
- Deux espaces clairement séparés :
  - **Admin** (`/admin/*`) : configuration instance-wide, accessible uniquement au `super_admin`.
  - **Console** (`/console/*`) : usage quotidien, accessible aux rôles `org_admin`, `team_admin` et `user`.
- Un `super_admin` peut naviguer dans les deux espaces.
- La console ne duplique pas bêtement l'API : elle expose les vues métier (modèles disponibles, mes clés, ma conso).
- Stack simple, testable, sans complexité Server Components inutile pour un dashboard.

### 1.3 Non-Goals

- Pas de playground/chat dans ce design.
- Pas de mobile app native (responsive web uniquement).
- Pas de white-label / multi-branding.
- Pas de gestion SSO/SAML côté frontend (le backend gère déjà ; l'UI affiche juste le login mot de passe).

---

## 2. Architecture

### 2.1 Stack technique

| Couche | Choix | Raison |
|--------|-------|--------|
| Build | Vite 5 + React 18 | Dev server rapide, config simple, pas de SSR nécessaire pour un dashboard. |
| Routing | React Router 6 | Guards programmatiques simples, layouts imbriqués. |
| Requêtes | TanStack Query (React Query) | Cache, retry, invalidation, polling. |
| État client | Zustand | Uniquement pour l'authentification ; le reste est dans React Query. |
| Styling | Tailwind CSS 3.4 | Design tokens Godwit, pas de lib UI lourde. |
| Formulaires | React Hook Form + Zod | Validation type-safe. |
| Icons | Lucide React | Léger, cohérent. |
| Tests | Vitest + React Testing Library + MSW | Tests unitaires + mocks API. |

### 2.2 Backend déjà existant

L'API Rust expose déjà :
- Auth : `POST /api/v1/auth/login`, `POST /api/v1/auth/logout`, `POST /api/v1/auth/refresh`, `GET /api/v1/auth/me`.
- Cookies httpOnly : `godwit_access` (Path=/), `godwit_refresh` (Path=/api/v1/auth).
- `jwt_auth` accepte soit `Authorization: Bearer`, soit le cookie `godwit_access`.
- RBAC backend : `super_admin`, `org_admin`, `team_admin`, `user`.

L'UI doit exploiter le cookie httpOnly via `credentials: 'include'` sur toutes les requêtes.

### 2.3 Structure du projet

```
apps/ui/
├── public/
├── src/
│   ├── main.tsx                      # point d'entrée Vite
│   ├── App.tsx                       # router + providers
│   ├── routes/
│   │   ├── login.tsx                 # /login
│   │   ├── admin/
│   │   │   ├── index.tsx             # /admin
│   │   │   ├── models.tsx            # /admin/models
│   │   │   ├── provider-profiles.tsx # /admin/provider-profiles
│   │   │   ├── users.tsx             # /admin/users
│   │   │   ├── keys.tsx              # /admin/keys
│   │   │   ├── usage.tsx             # /admin/usage
│   │   │   └── settings.tsx          # /admin/settings
│   │   └── console/
│   │       ├── index.tsx             # /console
│   │       ├── models.tsx            # /console/models
│   │       ├── keys.tsx              # /console/keys
│   │       ├── usage.tsx             # /console/usage
│   │       ├── organization.tsx      # /console/organization
│   │       └── team.tsx              # /console/team
│   ├── layouts/
│   │   ├── AdminLayout.tsx           # sidebar + header admin
│   │   ├── ConsoleLayout.tsx         # sidebar + header console
│   │   └── AuthLayout.tsx            # centré, sans sidebar
│   ├── components/
│   │   ├── ui/                       # primitives (Button, Card, Input, ...)
│   │   ├── auth/                     # LoginForm, RequireRole, RoleGuard
│   │   └── common/                   # PageHeader, EmptyState, ErrorBoundary
│   ├── lib/
│   │   ├── api.ts                    # définitions endpoints + helpers types
│   │   ├── http.ts                   # apiFetch, UnauthorizedError
│   │   ├── auth.ts                   # login/logout/fetchMe
│   │   └── utils.ts                  # formatDate, formatCurrency, ...
│   ├── store/
│   │   └── auth.ts                   # Zustand auth store
│   ├── hooks/
│   │   ├── useAuth.ts                # init auth + redirect
│   │   ├── useModels.ts              # queries modèles
│   │   ├── useProviderProfiles.ts    # queries providers
│   │   ├── useApiKeys.ts             # queries clés
│   │   └── useUsage.ts               # queries conso
│   ├── types/
│   │   └── index.ts                  # User, Model, ProviderProfile, ApiKey, ...
│   └── styles/
│       └── index.css                 # Tailwind + tokens Godwit
├── index.html
├── package.json
├── vite.config.ts
├── tailwind.config.ts
├── tsconfig.json
└── vitest.config.ts
```

---

## 3. Auth flow

### 3.1 Cookie-based auth

Le backend pose les cookies httpOnly lors du login. Le frontend n'a **jamais** accès au JWT : il appelle juste `fetchMe()` via le cookie.

```ts
// src/lib/auth.ts
export interface AuthUser {
  id: string;
  email: string;
  role: 'super_admin' | 'org_admin' | 'team_admin' | 'user';
  organization_id: string | null;
}

export async function login(email: string, password: string): Promise<AuthUser> {
  const res = await fetch('/api/v1/auth/login', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'include',
    body: JSON.stringify({ email, password }),
  });
  if (!res.ok) throw new Error(res.status === 401 ? 'Invalid credentials' : 'Login failed');
  await res.json();
  return fetchMe();
}

export async function logout(): Promise<void> {
  await fetch('/api/v1/auth/logout', { method: 'POST', credentials: 'include' });
}

export async function fetchMe(): Promise<AuthUser> {
  const res = await fetch('/api/v1/auth/me', { credentials: 'include' });
  if (!res.ok) throw new Error('Not authenticated');
  const data = await res.json();
  return data.user as AuthUser;
}
```

### 3.2 Fetch wrapper avec auto-refresh

```ts
// src/lib/http.ts
export class UnauthorizedError extends Error {
  constructor() {
    super('Unauthorized');
    this.name = 'UnauthorizedError';
  }
}

let refreshPromise: Promise<boolean> | null = null;

async function doRefresh(): Promise<boolean> {
  try {
    const r = await fetch('/api/v1/auth/refresh', {
      method: 'POST',
      credentials: 'include',
    });
    return r.ok;
  } catch {
    return false;
  }
}

export async function apiFetch(path: string, init: RequestInit = {}): Promise<Response> {
  const merged: RequestInit = { ...init, credentials: 'include' };
  const res = await fetch(path, merged);
  if (res.status !== 401) return res;

  if (!refreshPromise) {
    refreshPromise = doRefresh().finally(() => {
      refreshPromise = null;
    });
  }
  const ok = await refreshPromise;
  if (!ok) throw new UnauthorizedError();

  return fetch(path, merged);
}
```

### 3.3 Auth store

```ts
// src/store/auth.ts
import { create } from 'zustand';
import type { AuthUser } from '@/lib/auth';

export type AuthStatus = 'unknown' | 'authenticated' | 'unauthenticated';

interface AuthStore {
  user: AuthUser | null;
  status: AuthStatus;
  setUser: (user: AuthUser | null) => void;
}

export const useAuthStore = create<AuthStore>((set) => ({
  user: null,
  status: 'unknown',
  setUser: (user) => set({ user, status: user ? 'authenticated' : 'unauthenticated' }),
}));
```

### 3.4 Guards

```ts
// src/components/auth/RequireRole.tsx
type Role = 'super_admin' | 'org_admin' | 'team_admin' | 'user';

interface RequireRoleProps {
  allowed: Role[];
  fallback?: React.ReactNode;
  children: React.ReactNode;
}
```

Utilisation dans le router :

```tsx
<Route element={<RequireRole allowed={['super_admin']} fallback={<Navigate to="/console" />} />}>
  <Route element={<AdminLayout />}>
    <Route path="/admin" element={<AdminDashboard />} />
    <Route path="/admin/models" element={<ModelsPage />} />
    {/* ... */}
  </Route>
</Route>

<Route element={<RequireRole allowed={['super_admin', 'org_admin', 'team_admin', 'user']} />}>
  <Route element={<ConsoleLayout />}>
    <Route path="/console" element={<ConsoleDashboard />} />
    <Route path="/console/models" element={<ConsoleModelsPage />} />
    {/* ... */}
  </Route>
</Route>
```

### 3.5 Initialisation

`App.tsx` déclenche `fetchMe()` au mount via `useAuthInit` :

```ts
// src/hooks/useAuth.ts
export function useAuthInit() {
  const setUser = useAuthStore((s) => s.setUser);

  useEffect(() => {
    fetchMe()
      .then(setUser)
      .catch(() => setUser(null));
  }, [setUser]);
}
```

---

## 4. Routes & layouts

### 4.1 Auth

| Route | Rôle | Description |
|-------|------|-------------|
| `/login` | public | Formulaire email/password. Redirige vers `/admin` ou `/console` selon le rôle. |

### 4.2 Admin (`super_admin` uniquement)

| Route | Description |
|-------|-------------|
| `/admin` | Dashboard instance-wide (stats globales, conso, alertes). |
| `/admin/models` | Catalogue des modèles + création/édition. |
| `/admin/provider-profiles` | Configuration des providers : OpenAI wildcard, base URL, clé API, sglang/vllm/ollama. |
| `/admin/users` | Gestion des utilisateurs. |
| `/admin/keys` | Vue globale des clés API. |
| `/admin/usage` | Conso globale par org/team/user. |
| `/admin/settings` | Paramètres instance. |

### 4.3 Console (`org_admin`, `team_admin`, `user`)

| Route | Description | Rôle minimal |
|-------|-------------|------------|
| `/console` | Accueil conso perso + raccourcis. | `user` |
| `/console/models` | Liste des modèles disponibles (read-only). | `user` |
| `/console/keys` | Mes clés API (créer, révoquer, régénérer). | `user` |
| `/console/usage` | Ma consommation (requests, tokens, coût). | `user` |
| `/console/organization` | Clés et conso de toute l'organisation. | `org_admin` |
| `/console/team` | Clés et conso de l'équipe. | `team_admin` |

### 4.4 Navigation conditionnelle dans la console

La sidebar console affiche :
- Accueil, Modèles, Mes clés, Ma conso pour tout le monde.
- Onglet **Organisation** si `role === 'org_admin'`.
- Onglet **Équipe** si `role === 'team_admin'`.

---

## 5. Composants UI

### 5.1 Primitives (Tier 1)

Dans `src/components/ui/` :

- `Button` : variants `primary`, `secondary`, `ghost`, `danger` ; sizes `sm`, `md`, `lg`.
- `Card` : avec header/body/footer optionnels.
- `Input` : label, error, helper text.
- `Select` : single/multi.
- `Badge` : statuses neutre/success/warning/error/info.
- `Table` : head/body/row/cell avec styles Godwit.
- `Modal` : Dialog accessible.
- `Tabs` : pour la navigation interne.
- `Skeleton` : états de chargement.
- `EmptyState` : illustration + message + CTA.
- `PageHeader` : titre + description + action principale.
- `ErrorBoundary` : capture erreurs React.

### 5.2 Design tokens Tailwind

```ts
// tailwind.config.ts (extrait)
colors: {
  surface: '#f8f9fb',
  'surface-dim': '#d9dadc',
  'surface-container-lowest': '#ffffff',
  'surface-container-low': '#f3f4f6',
  'surface-container': '#edeef0',
  'surface-container-high': '#e7e8ea',
  'on-surface': '#191c1e',
  'on-surface-variant': '#434655',
  primary: '#004ac6',
  'on-primary': '#ffffff',
  'primary-container': '#2563eb',
  secondary: '#515f74',
  error: '#ba1a1a',
  success: '#10b981',
  warning: '#f59e0b',
  info: '#3b82f6',
  outline: '#737686',
  'outline-variant': '#c3c6d7',
},
fontFamily: {
  sans: ['Inter', 'system-ui', 'sans-serif'],
  mono: ['JetBrains Mono', 'monospace'],
},
```

### 5.3 Composants métier (Tier 2)

Dans `src/components/` :

- `auth/LoginForm`
- `auth/RequireRole`
- `common/PageHeader`
- `common/EmptyState`
- `common/ErrorBoundary`
- Plus tard : `models/ModelCard`, `keys/KeyList`, `usage/SpendChart`, etc.

---

## 6. Data fetching

### 6.1 TanStack Query

Configuration globale dans `App.tsx` :

```tsx
<QueryClientProvider client={queryClient}>
  <RouterProvider router={router} />
</QueryClientProvider>
```

Exemple de hook :

```ts
// src/hooks/useModels.ts
export function useModels() {
  return useQuery({
    queryKey: ['models'],
    queryFn: async () => {
      const res = await apiFetch('/api/v1/models');
      if (!res.ok) throw new Error('Failed to fetch models');
      const data = await res.json();
      return data.data as Model[];
    },
  });
}
```

### 6.2 Mutations

```ts
export function useCreateModel() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (payload: CreateModelPayload) => {
      const res = await apiFetch('/api/v1/models', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload),
      });
      if (!res.ok) throw new Error('Failed to create model');
      return res.json();
    },
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['models'] }),
  });
}
```

---

## 7. RBAC

### 7.1 Rôles

Rôles reconnus par le backend :
- `super_admin` : accès total.
- `org_admin` : accès console + onglet Organisation.
- `team_admin` : accès console + onglet Équipe.
- `user` : accès console basique.

### 7.2 Application côté frontend

- **Router** : `RequireRole` redirige si le rôle n'est pas autorisé.
- **Sidebar** : masque les liens non autorisés.
- **Boutons** : masque les actions non autorisées (ex: un `user` ne voit pas le bouton "Créer une clé org").

Le backend reste l'autorité définitive ; le frontend n'applique le RBAC que pour l'UX.

---

## 8. Tests

### 8.1 Tests unitaires

- Composants UI : render + interactions (Vitest + RTL).
- Hooks : mocks de `apiFetch` via MSW.
- Auth store : transitions de statut.
- Guards : redirections selon le rôle.

### 8.2 Tests d'intégration

- Login → cookie → fetchMe → navigation.
- `apiFetch` : 401 → refresh → retry.

### 8.3 Contract tests

Conserver un test qui valide que les routes définies dans `contract/routes.json` sont bien présentes côté backend (déjà existant dans l'ancien `apps/ui/tests/route-contract.test.ts` ; à adapter si le format change).

---

## 9. Milestone 0 : Auth + Shell

Objectif : livrer la fondation avant toute page métier.

### 9.1 Nettoyage

- Supprimer entièrement `apps/admin/`.
- Vider entièrement `apps/ui/` (pas de fichier conservé : on recrée package.json, Dockerfile, tests, etc. from scratch).

### 9.2 Scaffold

- `package.json` avec Vite, React, React Router, TanStack Query, Zustand, Tailwind, RHF, Zod, Lucide, Vitest, MSW.
- `vite.config.ts`, `tsconfig.json`, `tailwind.config.ts`, `vitest.config.ts`, `index.html`.
- Design tokens Godwit dans Tailwind.

### 9.3 Auth complet

- `lib/auth.ts`, `lib/http.ts`, `store/auth.ts`.
- `hooks/useAuth.ts` (init).
- `components/auth/LoginForm.tsx`.
- `components/auth/RequireRole.tsx`.

### 9.4 Shells

- `layouts/AuthLayout.tsx`.
- `layouts/AdminLayout.tsx` avec sidebar admin.
- `layouts/ConsoleLayout.tsx` avec sidebar console + navigation conditionnelle.

### 9.5 Composants UI de base

- `Button`, `Card`, `Input`, `Badge`, `Table`, `PageHeader`, `EmptyState`.

### 9.6 Router

- Toutes les routes définies en section 4.
- Pages placeholder vides pour chaque route.

### 9.7 Tests

- Tests sur `apiFetch`, auth store, `RequireRole`, `LoginForm`, composants UI.

### 9.8 Acceptance criteria

- `npm install` fonctionne.
- `npm run dev` démarre.
- `npm run test` passe.
- `npm run build` réussit.
- Login OK → redirection `/admin` pour `super_admin`, `/console` pour les autres.
- Accès `/admin/*` refusé aux non-`super_admin`.
- Navigation console conditionnelle selon le rôle.

---

## 10. Milestones suivants

### M1 : Gestion des modèles/provider profiles (admin)

- CRUD provider profiles (`/admin/provider-profiles`).
- CRUD modèles (`/admin/models`).
- Formulaire : provider, base URL, clé API, allow wildcard, mapping modèle public → provider_model_id, pricing, capabilities.

### M2 : Catalogue + clés API (console)

- `/console/models` : liste read-only des modèles disponibles.
- `/console/keys` : CRUD de ses clés API, copie one-shot, révocation.

### M3 : Usage

- `/console/usage` : conso perso.
- `/console/organization` : conso org + clés org.
- `/console/team` : conso équipe + clés équipe.

### M4 : Dashboard admin

- `/admin` : stats globales, graphiques conso, alertes.

---

## 11. Open Questions / Risks

1. **OIDC/SAML** : le login UI ne gère que le password dans M0. OIDC reste backend-only jusqu'à nouvel ordre.
2. **Backend endpoints** : plusieurs endpoints listés dans l'ancien design spec (`/metrics/*`, `/usage/*`) n'existent pas encore ; ils devront être créés au fil des milestones.
3. **Real-time** : WebSocket metrics pas prioritaire en M0 ; on utilisera du polling TanStack Query.
4. **Docker** : l'ancien `apps/ui/Dockerfile` était pour Next.js. Il faudra le réécrire pour Vite (nginx static).

---

## 12. Appendix : Backend endpoints déjà utilisables

| Endpoint | Usage |
|----------|-------|
| `POST /api/v1/auth/login` | Login |
| `POST /api/v1/auth/logout` | Logout |
| `POST /api/v1/auth/refresh` | Refresh token |
| `GET /api/v1/auth/me` | User courant |
| `GET /api/v1/models` | Liste modèles (admin) |
| `POST /api/v1/models` | Créer modèle |
| `PATCH /api/v1/models/:id` | Modifier modèle |
| `DELETE /api/v1/models/:id` | Supprimer modèle |
| `GET /api/v1/provider-profiles` | Liste providers (admin) |
| `POST /api/v1/provider-profiles` | Créer provider |
| `PATCH /api/v1/provider-profiles/:id` | Modifier provider |
| `GET /api/v1/api-keys` | Liste clés (scope selon rôle) |
| `POST /api/v1/api-keys` | Créer clé |
| `POST /api/v1/api-keys/:id/block` | Bloquer clé |
| `POST /api/v1/api-keys/:id/unblock` | Débloquer clé |
| `POST /api/v1/api-keys/:id/regenerate` | Régénérer clé |
| `POST /api/v1/api-keys/:id/reset_spend` | Reset conso clé |
| `GET /api/v1/spend?days=30` | Conso quotidienne |
| `GET /api/v1/admin/stats` | Stats dashboard admin |
| `GET /api/v1/admin/recent-activity` | Activité récente |

---

**END OF DESIGN SPEC**
