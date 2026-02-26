# Asciicker Game Logic System - C++ Architecture Documentation

This document provides comprehensive documentation of the game logic systems implemented in `game.cpp` and `game.h`. The Asciicker engine uses a C-style struct-based architecture following Data-Oriented Design principles rather than heavy object-oriented programming. The game logic encompasses character state management, equipment systems, combat mechanics, inventory management, AI behaviors, and save/load functionality.

## 1. Main Game State Machine

The Asciicker game implements a hierarchical state machine that manages both global game modes and individual character animation states. Understanding this state machine is fundamental to comprehending how the game transitions between different gameplay phases and how characters behave under various conditions.

### 1.1 Global Game Modes

The `Game` struct in `game.h` contains the `main_menu` boolean flag that controls the high-level game state. When `main_menu` is true, the game renders the main menu interface and pauses all gameplay mechanics. When false, the game enters active gameplay mode where physics simulation, character animation, combat, and AI systems operate normally.

```cpp
struct Game
{
    bool main_menu;
    // ... other fields
    void Render(uint64_t _stamp, AnsiCell* ptr, int width, int height);
};
```

The main menu state is established during game initialization in `CreateGame()`:

```cpp
Game* CreateGame()
{
    Game* g = (Game*)malloc(sizeof(Game));
    memset(g, 0, sizeof(Game));
    ReadConf(g);
    
    #ifdef EDITOR
    g->main_menu = false;
    #else
    g->main_menu = true;  // Start with main menu
    MainMenu_Show();
    #endif
    
    return g;
}
```

The `Render()` method checks this flag and delegates to appropriate rendering handlers:

```cpp
void Game::Render(uint64_t _stamp, AnsiCell* ptr, int width, int height)
{
    // ... FPS and input processing
    
    if (main_menu)
    {
        MainMenu_Render(_stamp, ptr, width, height);
        return;
    }
    
    // ... active gameplay rendering
}
```

### 1.2 Character Animation States

Characters operate within a finite state machine defined by the `ACTION` enum in `game.h`. Each character can be in exactly one of these states at any given time, and transitions between states are controlled by methods on the `Character` struct.

```cpp
struct ACTION { enum
{
    NONE = 0,    // IDLE/MOVE - default walking/idle animation
    ATTACK,      // Attack animation for melee weapons
    FALL,        // Death/fall animation (plays once, transitions to DEAD)
    DEAD,        // Dead state (final frame of FALL, stays indefinitely)
    STAND,       // Standing up animation (currently unused)
    SIZE         // Array bounds sentinel
};};
```

The state transitions are managed through methods on the `Character` struct:

```cpp
struct Character
{
    int anim;           // Current animation index
    int frame;          // Current frame within animation
    uint64_t action_stamp;  // Timestamp when current action started
    bool hit_tested;    // Whether hit test occurred for current attack
    
    SpriteReq req;      // Equipment and action state request
    
    bool SetActionNone(uint64_t stamp);
    bool SetActionAttack(uint64_t stamp);
    bool SetActionFall(uint64_t stamp);
    bool SetActionStand(uint64_t stamp);
    bool SetActionDead(uint64_t stamp);
};
```

Each state transition method validates whether the transition is legal before executing. For example, `SetActionAttack()` checks that the character is not already in a falling or dead state:

```cpp
bool Character::SetActionAttack(uint64_t stamp)
{
    if (req.action == ACTION::ATTACK)
        return true;
    if (req.action == ACTION::FALL || 
        req.action == ACTION::STAND || 
        req.action == ACTION::DEAD)
        return false;  // Cannot attack while falling/standing/dead
    
    int old = req.action;
    req.action = ACTION::ATTACK;
    
    Sprite* spr = GetSprite(&req, clr);
    if (!spr)
    {
        req.action = old;
        return false;
    }
    sprite = spr;
    
    // Crossbow uses static sprite during attack
    if (req.weapon == WEAPON::REGULAR_CROSSBOW)
    {
        anim = 0;
        frame = 0;
    }
    else
    {
        anim = 0;
        frame = 2;  // Start at attack frame
    }
    action_stamp = stamp;
    hit_tested = false;  // Reset hit test flag
    
    return true;
}
```

### 1.3 State Transition Rules

The character state machine enforces specific transition rules that prevent invalid state combinations. The `SetActionFall()` method, for instance, prevents transitioning to the falling state if the character is already dead:

```cpp
bool Character::SetActionFall(uint64_t stamp)
{
    if (req.action == ACTION::FALL)
        return true;
    
    if (req.action == ACTION::DEAD)
        return false;  // Cannot fall if already dead
    
    // ... sprite and animation setup
}
```

The state machine also handles automatic transitions. When a fall animation completes, the character automatically transitions to the dead state:

```cpp
case ACTION::FALL:
{
    int frame = (int)((_stamp - h->action_stamp) / stand_us_per_frame);
    if (frame >= h->sprite->anim[h->anim].length)
        h->SetActionDead(_stamp);  // Auto-transition to DEAD
    else
        h->frame = h->sprite->anim[h->anim].length-1 - frame;
    break;
}
```

### 1.4 Animation Frame Timing

Each animation state uses specific timing constants defined in `game.cpp`:

```cpp
static const int stand_us_per_frame = 30000;   // 30ms per frame for idle
static const int fall_us_per_frame = 30000;    // 30ms per frame for death
static const int attack_us_per_frame = 20000; // 20ms per frame for attacks
```

The frame calculation uses these constants along with the action stamp to determine the current animation frame:

```cpp
int frame_index = (int)((_stamp - player.action_stamp) / attack_us_per_frame);
```

## 2. Entity Systems

The Asciicker engine implements a hierarchical entity system that supports both player-controlled characters and AI-controlled NPCs. The system uses inheritance and composition to share common functionality while allowing specialized behaviors.

### 2.1 Character Base Class

The `Character` struct serves as the foundation for all game entities that can be positioned in the world and animated. It contains essential properties for physics simulation, rendering, and state management:

