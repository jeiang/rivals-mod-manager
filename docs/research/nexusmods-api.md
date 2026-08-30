# NexusMods API survey: surface and third-party policy

Research for issue #4. Establishes what the Nexus Mods API lets a third-party mod
manager do, from official sources. Surveyed 2026-08-30.

Marvel Rivals on Nexus Mods: game id `7106`, domain `marvelrivals` (verified with a
live unauthenticated GraphQL query; ~8,750 mods and 96 collections at survey time).

## API surfaces

Nexus Mods currently runs three API generations, all served from `api.nexusmods.com`:

| Surface | Base | Status | Relevance to a mod manager |
| --- | --- | --- | --- |
| REST v1 | `https://api.nexusmods.com/v1/` | "Stable", explicitly supported "for the foreseeable future" | Downloads, md5 lookup, update polling, endorsements, tracking |
| GraphQL v2 | `https://api.nexusmods.com/v2/graphql` | Public but self-described work-in-progress | Search, collections, bulk md5, richer mod metadata/thumbnails |
| REST v3 | `https://api.nexusmods.com/v3/` | New (OpenAPI 3, stability badges per endpoint) | Mostly author/upload workflows (upload sessions, file versions, dependency management); not needed for a consumer manager today |

- The v2 GraphQL docs say: "The current V1 REST API remains available and will
  continue to be supported for the foreseeable future", and warn that v2 is still
  being tweaked, so user-facing apps need error handling for schema drift.
  Source: <https://graphql.nexusmods.com/> (Welcome section).
- The v3 OpenAPI spec (`https://api.nexusmods.com/openapi.yaml`, rendered at
  <https://api-docs.nexusmods.com/>) calls v1 a "Legacy API" for existing
  integrations, and documents a 90-day deprecation policy for stable endpoints.
- v1 reference: <https://app.swaggerhub.com/apis-docs/NexusMods/nexus-mods_public_api_params_in_form_data/1.0>.
- The pre-2019 undocumented "legacy modding API" (what old NMM/MO used) was shut
  down in 2019; everything now goes through the keyed/tokened API.
  Source: <https://www.nexusmods.com/news/14028>.

**Takeaway:** build on REST v1 for downloads/md5/updates and GraphQL v2 for search,
collections, and richer metadata. Ignore v3 unless we grow upload features.

## Auth model

Three mechanisms, in order of modernity:

1. **OAuth 2.0 + PKCE (current, recommended).** Nexus's own guide: "All
   applications will require OAuth 2.0 to interact with the Nexus Mods API on
   behalf of a user." Public apps (mod managers) use PKCE; private/server apps get
   a client secret. OIDC discovery: `https://users.nexusmods.com/.well-known/openid-configuration`
   (authorize `https://users.nexusmods.com/oauth/authorize`, token
   `https://users.nexusmods.com/oauth/token`, S256 supported, `refresh_token`
   grant supported). The access token is a JWT whose claims include
   `membership_roles` (e.g. `premium`) and `premium_expiry`, so the app can tell
   premium status directly from login. Both v1 and v2 accept
   `Authorization: Bearer <access_token>`. Client registration has no self-serve
   UI yet: email support@nexusmods.com with app name, description, logo,
   source link (closed-source apps "may be declined"), and callback URI. The
   callback may be a custom protocol; Vortex uses `nxm://oauth/callback`.
   Sources: <https://modding.wiki/en/api/oauth2-guide> (official Nexus Mods wiki),
   openid-configuration above, Vortex `NXMUrl.ts`.
2. **Personal API key.** `apikey: <key>` header, key from
   `https://www.nexusmods.com/settings/api-keys`. The Acceptable Use Policy
   tolerates personal keys only for testing/personal-use builds; a public-facing
   app must be registered. Source: <https://help.nexusmods.com/article/114-api-acceptable-use-policy>.
3. **SSO websocket (legacy key handout).** `wss://sso.nexusmods.com`: app opens a
   socket with a UUID, sends the user to
   `https://www.nexusmods.com/sso?id=<uuid>&application=<slug>`, receives the
   user's API key over the socket. Requires a staff-issued application slug.
   Still works, but it hands out API keys, not OAuth tokens.
   Source: <https://github.com/Nexus-Mods/sso-integration-demo>.

GraphQL v2 note: "Most of GraphQL V2 is accessible without authentication, but
some endpoints do require an OAuth token" (confirmed live: `game` query works with
no credentials). Self-serve OAuth client registration is promised "once the API is
complete". Source: <https://graphql.nexusmods.com/>.

