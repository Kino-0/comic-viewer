-- SQLite スキーマ

-- マスタテーブル
CREATE TABLE IF NOT EXISTS artists    (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE) STRICT;
CREATE TABLE IF NOT EXISTS groups     (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE) STRICT;
CREATE TABLE IF NOT EXISTS types      (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE COLLATE NOCASE) STRICT;
CREATE TABLE IF NOT EXISTS series     (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE) STRICT;
CREATE TABLE IF NOT EXISTS characters (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE) STRICT;
CREATE TABLE IF NOT EXISTS tags       (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE COLLATE NOCASE) STRICT;
CREATE TABLE IF NOT EXISTS languages  (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE COLLATE NOCASE) STRICT;

-- メインテーブル
CREATE TABLE IF NOT EXISTS items (
    id          INTEGER PRIMARY KEY,
    title       TEXT    NOT NULL,
    type_id     INTEGER NOT NULL,
    language_id INTEGER,
    path        TEXT,
    page_count  INTEGER NOT NULL DEFAULT 0,
    cover_path  TEXT,
    created_at  TEXT DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(type_id) REFERENCES types(id),
    FOREIGN KEY(language_id) REFERENCES languages(id)
) STRICT;

-- 中間テーブル
CREATE TABLE IF NOT EXISTS item_artists (
    artist_id INTEGER,
    item_id   INTEGER,
    PRIMARY KEY (artist_id, item_id),
    FOREIGN KEY (item_id) REFERENCES items(id) ON DELETE CASCADE,
    FOREIGN KEY (artist_id) REFERENCES artists(id) ON DELETE CASCADE
) WITHOUT ROWID, STRICT;
CREATE TABLE IF NOT EXISTS item_groups (
    group_id INTEGER,
    item_id  INTEGER,
    PRIMARY KEY (group_id, item_id),
    FOREIGN KEY (item_id) REFERENCES items(id) ON DELETE CASCADE,
    FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE CASCADE
) WITHOUT ROWID, STRICT;
CREATE TABLE IF NOT EXISTS item_series (
    series_id INTEGER,
    item_id   INTEGER,
    PRIMARY KEY (series_id, item_id),
    FOREIGN KEY (item_id) REFERENCES items(id) ON DELETE CASCADE,
    FOREIGN KEY (series_id) REFERENCES series(id) ON DELETE CASCADE
) WITHOUT ROWID, STRICT;
CREATE TABLE IF NOT EXISTS item_characters (
    character_id INTEGER,
    item_id      INTEGER,
    PRIMARY KEY (character_id, item_id),
    FOREIGN KEY (item_id) REFERENCES items(id) ON DELETE CASCADE,
    FOREIGN KEY (character_id) REFERENCES characters(id) ON DELETE CASCADE
) WITHOUT ROWID, STRICT;
CREATE TABLE IF NOT EXISTS item_tags (
    tag_id  INTEGER,
    item_id INTEGER,
    PRIMARY KEY (tag_id, item_id),
    FOREIGN KEY (item_id) REFERENCES items(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
) WITHOUT ROWID, STRICT;

-- インデックス
CREATE INDEX IF NOT EXISTS idx_items_type_id ON items(type_id);
CREATE INDEX IF NOT EXISTS idx_items_language_id ON items(language_id);
CREATE INDEX IF NOT EXISTS idx_item_artists_item_id ON item_artists(item_id);
CREATE INDEX IF NOT EXISTS idx_item_groups_item_id ON item_groups(item_id);
CREATE INDEX IF NOT EXISTS idx_item_series_item_id ON item_series(item_id);
CREATE INDEX IF NOT EXISTS idx_item_characters_item_id ON item_characters(item_id);
CREATE INDEX IF NOT EXISTS idx_item_tags_item_id ON item_tags(item_id);

-- 初期値
INSERT OR IGNORE INTO types (id, name) VALUES
    (0, 'unknown'),
    (1, 'doujinshi'),
    (2, 'artistcg'),
    (3, 'manga'),
    (4, 'gamecg'),
    (5, 'imageset'),
    (6, 'anime');
