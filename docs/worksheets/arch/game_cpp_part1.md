# game.cpp Part 1: Analysis (Lines 1-4000)

Complete function analysis with 8-field schema per function.

---

## Functions Analyzed (Lines 1-4000)

### `uint8_t ConvertToCP437(uint32_t uc)` (game.cpp:180-326)

**Signature:** `uint8_t ConvertToCP437(uint32_t uc)`

**Purpose:** Converts a Unicode code point to its CP437 (Code Page 437) equivalent using static lookup tables.

**Called by:** game.cpp (line 402 in char* overload), other sources in game.

**Calls:** No significant functions; uses only lookup tables (tab00A1, tab0192, tab0393, tab2022, tab2190, tab2219, tab2302, tab2500, tab263A).

**Globals read:** None; all data is local/static.

**Globals mutated:** None.

**Side effects:** Pure lookup function; no I/O, no allocations, no state changes. Returns CP437 glyph value (0-255).

**Notes:** Implements Unicode-to-CP437 mapping via hardcoded lookup tables for ranges 0x00A1-0x00A1+94, 0x0192, 0x0393-0x0393+51, 0x2022-0x2022+133, 0x2190-0x2190+24, 0x2219-0x2219+76, 0x2302-0x2302+31, 0x2500-0x2500+217, 0x263A-0x263A+49. Unmapped codes default to space (0x20). Used in game name entry and text rendering for UTF-8 → CP437 encoding pipeline.

---

### `void ConvertToCP437(char* cp437, const char* _utf8, int maxlen)` (game.cpp:328-405)

**Signature:** `void ConvertToCP437(char* cp437, const char* _utf8, int maxlen)`

**Purpose:** Batch converts UTF-8 string to CP437 by processing up to maxlen characters; validates UTF-8 byte sequences and calls the single-character overload.

**Called by:** game_api.cpp (line 551), game.cpp (line 1852 in Server::Proc).

**Calls:** ConvertToCP437(uint32_t) — single-char overload at line 402.

**Globals read:** None.

**Globals mutated:** None.

**Side effects:** Writes to output buffer cp437; terminates string at null or maxlen boundary. No file I/O, no allocations.

**Notes:** Parses UTF-8 byte sequences (1-4 bytes per codepoint) with error recovery—skips malformed bytes. Used in networking to encode player_name_cp437 from player_name (UTF-8). Stops at null terminator or maxlen limit. Critical for cross-platform player name display.

---

### `void ChatLog(const char* fmt, ...)` (game.cpp:463-471)

**Signature:** `void ChatLog(const char* fmt, ...)`

**Purpose:** Variadic printf-style logging for chat messages and game events; broadcasts to stdout (and optionally to network/browser).

**Called by:** Server::Proc (line 1967 for incoming chat), Game::OnRender (multiple locations: player talk, enemy talk, debug output).

**Calls:** vprintf(fmt, args) via va_list; no other significant calls.

**Globals read:** None directly; vprintf writes to stdout.

**Globals mutated:** None; side effect is stdout write.

**Side effects:** Writes formatted string to stdout. In web builds, this may route to browser console or server log.

**Notes:** Comment at line 465 warns: "move it to game_app/web/srv and asciid — we don't want to printf in -term mode!" Intended as unified logging point for chat, debug output, and event notifications. No filtering or rate-limiting.

---

### `static void WriteJsonString(FILE* f, const char* str)` (game.cpp:473-483)

**Signature:** `static void WriteJsonString(FILE* f, const char* str)`

**Purpose:** Escapes and writes a string to FILE stream in JSON-safe format (quotes, backslash escaping for inner quotes/backslashes).

**Called by:** WriteShotJson (line 511 for g_loaded_a3d_path).

**Calls:** fputc (C stdlib) for character-by-character output.

**Globals read:** None.

**Globals mutated:** FILE* f state (buffered writes).

**Side effects:** File I/O; writes to open file pointer. No allocations.

**Notes:** Escapes `\` and `"` chars by prepending backslash. Used only for JSON metadata output in screenshot dumps. No error handling if file is closed.

---