```cpp
struct Character
{
    Sprite* sprite;           // Current sprite for rendering
    int anim;                 // Current animation index
    int frame;                // Current frame within animation
    float pos[3];             // World position (x, y, z)
    float dir;                // Direction facing (degrees)
    
    float impulse[2];         // External impulse forces
    
    uint64_t action_stamp;    // Timestamp of current action start
    bool hit_tested;          // Attack hit test flag
    int HP, MAX_HP;          // Current and maximum health
    
    Character* prev;          // Linked list previous
    Character* next;          // Linked list next
    
    SpriteReq req;            // Equipment request for sprite selection
    
    int leak;                 // Blood/guts particle count
    int leak_steps;           // Steps since last blood leak
    
    Inst* inst;               // World instance (for server players)
    int clr;                 // Color palette index
    int stuck;                // Stuck counter for AI
    int around;               // Direction preference when stuck
    float unstuck[2][3];     // Position history for stuck detection
    
    void* data;               // Physics state pointer
    void* gen;                // Enemy generator for reviving
    Character* master;        // Master character (for followers)
    Character* target;        // Current combat target
    Character* shoot_by;      // Character being shot by
    uint64_t shoot_by_stamp; // Timestamp of shooting
    int followers;            // Number of followers
    bool jump;                // Jump request flag
    bool enemy;               // True for enemies, false for allies
};
```

### 2.2 Human Player Class

The `Human` struct extends `Character` to add player-specific functionality including statistics, equipment management, and communication:

```cpp
struct Human : Character
{
    char name[32*4];          // UTF-8 player name
    char name_cp437[32];      // CP437 encoded name for terminal
    
    int level;                // Player level
    int max_xp;               // XP required for next level
    int cur_xp;               // Current XP
    
    int pr;                   // Reputation (-evil to +good)
    
    int max_hp, cur_hp;       // Health
    int max_mp, cur_mp;       // Mana
    int max_speed, cur_speed;// Movement speed
    int max_power, cur_power; // Power stat
    
    // Nutrition values (0-6 range)
    int prot_hit;
    int prot_fire;
    int nutr_vits; 
    int nutr_mins;
    int nutr_prots;
    int nutr_fats;
    int nutr_carbs;
    int nutr_water;
    
    // Equipment setters
    bool SetWeapon(int w);
    bool SetShield(int s);
    bool SetHelmet(int h);
    bool SetArmor(int a);
    bool SetMount(int m);
    
    // Communication
    void Say(const char* str, int len, uint64_t stamp);
    
    TalkBox* talk_box;        // Active talk input box
    
    struct Talk
    {
        uint64_t stamp;
        TalkBox* box;
        float pos[3];
    };
    
    int talks;
    Talk talk[3];              // Up to 3 active talk bubbles
    
    // Ranged combat
    Character* shoot_target;
    uint64_t shoot_stamp;
    float shoot_from[3];
    float shoot_to[3];
    bool shooting;
};
```

### 2.3 NPC Entity Types

The engine defines two NPC types that combine character functionality with item ownership:

```cpp
struct NPC_Creature : Character, ItemOwner {};
struct NPC_Human : Human, ItemOwner {};
```

The `ItemOwner` struct provides inventory management capability:

```cpp
struct ItemOwner
{
    static const int max_items = 5;
    int items;
    struct
    {
        Item* item;
        int story_id;
        bool in_use;
    } has[max_items];
};
```

### 2.4 Entity Management

All game entities are managed through a doubly-linked list structure with global head and tail pointers:

```cpp
Character* player_head = 0;
Character* player_tail = 0;
```

When the game initializes, entities are added to this list:

```cpp
void InitGame(Game* g, /* ... */)
{
    // ... player initialization
    
    g->player.prev = 0;
    g->player.next = player_head;
    if (player_head)
        player_head->prev = &g->player;
    else
        player_tail = &g->player;
    player_head = &g->player;
    
    // NPCs are added similarly
    enemy->prev = 0;
    enemy->next = player_head;
    
    if (!player_tail)
        player_tail = enemy;
    else
        player_tail->prev = enemy;
    
    player_head = enemy;
}
```

The entity list is traversed during each frame to update physics, AI, and rendering:

```cpp
Character* h = player_head;
while (h)
{
    if (h->data != physics)  // Skip local player (uses separate physics)
    {
        // Update NPC physics
        Physics* p = (Physics*)h->data;
        // ... AI processing
    }
    h = h->next;
}
```

### 2.5 Entity Initialization

NPC entities are spawned during game initialization based on enemy generator configurations:

```cpp
void InitGame(Game* g, /* ... */)
{
    #ifndef EDITOR
    EnemyGen* eg = enemygen_head;
    while (eg)
    {
        for (int i = 0; i < eg->alive_max; i++)
        {
            NPC_Human* enemy = (NPC_Human*)malloc(sizeof(NPC_Human));
            memset(enemy, 0, sizeof(NPC_Human));
            
            enemy->MAX_HP = 100;
            enemy->HP = (i + 1) * enemy->MAX_HP / eg->alive_max;
            enemy->enemy = true;
            
            // Random equipment based on enemy generator probabilities
            enemy->req.armor = fast_rand() % 11 < eg->armor ? 
                ARMOR::NONE : ARMOR::REGULAR_ARMOR;
            enemy->req.helmet = fast_rand() % 11 < eg->helmet ? 
                HELMET::NONE : HELMET::REGULAR_HELMET;
            enemy->req.shield = fast_rand() % 11 < eg->shield ? 
                SHIELD::NONE : SHIELD::REGULAR_SHIELD;
            enemy->req.weapon = fast_rand() % (eg->sword + eg->crossbow + 1) < eg->sword ?
                WEAPON::REGULAR_SWORD : WEAPON::REGULAR_CROSSBOW;
            
            // ... additional initialization
        }
        eg = eg->next;
    }
    #endif
}
```

## 3. Equipment System

The equipment system uses a sophisticated 5-dimensional array lookup mechanism for sprite selection. This design enables O(1) sprite retrieval based on any combination of equipment pieces, eliminating runtime sprite composition overhead.

### 3.1 Equipment Enums

The equipment system defines independent enums for each equipment slot, allowing Cartesian product combinations:

```cpp
struct WEAPON { enum
{
    NONE = 0,
    REGULAR_SWORD,
    REGULAR_CROSSBOW,
    SIZE
};};

struct SHIELD { enum
{
    NONE = 0,
    REGULAR_SHIELD,
    SIZE
};};

struct HELMET { enum
{
    NONE = 0,
    REGULAR_HELMET,
    SIZE
};};

struct ARMOR { enum
{
    NONE = 0,
    REGULAR_ARMOR,
    SIZE
};};

struct MOUNT { enum
{
    NONE = 0,
    WOLF,
    BEE,
    SIZE
};};
```

