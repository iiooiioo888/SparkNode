-- ═══════════════════════════════════════════════════
--  003: 開發環境種子用戶（供 gateway 預設 author_id）
-- ═══════════════════════════════════════════════════

INSERT INTO users (id, username, email, password_hash, display_name, role)
VALUES (
    '00000000-0000-0000-0000-000000000001'::uuid,
    'dev',
    'dev@sparknode.local',
    'dev_no_password',
    'Development User',
    'admin'
)
ON CONFLICT (id) DO NOTHING;