### `static void WriteShotJson(const char* path, uint64_t stamp, const PhysicsIO* io, const Game* g, int width, int height)` (game.cpp:485-535)

**Signature:** `static void WriteShotJson(const char* path, uint64_t stamp, const PhysicsIO* io, const Game* g, int width, int height)`

**Purpose:** Dumps game state (camera, player pos, light, water level) to JSON file for screenshot metadata and playback reconstruction.

**Called by:** Game::OnRender (line 4884 when screenshot saved, hardcoded path "./shot.json").

**Calls:** fopen, fprintf, fclose (C stdlib); WriteJsonString for string escaping.

**Globals read:** g_loaded_a3d_path (world filename).

**Globals mutated:** None; only file I/O.

**Side effects:** File I/O; creates/overwrites ./shot.json; normalizes light direction vector if needed. Returns early (no-op) if path/io/g are null.

**Notes:** Safety TODO at line 4884: hardcoded CWD path unsafe for production/distro. Includes version, timestamp, map path, camera (pos, yaw, zoom, perspective), player pos/dir, light dir/ambience, water level. Used for replay/debug analysis.

---

### `bool GetGamePadConfPath(char* path, const char* name, int axes, int buttons)` (game.cpp:540-584)

**Signature:** `bool GetGamePadConfPath(char* path, const char* name, int axes, int buttons)`

**Purpose:** Constructs safe filesystem path for gamepad configuration file by replacing invalid path characters; returns false if path construction fails.

**Called by:** ReadGamePadConf (line 589), WriteGamePadConf (line 606).

**Calls:** GetConfPath() (external; returns config directory), strrchr (C stdlib), sprintf, char validation loop (lines 561-581).

**Globals read:** None (GetConfPath is external).

**Globals mutated:** path buffer (output).

**Side effects:** String manipulation only; no I/O, no allocations.

**Notes:** Sanitizes gamepad name by replacing unsafe chars: control chars (<=32), DEL (>=127), `<>|?*":/` → safe alternates `[].|;;;;`. Format: `asciicker_(NAME)_A{axes}_B{buttons}.cfg`. If name is empty or GetConfPath returns null, returns false. Critical for safe file I/O on Windows with special chars in gamepad names.

---

### `bool ReadGamePadConf(uint8_t map[256], const char* name, int axes, int buttons)` (game.cpp:586-601)

**Signature:** `bool ReadGamePadConf(uint8_t map[256], const char* name, int axes, int buttons)`

**Purpose:** Loads gamepad button/axis remapping from binary config file; returns true on successful read of exactly 2*axes+buttons bytes.

**Called by:** GamePadOnConnect (line 10764, gamepad initialization to load user remapping).

**Calls:** GetGamePadConfPath (line 589), fopen, fread, fclose (C stdlib).

**Globals read:** None.

**Globals mutated:** map array (output buffer, 256 bytes).

**Side effects:** File I/O; reads binary config. Returns false if file missing or size mismatch.

**Notes:** Binary format: 2 bytes per axis + 1 byte per button (raw input indices). Expected size = 2*axes+buttons. If file doesn't exist, returns false silently (no error logged). Used to restore user gamepad customizations at startup.

---

### `bool WriteGamePadConf(const uint8_t* map, const char* name, int axes, int buttons)` (game.cpp:603-620)

**Signature:** `bool WriteGamePadConf(const uint8_t* map, const char* name, int axes, int buttons)`

**Purpose:** Persists gamepad remapping to binary config file after user customization; returns true on successful write.

**Called by:** gamepad_close callback (line 10948, when user finishes remapping and saves).

**Calls:** GetGamePadConfPath (line 606), fopen, fwrite, fclose (C stdlib), SyncConf() (line 617, external—syncs to server).

**Globals read:** None.

**Globals mutated:** Filesystem state via file write and SyncConf side effect.

**Side effects:** File I/O; creates/overwrites config file; calls SyncConf to notify server of config change. Returns false if file write fails.

**Notes:** Binary format matches ReadGamePadConf: 2*axes+buttons bytes. If fopen or fwrite fails, returns false. SyncConf()  notifies networking layer. Used to save user gamepad customizations persistently.