### 3.2 Sprite Request Structure

The `SpriteReq` struct encapsulates all equipment state needed for sprite selection:

```cpp
struct SpriteReq
{
    enum KIND
    {
        HUMAN = 0,  // Player or NPC human
        WOLF = 1,   // Dismounted wolf companion
        BEE = 2,    // Dismounted bee companion
    };
    
    KIND kind;      // Determines sprite array family
    
    // Only used when kind == HUMAN
    int mount;      // MOUNT::NONE, WOLF, or BEE
    int action;     // Current action (NONE, ATTACK, FALL, etc.)
    int armor;      // ARMOR index
    int helmet;     // HELMET index
    int shield;    // SHIELD index
    int weapon;    // WEAPON index
};
```

### 3.3 5-Dimensional Sprite Arrays

The sprite arrays are declared as 5-dimensional arrays indexed by color, armor, helmet, shield, and weapon:

```cpp
// Main player sprites
Sprite* player[2][ARMOR::SIZE][HELMET::SIZE][SHIELD::SIZE][WEAPON::SIZE] = { 0 };
Sprite* player_fall[2][ARMOR::SIZE][HELMET::SIZE][SHIELD::SIZE][WEAPON::SIZE] = { 0 };
Sprite* player_attack[2][ARMOR::SIZE][HELMET::SIZE][SHIELD::SIZE][WEAPON::SIZE] = { 0 };

// Wolf mount variants
Sprite* wolfie[2][ARMOR::SIZE][HELMET::SIZE][SHIELD::SIZE][WEAPON::SIZE] = { 0 };
Sprite* wolfie_attack[2][ARMOR::SIZE][HELMET::SIZE][SHIELD::SIZE][WEAPON::SIZE] = { 0 };

// Bee mount variants
Sprite* bigbee[2][ARMOR::SIZE][HELMET::SIZE][SHIELD::SIZE][WEAPON::SIZE] = { 0 };
Sprite* bigbee_attack[2][ARMOR::SIZE][HELMET::SIZE][SHIELD::SIZE][WEAPON::SIZE] = { 0 };

// Dismounted companion sprites (simple 2-element arrays)
Sprite* wolf[2] = { 0 };
Sprite* bee[2] = { 0 };
```

The first dimension (2) represents color palettes: index 0 for player/buddy (default palette) and index 1 for enemy (red-shifted palette).

### 3.4 Sprite Loading

Sprites are loaded during initialization using a nested loop that iterates all equipment combinations:

```cpp
void LoadSprites()
{
    // Load base sprites
    wolf[0] = LoadSpriteBP("wolfie.xp", 0, false);
    wolf[1] = LoadSpriteBP("wolfie.xp", wolf_recolor, false);
    
    // Load all 5D equipment combinations
    for (int a = 0; a < ARMOR::SIZE; a++)
    {
        for (int h = 0; h < HELMET::SIZE; h++)
        {
            for (int s = 0; s < SHIELD::SIZE; s++)
            {
                for (int c = 0; c < 2; c++)  // color
                {
                    for (int w = 0; w < WEAPON::SIZE; w++)
                    {
                        char name[64];
                        
                        // Load player sprites
                        sprintf(name, "player-%x%x%x%x.xp", a, h, s, w);
                        player[c][a][h][s][w] = LoadSpriteBP(name, recolor[c], false);
                        
                        // Load death sprites
                        sprintf(name, "plydie-%x%x%x%x.xp", a, h, s, w);
                        player_fall[c][a][h][s][w] = LoadSpriteBP(name, recolor[c], false);
                        
                        // Load wolf mount sprites
                        sprintf(name, "wolfie-%x%x%x%x.xp", a, h, s, w);
                        wolfie[c][a][h][s][w] = LoadSpriteBP(name, recolor[c], false);
                        
                        // Load bee mount sprites
                        sprintf(name, "bigbee-%x%x%x%x.xp", a, h, s, w);
                        bigbee[c][a][h][s][w] = LoadSpriteBP(name, recolor[c], false);
                    }
                    
                    // Load attack sprites (weapons only, not NONE)
                    for (int w = 1; w < WEAPON::SIZE; w++)
                    {
                        sprintf(name, "attack-%x%x%x%x.xp", a, h, s, w);
                        player_attack[c][a][h][s][w] = LoadSpriteBP(name, recolor[c], false);
                        
                        sprintf(name, "wolack-%x%x%x%x.xp", a, h, s, w);
                        wolfie_attack[c][a][h][s][w] = LoadSpriteBP(name, recolor[c], false);
                    }
                }
            }
        }
    }
}
```

### 3.5 Sprite Selection Algorithm

The `GetSprite()` function performs O(1) lookup based on the SpriteReq:

```cpp
Sprite* GetSprite(const SpriteReq* req, int clr)
{
    assert(req);
    
    // Handle dismounted companions (simple 2-element arrays)
    if (req->kind == SpriteReq::WOLF)
    {
        if (req->action == ACTION::NONE &&
            req->weapon == WEAPON::NONE &&
            req->shield == SHIELD::NONE &&
            req->helmet == HELMET::NONE &&
            req->armor == ARMOR::NONE &&
            req->mount == MOUNT::NONE)
        {
            return wolf[clr];
        }
        return 0;
    }
    
    if (req->kind == SpriteReq::BEE)
    {
        if (req->action == ACTION::NONE &&
            req->weapon == WEAPON::NONE &&
            req->shield == SHIELD::NONE &&
            req->helmet == HELMET::NONE &&
            req->armor == ARMOR::NONE &&
            req->mount == MOUNT::NONE)
        {
            return bee[clr];
        }
        return 0;
    }
    
    // Bounds checking
    if (req->action < 0 || req->action >= ACTION::SIZE)
        return 0;
    if (req->weapon < 0 || req->weapon >= WEAPON::SIZE)
        return 0;
    if (req->shield < 0 || req->shield >= SHIELD::SIZE)
        return 0;
    if (req->helmet < 0 || req->helmet >= HELMET::SIZE)
        return 0;
    if (req->armor < 0 || req->armor >= ARMOR::SIZE)
        return 0;
    
    // Select array based on mount type
    switch (req->mount)
    {
        case MOUNT::NONE:
        {
            switch (req->action)
            {
                case ACTION::NONE:
                    return player[clr][req->armor][req->helmet][req->shield][req->weapon];
                case ACTION::ATTACK:
                    // Crossbow uses idle sprite during attack
                    if (req->weapon == WEAPON::REGULAR_CROSSBOW)
                        return player[clr][req->armor][req->helmet][req->shield][req->weapon];
                    else
                        return player_attack[clr][req->armor][req->helmet][req->shield][req->weapon];
                case ACTION::FALL:
                case ACTION::DEAD:
                case ACTION::STAND:
                    return player_fall[clr][req->armor][req->helmet][req->shield][req->weapon];
            }
            return 0;
        }
        
        case MOUNT::WOLF:
        {
            switch (req->action)
            {
                case ACTION::NONE:
                    return wolfie[clr][req->armor][req->helmet][req->shield][req->weapon];
                case ACTION::ATTACK:
                    if (req->weapon == WEAPON::REGULAR_CROSSBOW)
                        return wolfie[clr][req->armor][req->helmet][req->shield][req->weapon];
                    else
                        return wolfie_attack[clr][req->armor][req->helmet][req->shield][req->weapon];
                case ACTION::FALL:
                case ACTION::DEAD:
                case ACTION::STAND:
                    return wolfie_fall[clr][req->armor][req->helmet][req->shield][req->weapon];
            }
            return 0;
        }
        
        case MOUNT::BEE:
        {
            switch (req->action)
            {
                case ACTION::NONE:
                    return bigbee[clr][req->armor][req->helmet][req->shield][req->weapon];
                case ACTION::ATTACK:
                    if (req->weapon == WEAPON::REGULAR_CROSSBOW)
                        return bigbee[clr][req->armor][req->helmet][req->shield][req->weapon];
                    else
                        return bigbee_attack[clr][req->armor][req->helmet][req->shield][req->weapon];
                case ACTION::FALL:
                case ACTION::DEAD:
                case ACTION::STAND:
                    return bigbee_fall[clr][req->armor][req->helmet][req->shield][req->weapon];
            }
            return 0;
        }
    }
    
    return 0;
}
```

### 3.6 Equipment Modification

Equipment changes are performed through setter methods that validate the change and update the sprite accordingly:

```cpp
bool Human::SetWeapon(int w)
{
    if (req.action == ACTION::ATTACK)
        return false;  // Cannot change weapon during attack
    if (w == req.weapon)
        return true;
    
    int old = req.weapon;
    req.weapon = w;
    
    Sprite* spr = GetSprite(&req, clr);
    if (!spr)
    {
        req.weapon = old;
        return false;
    }
    sprite = spr;
    
    return true;
}
```

## 4. Combat System

The combat system implements a hybrid approach combining animation-driven hit detection with distance-based targeting. Both melee and ranged combat are supported, with distinct mechanics for each type.

### 4.1 Melee Combat

Melee combat uses an animation-driven hit testing approach where damage is applied at a specific frame during the attack animation. The sword attack animation consists of multiple frames, and the hit test occurs at frame 21:

```cpp
case PLAYER_WEAPON_INDEX::SWORD:
{
    // Animation frames for sword attack
    static const int frames[] = { 7,7,7,1,1,1,0,0,0,0,0,0,0,0,
                                  0,1,2,3,4,4,4,5,5,5,5,5,5,5,5,5,5,5,5,5,
                                  6,6,6,6,6,6,6 };
    
    int frame_index = (int)((_stamp - player.action_stamp) / attack_us_per_frame);
    
    // Hit test occurs once at frame 21
    if (frame_index > 21 && !h->hit_tested)
    {
        h->hit_tested = true;
        
        // Find closest enemy within range
        Character* h2 = player_head;
        Character* ch = 0;
        float cd = 0;
        while (h2)
        {
            if (h2->data != physics && h2->enemy)
            {
                float dx = h2->pos[0] - h->pos[0];
                float dy = h2->pos[1] - h->pos[1];
                float dd = dx * dx + dy * dy;
                
                // Check if within sword range (4 units)
                if (dd < 4*4)
                {
                    // Check direction (within 90 degrees of facing)
                    float dif = (float)(atan2(dy, dx) * 180 / M_PI + 90 - h->dir);
                    dif = fmodf(dif, 360);
                    if (dif < -180) dif += 360;
                    if (dif > 180) dif -= 360;
                    
                    if (fabsf(dif) <= 90)
                    {
                        if (!ch || cd < dd)
                        {
                            cd = dd;
                            ch = h2;
                        }
                    }
                }
            }
            h2 = h2->next;
        }
        
        h->target = ch;
        
        // Apply damage if target in range
        if (h->target && h->target->enemy != h->enemy)
        {
            float dx = h->target->pos[0] - h->pos[0];
            float dy = h->target->pos[1] - h->pos[1];
            float d = sqrtf(dx*dx + dy*dy);
            
            if (d < 3)
            {
                int hp = h->target->HP;
                h->target->HP -= rand() % 100;  // Random damage 0-99
                
                // Blood particle effect
                h->target->leak += (hp - h->target->HP) / 5;
                float r = fast_rand() % 20 * 0.1f + 0.6f;
                if (hp > 0 && h->target->HP <= 0)
                    r = fmaxf(r, 2.5f);
                
                // Paint blood on terrain
                if (blood)
                {
                    float dR = 1.0;
                    float dr = dR * sqrtf((fast_rand() & 0xfff) / (float)0xfff);
                    float dt = (fast_rand() & 0xfff) * (float)(2.0 * M_PI) / (float)0xfff;
                    float xy[2] = { h->target->pos[0] + dr * cosf(dt),
                                   h->target->pos[1] + dr * sinf(dt) };
                    PaintTerrain(xy, r, 5);
                }
                
                // Knockback impulse
                float d = 15.0f / sqrtf(dx*dx + dy*dy);
                h->target->impulse[0] += dx * d;
                h->target->impulse[1] += dy * d;
                
                // Handle target death
                if (h->target->HP <= 0)
                {
                    if (h->target->req.mount != MOUNT::NONE)
                    {
                        // Dismount instead of dying
                        ((Human*)h->target)->SetMount(MOUNT::NONE);
                        h->target->HP = hp;
                    }
                    else
                    {
                        // Death animation
                        h->target->dir = (float)(atan2(-dy, -dx) * 180 / M_PI + 90);
                        Physics* p = (Physics*)h->target->data;
                        SetPhysicsDir(p, h->target->dir);
                        h->target->HP = 0;
                        h->target->SetActionFall(_stamp);
                    }
                }
            }
        }
    }
    
    // Update animation frame
    if (frame_index >= sizeof(frames) / sizeof(int))
        player.SetActionNone(_stamp);
    else
        player.frame = frames[frame_index];
    break;
}
```