Registered apps must send `Application-Name` and `Application-Version` headers on
every request; blank or impersonated metadata is prohibited. Source: AUP (above).

**Takeaway:** register an OAuth client (public/PKCE) with Nexus support early —
it is an email process with human review. Fall back to a user-supplied personal
API key during development.

## `nxm://` links

Anatomy (from Vortex's parser, `src/renderer/src/extensions/nexus_integration/NXMUrl.ts`
in <https://github.com/Nexus-Mods/Vortex>):

```
nxm://<game_domain>/mods/<mod_id>/files/<file_id>?key=<key>&expires=<unix_ts>&user_id=<id>
nxm://<game_domain>/collections/<slug>/revisions/<number|latest>
nxm://oauth/callback?code=...&state=...          (OAuth redirect)
nxm://premium                                     (premium upsell callback)
```

- Host part is the game domain (`marvelrivals`); path selects mod-file vs
  collection-revision; `key`/`expires`/`user_id` query params are present when the
  link came from a website "Mod Manager Download" click by a non-premium user.
- Registration is plain OS URI-scheme handling — there is no Nexus-side handshake.
  Windows: `HKCU\Software\Classes\nxm` shell/open/command (Electron apps use
  `app.setAsDefaultProtocolClient`). Linux: `.desktop` entry with
  `MimeType=x-scheme-handler/nxm` + `xdg-mime default` (see Vortex
  `src/renderer/src/util/protocolRegistration/linux/nxm.ts` and its flatpak
  manifest). macOS: `CFBundleURLTypes`/`CFBundleURLSchemes` in Info.plist.
- The website's "Mod Manager Download" button emits an `nxm://` URL and relies on
  whatever app the OS has registered; any third-party manager can claim it (e.g.
  Reloaded-II does: <https://github.com/Reloaded-Project/Reloaded-II/issues/727>).

## Key endpoints for a mod manager (REST v1)

All from the official v1 OpenAPI spec
(<https://app.swaggerhub.com/apis-docs/NexusMods/nexus-mods_public_api_params_in_form_data/1.0>),
field names cross-checked against the official client
<https://github.com/Nexus-Mods/node-nexus-api> (`src/types.ts`):

- **md5 lookup:** `GET /v1/games/{game_domain}/mods/md5_search/{md5}.json` — looks
  up a local archive's MD5, returns mod + file info (`IMD5Result`). GraphQL v2
  adds bulk lookup: `fileHash(md5:)` and `fileHashes(md5s: [...])` returning
  `fileName`, `fileSize`, `gameId`, `modFile { ... }`.
- **Mod metadata:** `GET /v1/games/{game_domain}/mods/{id}.json` — includes
  `name`, `summary`, `version`, `author`, `picture_url` (header image),
  endorsement info. GraphQL `mod(modId:, gameId:)` additionally exposes
  `pictureUrl`, `thumbnailUrl`, `thumbnailLargeUrl` and blurred variants — better
  suited for grid UIs.
- **Files:** `GET .../mods/{mod_id}/files.json` (filter by category:
  main/update/optional/old_version/miscellaneous). Response carries
  `file_updates` (`IFileUpdate[]`: old_file_id → new_file_id chains), which is the
  per-mod update mechanism.
- **Update polling:** `GET /v1/games/{game_domain}/mods/updated.json?period=1d|1w|1m`
  — mods updated in the window, cached server-side for 5 minutes. Pattern: poll
  `updated.json`, intersect with installed mods, then refetch `files.json` and
  walk `file_updates`. GraphQL `mod` also exposes `viewerUpdateAvailable`.
- **Download links:** `GET .../files/{file_id}/download_link.json` — see next
  section.
- **Misc:** `validate.json` (check key / get user), endorse/abstain, tracked
  mods CRUD, `games.json`.

## Downloads: free vs premium

The v1 `download_link.json` doc is explicit:

- Premium users: returns an array of CDN download links (preferred location
  first), with no preconditions — full "one-click"/automated downloads.
- Non-premium: "Non-premium members must provide the key and expiry from the .nxm
  link provided by the website... This ensures that all non-premium members must
  access the website to download through the API."

So **yes**: the app can download from a free user's "Mod Manager Download" click —
that click yields `nxm://...?key=...&expires=...`, and passing `key` + `expires`
to `download_link.json` returns a valid link. What the app cannot do for free
users is *initiate* downloads itself (no key = HTTP 403), so batch operations
(update-all, collection install) degrade to one website click per file. Free
downloads are also speed-capped (1-3 MB/s tier depending on membership/adblock;
premium is uncapped). Sources: v1 spec (download_link description);
<https://help.nexusmods.com/article/96-download-speed-caps-adblockers-and-different-types-of-membership>;
<https://www.nexusmods.com/premium>.

Generated links are time-limited and user-bound; the ToS prohibits rehosting,
bulk/automated grabbing beyond "normal and expected usage", and any scraping.
Sources: <https://help.nexusmods.com/article/18-terms-of-service>, AUP.

## Rate limits

- 20,000 requests per 24h; once exhausted, 500 requests/hour. Daily quota resets
  00:00 GMT, hourly on the hour. Source:
  <https://help.nexusmods.com/article/105-i-have-reached-a-daily-or-hourly-limit-api-requests-have-been-consumed-rate-limit-exceeded-what-does-this-mean>.
- Remaining quota is sent on every response; headers are `x-rl-daily-remaining` /
  `x-rl-hourly-remaining` (as consumed by the official node client, `Nexus.ts`).
  HTTP 429 signals limit exceeded or load shedding.
- Practical: 20k/day is roomy for one user's manager (metadata + downloads), but
  argues for caching metadata and batching md5 lookups via GraphQL `fileHashes`.

## Collections

- **Policy: third-party install is permitted.** Nexus's collections announcement:
  collections launch with Vortex, but "we will be providing the source code for
  Vortex and an open GraphQL API which can be used by third-party tools."
  Source: <https://www.nexusmods.com/news/14568>. Community precedent exists
  (Collections Manager plugin for MO2). Nothing in the ToS/AUP forbids
  third-party collection installs; the AUP constraints (registered app, no
  scraping/rehosting) apply as usual.
- **API surface is GraphQL v2 only:** `collection(slug:)`,
  `collectionRevision(slug:, revision:, domainName:)` — returns `downloadLink`
  (the collection archive containing the manifest), `modFiles`
  (`CollectionRevisionMod` entries), `collectionChangelog`, sizes, ratings —
  plus `collectionsV2` search with facets/filters. `nxm://` collection links and
  the site's "Add to Vortex" style buttons resolve to slug + revision, which maps
  1:1 onto `collectionRevision`. Source: <https://graphql.nexusmods.com/>.
- **Practicality gates:**
  - Per-file downloads inside a collection still go through v1
    `download_link.json`, so automated install of an N-mod collection is
    effectively premium-only; free users need a website click per mod (this is
    exactly how Vortex behaves, with its `nxm://premium` upsell path).
  - The v2 schema is declared unstable; collection manifest format
    (`collection.json` inside the archive) is Vortex-defined.
  - Marvel Rivals has ~96 collections today.
- **Recommendation:** collections *can* stay in scope ToS-wise, but as a later
  phase: premium-only automation, unstable API, and small catalog for this game
  make them poor v1 scope. Single-mod nxm handling covers the dominant flow.

## Constraints checklist for the app

1. Register an OAuth public client via support@nexusmods.com before public
   release (name, description, logo, source link, `nxm://oauth/callback` or
   loopback redirect). Personal API keys only for dev builds.
2. Send `Application-Name` / `Application-Version` on every request.
3. Never store users' keys/tokens server-side, never scrape or rehost mod data;
   filter returned content ourselves (ToS makes third parties responsible).
4. Track `x-rl-*` headers; back off on 429.
5. Gate "download all"/collection automation on `membership_roles` containing
   `premium`; free flow = register `nxm://` handler + accept key/expires links.