---

### `void ReadConf(Game* g)` (game.cpp:622-643)

**Signature:** `void ReadConf(Game* g)`

**Purpose:** Loads game settings (talk memory, perspective mode, blood, mute) from config file into Game struct at startup.

**Called by:** CreateGame (game initialization).

**Calls:** fopen, fread, fclose (C stdlib), GetConfPath() (external), AudioMute (line 637, external).

**Globals read:** None directly; GetConfPath() is external call.

**Globals mutated:** g->talk_mem (4x TalkMem), g->perspective, g->blood, g->mute.

**Side effects:** File I/O (reads binary); applies mute setting to audio system via AudioMute(). Silent no-op if file missing.

**Notes:** Binary format: 4*sizeof(Game::TalkMem) bytes talk_mem, 1 byte perspective, 1 byte blood, 1 byte mute. Commented-out debug printf at lines 627/641 suggests this was debug/profile point. AudioMute(g->mute) applied immediately (line 637). Critical for preserving player settings across sessions.

---

### `void WriteConf(Game* g)` (game.cpp:645-666)

**Signature:** `void WriteConf(Game* g)`

**Purpose:** Persists game settings (talk memory, perspective, blood, mute) to config file for player preference retention.

**Called by:** Game::OnRender (multiple locations: perspective toggle, blood toggle, mute toggle; implied from comments).

**Calls:** fopen, fwrite, fclose (C stdlib), GetConfPath() (external), SyncConf() (line 665, external).

**Globals read:** None directly.

**Globals mutated:** Filesystem via file write; SyncConf side effect.

**Side effects:** File I/O; creates/overwrites config. Calls SyncConf() afterward. Silent no-op if fopen fails.

**Notes:** Binary format matches ReadConf: 4*sizeof(Game::TalkMem), 1 byte each for perspective/blood/mute. Commented-out debug printf at lines 651/662. SyncConf()  notifies server of config change (networking). Mirrors ReadConf exactly for symmetric save/load.

---

### `bool Server::Proc(const uint8_t* ptr, int size)` (game.cpp:1841-2025)

**Signature:** `bool Server::Proc(const uint8_t* ptr, int size)`

**Purpose:** Processes incoming network messages from server (join, exit, pose, chat, lag response); updates local player list and state based on server broadcasts.

**Called by:** game_api.cpp or network integration layer (TCP/WebSocket receiver calling this with message pointer/size).

**Calls:** memset, strcpy, ConvertToCP437 (line 1852), malloc, CreateInst, UpdateSpriteInst, DeleteInst, ChatLog (line 1967).

**Globals read:** world, server->others[], server->head, server->tail, stamp (server timestamp).

**Globals mutated:** server->others[] (remote player array), server->head/tail (linked list of active players), server->lag_ms, server->lag_wait.

**Side effects:** Dynamic memory allocation (malloc for TalkBox), world instance creation/deletion, chat logging, lag measurement calculation.

**Notes:** Switch on message type (ptr[0]):
- 'j' (BRC_JOIN): Insert new remote player, parse equipment, create world instance.
- 'e' (BRC_EXIT): Remove player from linked list, free talks, delete world instance.
- 'p' (BRC_POSE): Update remote player position/animation/equipment, refresh world instance.
- 't' (BRC_TALK): Add chat bubble, allocate TalkBox, manage 3-message cap with FIFO eviction.
- 'l' (RSP_LAG): Echo timestamp for latency calculation, store in lag_ms.
Returns false for unknown message types; true on success. Critical for multiplayer synchronization.

---

### `void LoadSprites()` (game.cpp:3253-3514)

**Signature:** `void LoadSprites()`

**Purpose:** Initializes all sprite resources at startup: character equipment variants, mount sprites, item sprites, UI elements, and builds static ItemProto catalog.

**Called by:** game_web.cpp (line 659, web startup), game_app.cpp (line 2064, native game init).

**Calls:** LoadFont1(), LoadGamePad(), LoadMainMenuSprites(), LoadSpriteBP() (line 3266+, custom wrapper), GetSprite() (called on loaded sprites for validation in some versions).