### 4.2 Ranged Combat

Crossbow combat operates differently from melee, using projectile-based hit detection:

```cpp
case PLAYER_WEAPON_INDEX::CROSSBOW:
{
    int frame_index = (int)((_stamp - player.action_stamp) / attack_us_per_frame);
    
    // Movement frozen during crossbow attack
    io.x_force = 0;
    io.y_force = 0;
    
    int frames = 10;
    
    // Arrow release at frame 10 (halfway through)
    if (2 * frame_index >= frames)
    {
        // Arrow should be released here
    }
    
    if (frame_index >= frames)
        player.SetActionNone(_stamp);
    break;
}
```

The actual shooting mechanics are handled separately when the player triggers a shot:

```cpp
if (input.shoot && 
    player.req.weapon == WEAPON::REGULAR_CROSSBOW &&
    stamp - player.shoot_stamp > 1000000)  // 1 second cooldown
{
    if (player.SetActionAttack(_stamp))
    {
        // Calculate shot trajectory
        // Find closest enemy within range and angle
        // Perform raycast to check for obstacles
        // Set shoot_from and shoot_to positions
    }
}
```

### 4.3 Combat Damage System

The damage calculation is currently simple, using random values without equipment modifiers:

```cpp
h->target->HP -= rand() % 100;  // 0-99 damage
```

Future implementations may add:
- Base damage from weapon type
- Armor damage reduction
- Critical hit chance
- Damage modifiers from stats

### 4.4 Knockback Physics

When a character takes damage, a knockback impulse is applied:

```cpp
float d = 15.0f / sqrtf(dx*dx + dy*dy);
h->target->impulse[0] += dx * d;
h->target->impulse[1] += dy * d;
```

The knockback magnitude scales inversely with distance, ensuring closer hits produce stronger knockback. Death produces larger knockback for more dramatic ragdoll effects.

## 5. Inventory System

The inventory system implements a grid-based storage mechanism with bitmask collision detection, supporting item pickup, dropping, equipment management, and consumption.

### 5.1 Inventory Structure

The inventory is defined in a separate `inventory.h` header (referenced in `game.h`):

```cpp
struct Inventory
{
    static const int MAX_ITEMS = 64;
    int my_items;
    
    struct MyItem
    {
        Item* item;
        int xy[2];         // Grid position
        int story_id;
        const char* desc;
        bool in_use;       // Whether item is equipped/active
    } my_item[MAX_ITEMS];
    
    int width;             // Grid width in cells
    int height;           // Grid height in cells
    uint8_t* bitmask;     // Occupancy bitmask
    
    // Layout and rendering
    int layout_x, layout_y;
    int layout_width, layout_height;
    int layout_frame[4];
    int layout_reps[4];
    int scroll;
    int layout_max_scroll;
    int focus;
    bool animate_scroll;
    int smooth_scroll;
    
    // Methods
    void UpdateLayout(int screen_width, int screen_height, int scene_shift, int bars_pos);
    bool InsertItem(Item* item, int xy[2], const char* desc, int* story_id);
    void RemoveItem(int index, float world_pos[3], float dir);
};
```

### 5.2 Item Pickup

Items are picked up using a first-fit algorithm that finds the first available space in the grid:

```cpp
bool Game::PickItem(Item* item)
{
    int iw = (item->proto->sprite_2d->atlas->width + 1) / 4;
    int ih = (item->proto->sprite_2d->atlas->height + 1) / 4;
    
    int xx = inventory.width - iw + 1;
    for (int y = inventory.height - ih; y >= 0; y--)
    {
        for (int x = 0; x < xx; x++)
        {
            bool ok = true;
            for (int v = y; v < y + ih && ok; v++)
            {
                for (int u = x; u < x + iw && ok; u++)
                {
                    int i = u + v * inventory.width;
                    if (inventory.bitmask[i >> 3] & 1 << (i & 7))
                        ok = false;
                }
            }
            
            if (!ok) continue;
            
            // Found valid position - insert item
            const char* desc = item->proto->desc;
            int story_id = GetInstStoryID(item->inst);
            // ... story API call ...
            
            int xy[2] = { x, y };
            inventory.InsertItem(item, xy, desc, &story_id);
            return true;
        }
    }
    return false;
}
```

### 5.3 Item Dropping

Dropping an item removes it from the inventory and places it in the world:

```cpp
bool Game::DropItem(int index)
{
    assert(index >= 0 && index < inventory.my_items);
    
    // Calculate drop position near player
    float ang = (float)(rand() % 360);
    double dpos[3] =
    {
        player.pos[0] + (float)(2 * cos(ang*M_PI / 180)),
        player.pos[1] + (float)(2 * sin(ang*M_PI / 180)),
        0
    };
    
    // Find ground height
    double downward[3] = { 0, 0, -1 };
    double ret[4];
    double z = 0;
    bool ok = HitTerrain(terrain, dpos, downward, ret, 0);
    
    if (ok)
    {
        z = ret[2];
        dpos[2] = player.pos[2] + 3 * HEIGHT_SCALE;
        // Check world objects too
        // ... collision detection ...
    }
    
    if (ok)
    {
        float _pos[3] = { (float)dpos[0], (float)dpos[1], (float)z };
        
        Inventory::MyItem* mi = inventory.my_item + index;
        // ... story API call for permission ...
        
        inventory.RemoveItem(index, _pos, prev_yaw);
    }
    
    return ok;
}
```

### 5.4 Equipment Management

Equipment items are managed through the inventory interface with in-use tracking:

```cpp
void Game::ExecuteItem(int my_item)
{
    Inventory::MyItem* mi = inventory.my_item + my_item;
    Item* item = mi->item;
    
    switch (item->proto->kind)
    {
        case 'W':  // Weapon
        {
            if (inventory.my_item[my_item].in_use)
            {
                // Unequip
                if (player.SetWeapon(PLAYER_WEAPON_INDEX::WEAPON_NONE))
                {
                    inventory.my_item[my_item].in_use = false;
                }
            }
            else
            {
                // Equip - dismount first if needed
                if (player.req.mount != MOUNT::NONE)
                    player.SetMount(MOUNT::NONE);
                if (player.SetWeapon(item->proto->sub_kind))
                {
                    // Unequip other weapons of same type
                    for (int i = 0; i < inventory.my_items; i++)
                    {
                        if (inventory.my_item[i].in_use && 
                            inventory.my_item[i].item->proto->kind == item->proto->kind)
                        {
                            inventory.my_item[i].in_use = false;
                            break;
                        }
                    }
                    inventory.my_item[my_item].in_use = true;
                }
            }
            break;
        }
        
        case 'S':  // Shield
        case 'H':  // Helmet
        case 'A':  // Armor
            // Similar pattern to weapon
            break;
        
        case 'R':  // Ring - toggle in-use
            inventory.my_item[my_item].in_use = !inventory.my_item[my_item].in_use;
            break;
        
        case 'F':  // Food
        case 'P':  // Potion
        case 'D':  // Drink
            // Consume immediately
            if (item->count > 1)
                item->count--;
            else
            {
                // Create consumption animation
                inventory.RemoveItem(my_item, 0, 0);
            }
            break;
    }
}
```

### 5.5 Item Grid Collision

The inventory uses a bitmask for efficient collision detection:

```cpp
bool Game::CheckDrop(int c, int drop_xy[2], AnsiCell* ptr, int width, int height)
{
    // Check if position is within inventory bounds
    // ...
    
    // Calculate grid position from screen coordinates
    int qx = (cp[0] - inventory.layout_x - 4) / 4;
    int qy = (cp[1] - inventory.layout_y - inventory.layout_height + 6 - scroll) / 4;
    
    int qw = (frame->width + 1) / 4;
    int qh = (frame->height + 1) / 4;
    
    // Check bitmask collision
    bool fit = true;
    for (int my = qy; fit && my < qy + qh; my++)
    {
        for (int mx = qx; mx < qx + qw; mx++)
        {
            int m = mx + my * inventory.width;
            if (inventory.bitmask[m >> 3] & (1 << (m & 7)))
            {
                fit = false;
                break;
            }
        }
    }
    
    return fit;
}
```

### 5.6 Consumption Effects

Consuming items triggers specific effects based on item type:

```cpp
for (int i = 0; i < consume_anims; i++)
{
    ConsumeAnim* a = consume_anim + i;
    int elaps = (int)((_stamp - a->stamp) / 50000);
    
    if (elaps >= max_elaps)
    {
        // Apply item effect based on sprite
        if (a->sprite == item_proto_lib[40].sprite_2d)
        {
            // Grey potion - Wolf mount
            player.SetMount(MOUNT::WOLF);
        }
        else if (a->sprite == item_proto_lib[39].sprite_2d)
        {
            // Gold potion - Bee mount
            player.SetMount(MOUNT::BEE);
        }
        else if (a->sprite == item_proto_lib[38].sprite_2d)
        {
            // Cyan potion - Dismount
            player.SetMount(MOUNT::NONE);
        }
        else if (a->sprite == item_proto_lib[34].sprite_2d)
        {
            // Healing potion - Restore HP
            player.HP = player.MAX_HP;
        }
        
        // Remove animation
        consume_anims--;
    }
}
```

## 6. AI System

The AI system implements a behavior tree-like approach with target selection, movement toward targets, stuck detection, and combat engagement.

### 6.1 Target Selection

AI characters continuously evaluate potential targets, prioritizing the closest enemy:

```cpp
// Find closest enemy
Character* enemy_ch = 0;
float enemy_cd = 0;
int enemy_cf = 0;

Character* h2 = player_head;
while (h2)
{
    // Check if this is an enemy
    if (h2->enemy != h->enemy && h2->req.action != ACTION::DEAD)
    {
        float bx = h2->pos[0] - h->pos[0];
        float by = h2->pos[1] - h->pos[1];
        float d = (bx * bx + by * by);
        
        // Reduce distance weighting if being shot by this enemy
        if (h->shoot_by == h2 && 
            stamp > 500000 + h->shoot_by_stamp &&
            stamp < 5000000 + h->shoot_by_stamp)
        {
            d *= 0.2f;  // Higher priority
        }
        
        // Select enemy with lowest weighted distance
        if (!enemy_ch || d * (h2->followers + 4) < enemy_cd * (enemy_cf + 4))
        {
            enemy_cf = h2->followers;
            enemy_cd = d;
            enemy_ch = h2;
        }
    }
    h2 = h2->next;
}

// Set target if within engagement distance
float ret_md = 40;   // Max distance to master to consider combat
float max_ed = 20;   // Max distance to enemy to engage

if (enemy_ch && enemy_cd < max_ed*max_ed && master_distance < ret_md)
{
    h->target = enemy_ch;
    h->target->followers++;
}
else if (h->master)
{
    // No enemy - follow master
    h->target = h->master;
    h->target->followers++;
}
```

### 6.2 Movement Behavior

Once a target is selected, the AI calculates movement forces to approach the target:

```cpp
if (h->target)
{
    float dx = h->target->pos[0] - h->pos[0];
    float dy = h->target->pos[1] - h->pos[1];
    float d = sqrtf(dx*dx + dy*dy);
    
    // Reduce speed when close to prevent overshooting
    if (d < 10)
    {
        dx *= 0.7f;
        dy *= 0.7f;
    }
    
    // Approach distance varies by weapon type
    float min_target_dist;
    if (h->req.weapon == WEAPON::REGULAR_CROSSBOW)
        min_target_dist = 10;  // Archers stay back
    else
        min_target_dist = 3;    // Melee engages closely
    
    if (d > min_target_dist)
    {
        if (d > 15)
        {
            pio.x_force = dx / d;
            pio.y_force = dy / d;
        }
        else
        {
            pio.x_force = dx / 15;
            pio.y_force = dy / 15;
        }
    }
    else
    {
        // Close enough - attack
        if (h->target != h->master)
        {
            h->SetActionAttack(_stamp);
        }
    }
}
```

### 6.3 Collision Avoidance

The AI avoids clustering with other allies by adjusting movement when near buddy characters:

```cpp
if (buddy_ch && buddy_cd < buddy_nd*buddy_nd)
{
    // Check if movement direction conflicts with buddy position
    float bx = buddy_ch->pos[0] - h->pos[0];
    float by = buddy_ch->pos[1] - h->pos[1];
    
    if (pio.x_force * bx + pio.y_force * by > 0)
    {
        // Movement would move toward buddy - adjust to slide past
        if (d < max_target_dist && buddy_nn > 1)
        {
            // Too many allies - stop moving
            h->jump = false;
            pio.x_force = 0;
            pio.y_force = 0;
        }
        else
        {
            // Calculate perpendicular force to slide around
            float x1[3] = { bx, by, 0 };
            float y1[3] = { dx, dy, 0 };
            float z1[3];
            CrossProduct(x1, y1, z1);
            CrossProduct(z1, y1, x1);
            
            float len = sqrtf(x1[0] * x1[0] + x1[1] * x1[1]);
            x1[0] *= d / len;
            x1[1] *= d / len;
            
            pio.x_force = 0.1f * x1[0];
            pio.y_force = 0.1f * x1[1];
        }
    }
}
```

### 6.4 Stuck Detection and Resolution

The AI tracks position history to detect when movement is blocked:

```cpp
// Track position changes
float adv[2] = { pio.pos[0] - h->unstuck[1][0], pio.pos[1] - h->unstuck[1][1] };
if (adv[0] * adv[0] + adv[1] * adv[1] > 2.0f)
{
    // Update position history
    h->unstuck[0][0] = h->unstuck[1][0];
    h->unstuck[0][1] = h->unstuck[1][1];
    h->unstuck[0][2] = h->unstuck[1][2];
    h->unstuck[1][0] = pio.pos[0];
    h->unstuck[1][1] = pio.pos[1];
    h->unstuck[1][2] = pio.pos[2];
}

// Stuck detection
int s_stucks = 5;
if (h->stuck < 100 && h->stuck + s * s_stucks >= 100)
{
    // Reset to previous position
    pio.pos[0] = h->pos[0] = h->unstuck[1][0] = h->unstuck[0][0];
    pio.pos[1] = h->pos[1] = h->unstuck[1][1] = h->unstuck[0][1];
    pio.pos[2] = h->pos[2] = h->unstuck[1][2] = h->unstuck[0][2];
}

// Clear stuck counter if making progress
if (fabsf(pio.x_impulse) > 0.001 || fabsf(pio.y_impulse) > 0.001)
{
    h->stuck = 0;
}

// Increment stuck counter and try to resolve
if (h->stuck >= 100)
{
    h->jump = true;  // Try jumping over obstacle
    h->stuck += s * s_stucks;
}

// Check if not making progress toward target
if (s && h->stuck < 100 && fabsf(pio.x_force) + fabsf(pio.y_force) > 0.5)
{
    float dx = h->target->pos[0] - pio.pos[0];
    float dy = h->target->pos[1] - pio.pos[1];
    float d2 = sqrtf(dx*dx + dy*dy);
    
    if (distance - d2 < 0.001 * s)
    {
        h->jump = true;
        
        if (h->stuck < 100)
        {
            h->stuck += s * s_stucks;
            if (h->stuck >= 100)
            {
                h->around = fast_rand() & 1;  // Random direction to go around
            }
        }
    }
}
```

### 6.5 Stuck Resolution Behaviors

The AI implements escalating resolution strategies based on how long it has been stuck:

```cpp
if (h->stuck >= 100 && h->stuck < 200)
{
    // Stage 1: Go opposite direction
    pio.x_force = -pio.x_force;
    pio.y_force = -pio.y_force;
}
else if (h->stuck >= 200 && h->stuck < 300)
{
    // Stage 2: Go around (perpendicular)
    if (h->around == 0)
    {
        float t = pio.x_force;
        pio.x_force = -pio.y_force;
        pio.y_force = t;
    }
    else
    {
        float t = pio.x_force;
        pio.x_force = pio.y_force;
        pio.y_force = -t;
    }
}
else if (h->stuck >= 300 && h->stuck < 400)
{
    // Stage 3: Keep jumping
    // Even if on way back to target
}
else if (h->stuck >= 400)
{
    // Stage 4: Reset stuck counter
    h->stuck = 0;
}
```

## 7. Save/Load System

The save/load system uses simple binary file formats for configuration and JSON for game state snapshots.

### 7.1 Configuration Save/Load

Game settings are stored in a simple binary format:

```cpp
void ReadConf(Game* g)
{
    FILE* f = fopen(GetConfPath(), "rb");
    if (f)
    {
        // Read talk history
        fread(g->talk_mem, sizeof(Game::TalkMem), 4, f);
        
        // Read settings
        fread(&g->perspective, 1, 1, f);
        fread(&g->blood, 1, 1, f);
        fread(&g->mute, 1, 1, f);
        
        fclose(f);
        
        // Apply settings immediately
        AudioMute(g->mute);
    }
}

void WriteConf(Game* g)
{
    FILE* f = fopen(GetConfPath(), "wb");
    if (f)
    {
        // Write talk history
        fwrite(g->talk_mem, sizeof(Game::TalkMem), 4, f);
        
        // Write settings
        fwrite(&g->perspective, 1, 1, f);
        fwrite(&g->blood, 1, 1, f);
        fwrite(&g->mute, 1, 1, f);
        
        fclose(f);
    }
    
    SyncConf();
}
```

### 7.2 Gamepad Configuration

Gamepad mappings are stored separately with automatic path generation:

```cpp
bool GetGamePadConfPath(char* path, const char* name, int axes, int buttons)
{
    const char* cfg = GetConfPath();
    const char* filepart1 = strrchr(cfg, '/');
    const char* filepart2 = strrchr(cfg, '\\');
    
    // ... path construction ...
    
    sprintf(path + pos + 1, "asciicker_(%s)_A%d_B%d.cfg", name, axes, buttons);
    
    // Sanitize path for invalid characters
    // ...
    
    return true;
}

bool ReadGamePadConf(uint8_t map[256], const char* name, int axes, int buttons)
{
    char path[1024];
    if (!GetGamePadConfPath(path, name, axes, buttons))
        return false;
    
    FILE* f = fopen(path, "rb");
    if (!f)
        return false;
    
    int n = 2 * axes + buttons;
    int r = fread(map, 1, n, f);
    fclose(f);
    
    return n == r;
}

bool WriteGamePadConf(const uint8_t* map, const char* name, int axes, int buttons)
{
    char path[1024];
    if (!GetGamePadConfPath(path, name, axes, buttons))
        return false;
    
    FILE* f = fopen(path, "wb");
    if (!f)
        return false;
    
    int n = 2 * axes + buttons;
    int w = fwrite(map, 1, n, f);
    fclose(f);
    
    SyncConf();
    
    return n == w;
}
```

