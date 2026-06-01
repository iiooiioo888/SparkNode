-- ═══════════════════════════════════════════════════
--  002: 创建故事、节点、边、NPC、记忆等核心表
-- ═══════════════════════════════════════════════════

-- ── 故事/项目 ─────────────────────────────────────
CREATE TABLE IF NOT EXISTS stories (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    title           VARCHAR(255) NOT NULL,
    description     TEXT,
    author_id       UUID NOT NULL REFERENCES users(id),
    genre           VARCHAR(50)[],
    world_rules     JSONB DEFAULT '{}',
    mdp_config      JSONB DEFAULT '{}',
    status          VARCHAR(20) DEFAULT 'draft',
    version         BIGINT DEFAULT 1,
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX idx_stories_author ON stories(author_id);
CREATE INDEX idx_stories_genre ON stories USING GIN(genre);

-- ── 叙事节点 ──────────────────────────────────────
CREATE TABLE IF NOT EXISTS story_nodes (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    story_id        UUID NOT NULL REFERENCES stories(id) ON DELETE CASCADE,
    node_type       VARCHAR(30) NOT NULL,
    title           VARCHAR(255),
    content         TEXT,
    position_x      DOUBLE PRECISION DEFAULT 0,
    position_y      DOUBLE PRECISION DEFAULT 0,
    metadata        JSONB DEFAULT '{}',
    world_snapshot  JSONB DEFAULT '{}',
    llm_provider    VARCHAR(50),
    llm_prompt      TEXT,
    llm_tokens_used INTEGER DEFAULT 0,
    crdt_vector     JSONB DEFAULT '{}',
    version         BIGINT DEFAULT 1,
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX idx_nodes_story ON story_nodes(story_id);
CREATE INDEX idx_nodes_type ON story_nodes(story_id, node_type);

-- ── 叙事边 ────────────────────────────────────────
CREATE TABLE IF NOT EXISTS narrative_edges (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    story_id        UUID NOT NULL REFERENCES stories(id) ON DELETE CASCADE,
    source_node_id  UUID NOT NULL REFERENCES story_nodes(id) ON DELETE CASCADE,
    target_node_id  UUID NOT NULL REFERENCES story_nodes(id) ON DELETE CASCADE,
    edge_type       VARCHAR(30) NOT NULL,
    probability     DOUBLE PRECISION DEFAULT 1.0,
    reward_signal   DOUBLE PRECISION DEFAULT 0.0,
    observer_weight DOUBLE PRECISION DEFAULT 0.0,
    collapse_count  INTEGER DEFAULT 0,
    conditions      JSONB DEFAULT '[]',
    metadata        JSONB DEFAULT '{}',
    created_at      TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX idx_edges_story ON narrative_edges(story_id);
CREATE INDEX idx_edges_source ON narrative_edges(source_node_id);
CREATE INDEX idx_edges_target ON narrative_edges(target_node_id);

-- ── 世界状态快照 (Chronos 引擎) ──────────────────
CREATE TABLE IF NOT EXISTS world_checkpoints (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    story_id        UUID NOT NULL REFERENCES stories(id) ON DELETE CASCADE,
    node_id         UUID REFERENCES story_nodes(id),
    checkpoint_type VARCHAR(20) NOT NULL,
    world_state     JSONB NOT NULL,
    npc_memory_refs JSONB DEFAULT '{}',
    timeline_depth  INTEGER DEFAULT 0,
    parent_id       UUID REFERENCES world_checkpoints(id),
    diff_from_parent JSONB DEFAULT '{}',
    created_at      TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX idx_checkpoints_story ON world_checkpoints(story_id);
CREATE INDEX idx_checkpoints_parent ON world_checkpoints(parent_id);

-- ── NPC 实体 ──────────────────────────────────────
CREATE TABLE IF NOT EXISTS npcs (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    story_id        UUID NOT NULL REFERENCES stories(id) ON DELETE CASCADE,
    name            VARCHAR(255) NOT NULL,
    avatar_url      VARCHAR(500),
    personality     JSONB NOT NULL DEFAULT '{}',
    emotional_state JSONB DEFAULT '{}',
    motivation      TEXT,
    backstory       TEXT,
    temperature     DOUBLE PRECISION DEFAULT 0.7,
    autonomy_level  VARCHAR(20) DEFAULT 'scripted',
    is_alive        BOOLEAN DEFAULT TRUE,
    current_location VARCHAR(255),
    relationships   JSONB DEFAULT '{}',
    dao_address     VARCHAR(42),
    treasury_balance NUMERIC(20,8) DEFAULT 0,
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX idx_npcs_story ON npcs(story_id);

-- ── NPC 海马体记忆 ────────────────────────────────
CREATE TABLE IF NOT EXISTS npc_memories (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    npc_id          UUID NOT NULL REFERENCES npcs(id) ON DELETE CASCADE,
    memory_type     VARCHAR(30) NOT NULL,
    content         TEXT NOT NULL,
    vector_id       VARCHAR(255),
    strength        DOUBLE PRECISION DEFAULT 1.0,
    stability       DOUBLE PRECISION DEFAULT 1.0,
    rehearsal_count INTEGER DEFAULT 0,
    last_accessed   TIMESTAMPTZ DEFAULT NOW(),
    emotional_valence DOUBLE PRECISION DEFAULT 0.0,
    emotional_arousal DOUBLE PRECISION DEFAULT 0.0,
    source_node_id  UUID REFERENCES story_nodes(id),
    involved_npcs   UUID[] DEFAULT '{}',
    dream_generated BOOLEAN DEFAULT FALSE,
    created_at      TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX idx_memories_npc ON npc_memories(npc_id);
CREATE INDEX idx_memories_type ON npc_memories(npc_id, memory_type);
CREATE INDEX idx_memories_strength ON npc_memories(npc_id, strength DESC);

-- ── 生成事件日志 ──────────────────────────────────
CREATE TABLE IF NOT EXISTS generation_events (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    story_id        UUID NOT NULL REFERENCES stories(id),
    event_type      VARCHAR(50) NOT NULL,
    actor_id        UUID,
    payload         JSONB NOT NULL,
    vector_clock    JSONB DEFAULT '{}',
    created_at      TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX idx_events_story ON generation_events(story_id, created_at DESC);