**Globals read:** base_path (config), recolor[] palette for enemy team differentiation.

**Globals mutated:** All 5D sprite arrays (player[], player_fall[], player_attack[], wolfie[], wolfie_attack[], wolfie_fall[], bigbee[], bigbee_attack[], bigbee_fall[], wolf[], bee[], fire_sprite[], character_button, inventory_sprite, keyb_sprite[], caps_sprite[], item sprites), item_proto_lib pointer.

**Side effects:** Sprite loading via LoadSpriteBP(); palette recoloring for enemy sprites (lines 3297-3299); ItemProto static array initialization (line 3458-3513).

**Notes:** WIN32 guard at line 3255 sets printf count output (debug). 5D equipment arrays populated via nested loops (lines 3309-3359): for each (armor, helmet, shield, color) load player/fall/wolfie/bigbee variants with corresponding weapon. Mount sprites named "wolfie-AHSW.xp", "bigbee-AHSW.xp". Attack sprites only for weapons 1+ (no attack for WEAPON::NONE). wolfie_fall[], bigbee_fall[] hardcoded to 0 (TODOs at lines 3334, 3339). Loads 43 item prototypes into static item_proto_lib catalog. Critical blocking call—game cannot render until all sprites loaded.

---

### `void FreeSprites()` (game.cpp:3664-3674)

**Signature:** `void FreeSprites()`

**Purpose:** Releases all allocated sprite memory at shutdown; inverse of LoadSprites().

**Called by:** game_app.cpp (lines 2241, 3515, game cleanup/shutdown).

**Calls:** FreeFont1(), FreeGamePad(), FreeMainMenuSprites(), GetFirstSprite() loop + FreeSprite() (lines 3672-3673).

**Globals read:** Sprite registry (internal to sprite system).

**Globals mutated:** All sprite globals cleared implicitly via FreeSprite (pointers become invalid).

**Side effects:** Memory deallocation; invalidates all sprite pointers. Handles double-refs gracefully per comment at line 3671 ("handles double refs but not sprite prefs!").

**Notes:** Generic sprite cleanup via GetFirstSprite() loop—does not directly clear 5D arrays, relies on sprite system to manage. Comment at line 3671 hints at potential issue: shared sprite references (multiple pointers to same sprite) are freed correctly, but sprite "prefs" (preferences/metadata?) may not be. Called at program exit or on critical error.

---

### `void InitGame(Game* g, int water, float pos[3], float yaw, float dir, float lt[4], uint64_t stamp)` (game.cpp:3678-4000+)

**Signature:** `void InitGame(Game* g, int water, float pos[3], float yaw, float dir, float lt[4], uint64_t stamp)`

**Purpose:** Initializes game state (game object, player struct, input, inventory) and spawns NPCs (enemies and buddies) at world load time.

**Called by:** game_web.cpp (web game init), mainmenu.cpp (line 1552, level transition), term.cpp (line 433, terminal game init), game.cpp (line 5476, test mode/restart).

**Calls:** memset (line 3685-3688), fast_srand (line 3700), enemygen_head iteration (line 3703), malloc for NPC allocation, CreateItem, CreatePhysics, CreateInst, AttachInst, SetInstSpriteData, GetSprite.

**Globals read:** terrain, world, enemygen_head (linked list of spawn generators), player_head, player_tail, fast_rand(), item_proto_lib.

**Globals mutated:** g->menu_depth, perspective, blood, consume_anims, inventory, input, player, items_* counters, prev_grounded, cam_shift, keyb_key[], player_head, player_tail (NPC linked list), stamp for RNG seed.

**Side effects:** Dynamic memory allocation (enemy/buddy objects), world instance creation, physics subsystem init, RNG seed reset, enemy/buddy spawning with equipment and items.