### 7.3 Screenshot State Export

Game state can be exported to JSON for debugging and replay:

```cpp
static void WriteShotJson(const char* path, uint64_t stamp, const PhysicsIO* io, 
                          const Game* g, int width, int height)
{
    if (!path || !io || !g)
        return;
    
    // Normalize light direction
    float lt[3] = { g->light[0], g->light[1], g->light[2] };
    float n = lt[0] * lt[0] + lt[1] * lt[1] + lt[2] * lt[2];
    if (n > 0.001f)
    {
        float inv = 1.0f / sqrtf(n);
        lt[0] *= inv;
        lt[1] *= inv;
        lt[2] *= inv;
    }
    
    FILE* f = fopen(path, "wb");
    if (!f)
        return;
    
    fprintf(f, "{\n");
    fprintf(f, "  \"version\": 1,\n");
    fprintf(f, "  \"stamp\": %llu,\n", (unsigned long long)stamp);
    fprintf(f, "  \"size\": {\"width\": %d, \"height\": %d},\n", width, height);
    
    if (g_loaded_a3d_path[0])
    {
        fprintf(f, "  \"map_path\": \"");
        // Write escaped JSON string
        for (const char* p = g_loaded_a3d_path; *p; ++p)
        {
            if (*p == '\\' || *p == '\"')
                fputc('\\', f);
            fputc(*p, f);
        }
        fprintf(f, "\",\n");
    }
    else
    {
        fprintf(f, "  \"map_path\": null,\n");
    }
    
    fprintf(f, "  \"camera\": {\n");
    fprintf(f, "    \"pos\": [%.4f, %.4f, %.4f],\n", io->pos[0], io->pos[1], io->pos[2]);
    fprintf(f, "    \"yaw\": %.4f,\n", io->yaw);
    fprintf(f, "    \"zoom\": %.4f,\n", g->zoom);
    fprintf(f, "    \"perspective\": %s,\n", g->perspective ? "true" : "false");
    fprintf(f, "    \"scene_shift\": %d,\n", g->scene_shift);
    fprintf(f, "    \"cam_shift\": %d\n", g->cam_shift);
    fprintf(f, "  },\n");
    
    fprintf(f, "  \"player\": {\n");
    fprintf(f, "    \"pos\": [%.4f, %.4f, %.4f],\n", g->player.pos[0], g->player.pos[1], g->player.pos[2]);
    fprintf(f, "    \"dir\": %.4f\n", g->player.dir);
    fprintf(f, "  },\n");
    
    fprintf(f, "  \"light\": {\n");
    fprintf(f, "    \"dir\": [%.4f, %.4f, %.4f],\n", lt[0], lt[1], lt[2]);
    fprintf(f, "    \"ambience\": %.4f\n", g->light[3]);
    fprintf(f, "  },\n");
    
    fprintf(f, "  \"water\": %d\n", g->water);
    fprintf(f, "}\n");
    fclose(f);
}
```

### 7.4 Character State Persistence

Character states can be serialized through the network protocol for multiplayer synchronization:

```cpp
bool Server::Proc(const uint8_t* ptr, int size)
{
    switch (ptr[0])
    {
        case 'j':  // BRC_JOIN - New player joined
        {
            STRUCT_BRC_JOIN* join = (STRUCT_BRC_JOIN*)ptr;
            Human* h = others + join->id;
            memset(h, 0, sizeof(Human));
            
            strcpy(h->name, join->name);
            ConvertToCP437(h->name_cp437, h->name);
            
            // Set equipment state
            h->req.action = (join->am >> 4) & 0xF;
            h->req.mount = join->am & 0xF;
            h->req.armor = (join->sprite >> 12) & 0xF;
            h->req.helmet = (join->sprite >> 8) & 0xF;
            h->req.shield = (join->sprite >> 4) & 0xF;
            h->req.weapon = join->sprite & 0xF;
            
            h->sprite = GetSprite(&h->req, h->clr);
            h->anim = join->anim;
            h->frame = join->frame;
            h->dir = join->dir;
            h->pos[0] = join->pos[0];
            h->pos[1] = join->pos[1];
            h->pos[2] = join->pos[2];
            
            // Create world instance
            int flags = INST_USE_TREE | INST_VISIBLE | INST_VOLATILE;
            h->inst = CreateInst(world, h->sprite, flags, h->pos, h->dir, 
                                 h->anim, h->frame, reps, 0, -1);
            break;
        }
        
        case 'p':  // BRC_POSE - Player state update
        {
            STRUCT_BRC_POSE* pose = (STRUCT_BRC_POSE*)ptr;
            Human* h = others + pose->id;
            
            // Update equipment and state
            h->req.action = (pose->am >> 4) & 0xF;
            h->req.mount = pose->am & 0xF;
            // ... equipment fields ...
            
            h->sprite = GetSprite(&h->req, h->clr);
            h->anim = pose->anim;
            h->frame = pose->frame;
            h->dir = pose->dir;
            h->pos[0] = pose->pos[0];
            h->pos[1] = pose->pos[1];
            h->pos[2] = pose->pos[2];
            
            // Update world instance
            if (h->inst)
            {
                int reps[4];
                UpdateSpriteInst(world, h->inst, h->sprite, h->pos, h->dir, 
                                 h->anim, h->frame, reps);
            }
            break;
        }
    }
    
    return true;
}
```

## Summary

The Asciicker game logic system implements a complete game engine with sophisticated systems for character management, equipment handling, combat, inventory, and AI. The architecture emphasizes performance through data-oriented design and pre-computed sprite arrays, while maintaining flexibility through configurable equipment enums and extensible AI behaviors.

Key architectural decisions include the 5D sprite lookup for O(1) equipment visualization, animation-driven combat hit testing, bitmask-based inventory collision, and multi-stage stuck resolution for AI navigation. The save/load system provides basic persistence for settings and multiplayer synchronization, with JSON export capability for debugging.

This documentation should serve as a comprehensive reference for porting or extending the Asciicker game logic systems.