**Notes:** Initializes game struct (lines 3680-3698): menu depth = -1, perspective/blood true, zero out anims/inventory/input/player. Resets RNG with stamp (line 3700). Spawns enemies from enemygen_head (lines 3703-3839, #ifndef EDITOR): for each EnemyGen, creates alive_max enemies with randomized equipment (armor/helmet/shield/sword-or-crossbow ratios), pre-allocates inventory items (armor/helmet/shield/weapon per spawn), creates world instances via CreateInst. Spawns buddies (lines 3979-3999, #ifndef EDITOR): 2 buddies, friendly color 0, same equipment randomization. Comment at line 3677: "CreateGame will be called before loading world!!!" Critical initialization—sets up all NPC state and physics before rendering begins. Lines 3843-3976 contain #if 0 (disabled) code for alternative enemy spawn without EnemyGen system.

---

## Summary

**Lines 1-4000 analyzed:** 14 functions (1 overload pair for ConvertToCP437)

| Function | Category | Callers | Key Operations |
|----------|----------|---------|-----------------|
| ConvertToCP437 (2 overloads) | Encoding | game_api, mainmenu, networking | UTF-8 → CP437 lookup |
| ChatLog | Logging | Server::Proc, Game::OnRender | stdout printf |
| WriteJsonString | I/O | WriteShotJson | JSON escaping |
| WriteShotJson | I/O | Game::OnRender | Game state dump to JSON |
| GetGamePadConfPath | File ops | ReadGamePadConf, WriteGamePadConf | Path sanitization |
| ReadGamePadConf | I/O | Game::OnPad | Binary config read |
| WriteGamePadConf | I/O | Game::OnPad | Binary config write |
| ReadConf | I/O | CreateGame | Settings load |
| WriteConf | I/O | Game::OnRender | Settings persist |
| Server::Proc | Networking | game_api receiver | Network message dispatch |
| LoadSprites | Init | CreateGame, mainmenu | 5D sprite array population |
| FreeSprites | Cleanup | CreateGame, mainmenu | Memory deallocation |
| InitGame | Init | CreateGame, game_web | NPC spawn, physics init |

**Key architectural patterns:**
- **5D Equipment Arrays:** player[color][armor][helmet][shield][weapon] enables O(1) sprite lookup
- **Networking:** Client-server model; Server::Proc handles join/pose/chat/lag messages
- **Persistence:** ReadConf/WriteConf for settings, GetGamePadConfPath for safe path construction
- **Initialization:** LoadSprites → InitGame pipeline critical for startup
- **NPC Spawning:** InitGame iterates enemygen_head; allocates equipment and items per spawn

---

### `LoadSpriteBP` (game.cpp:3240-3249)

**Signature:** `Sprite* LoadSpriteBP(const char* name, const uint8_t* recolor, bool detached)`

**Purpose:** Load a sprite from the sprites/ directory using base_path prefix

**Called by:**
- LOAD_SPRITE macro (game.cpp:3251, used throughout LoadSprites)
- LoadSprites (line 3253+)

**Calls:** LoadSprite (core sprite loading function)

**Globals read:** base_path (file system base path for asset loading)

**Globals mutated:** None

**Side effects:** Loads sprite file from disk, allocates sprite data

**Notes:** Wrapper around LoadSprite that prepends "sprites/" to the name and uses base_path. Passes recolor palette and detached flag through to LoadSprite. LOAD_SPRITE macro uses this with recolor=0 and detached=false for standard sprite loading.

---

### `GetSprite` (game.cpp:3531-3662)

**Signature:** `Sprite* GetSprite(const SpriteReq* req, int clr)`

**Purpose:** Retrieve sprite pointer for given equipment state and color index

**Called by:**
- Game::Render (player and NPC sprite lookup)
- asciiid.cpp (sprite preview rendering)

**Calls:** None (direct array access)

**Globals read:**
- wolf[] (dismounted wolf sprite array)
- bee[] (dismounted bee sprite array)
- Equipment 5D arrays (sprite[action][weapon][shield][helmet][armor])

**Globals mutated:** None (read-only lookup)

**Side effects:** None (returns pointer to existing sprite)

**Notes:** Special handling for WOLF and BEE kinds with MOUNT::NONE (uses simple 2-element arrays instead of 5D equipment arrays). Returns nullptr for invalid equipment indices (bounds-checked). Equipment arrays are 5D indexed: [action][weapon][shield][helmet][armor]. Color index (clr) selects from color variant array (0 or 1 for player colors).

---
