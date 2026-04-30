use rusqlite::{Connection, Result, params};
use std::sync::Mutex;

#[derive(Debug, Default)]
pub struct Info {
    pub gallery_id: Option<i64>,
    pub title: String,
    pub artists: Vec<String>,
    pub groups: Vec<String>,
    pub type_name: String,
    pub series: Vec<String>,
    pub characters: Vec<String>,
    pub tags: Vec<String>,
    pub language: String,
}

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute("PRAGMA foreign_keys = ON;", [])?;
        conn.execute_batch(include_str!("./schema.sql"))?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn insert_info(&self, info: &Info, dir_path: &str) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        // 共通クロージャ: マスタからIDを取得（なければ新規作成）
        let get_or_create_master_id = |table: &str, name: &str| -> Result<i64> {
            let select_sql = format!("SELECT id FROM {} WHERE name = ?1", table);
            let insert_sql = format!("INSERT INTO {} (name) VALUES (?1)", table);

            let mut stmt_select = tx.prepare_cached(&select_sql)?;

            match stmt_select.query_row(params![name], |row| row.get(0)) {
                Ok(id) => Ok(id), // 既存データがあればIDを返す
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    // なければ挿入して新しいIDを返す
                    let mut stmt_insert = tx.prepare_cached(&insert_sql)?;
                    stmt_insert.execute(params![name])?;
                    Ok(tx.last_insert_rowid())
                }
                Err(e) => Err(e),
            }
        };

        // 1. types テーブル
        let type_id: i64 = if info.type_name.is_empty() {
            get_or_create_master_id("types", "unknown")?
        } else {
            get_or_create_master_id("types", &info.type_name)?
        };

        // 2. languages テーブル
        let language_id: Option<i64> = if info.language.is_empty() {
            None
        } else {
            Some(get_or_create_master_id("languages", &info.language)?)
        };

        // 3. items テーブルへの挿入
        let item_id = match info.gallery_id {
            Some(id) => {
                tx.execute(
                    "INSERT OR IGNORE INTO items (id, title, type_id, language_id, path) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![id, info.title, type_id, language_id, dir_path],
                )?;
                id
            }
            None => {
                // 本家のギャラリーIDとの衝突を避けるため、負の数で採番する。
                let min_negative_new_id: i64 = tx
                    .query_row("SELECT MIN(id) FROM items WHERE id < 0", [], |row| {
                        row.get(0)
                    })
                    .unwrap_or(0)
                    - 1;

                tx.execute(
                    "INSERT INTO items (id, title, type_id, language_id, path) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![min_negative_new_id, info.title, type_id, language_id, dir_path],
                )?;
                min_negative_new_id
            }
        };

        // 4. 中間テーブルへ挿入するヘルパークロージャ
        let insert_relations = |table_master: &str,
                                table_rel: &str,
                                master_col: &str,
                                names: &[String]|
         -> Result<()> {
            if names.is_empty() {
                return Ok(());
            }

            let insert_rel_sql = format!(
                "INSERT OR IGNORE INTO {} ({}, item_id) VALUES (?1, ?2)",
                table_rel, master_col
            );
            let mut stmt_insert_rel = tx.prepare_cached(&insert_rel_sql)?;

            for name in names {
                if name.is_empty() {
                    continue;
                }

                let master_id = get_or_create_master_id(table_master, name)?;

                // 中間テーブルへの挿入
                stmt_insert_rel.execute(params![master_id, item_id])?;
            }
            Ok(())
        };

        insert_relations("artists", "item_artists", "artist_id", &info.artists)?;
        insert_relations("groups", "item_groups", "group_id", &info.groups)?;
        insert_relations("series", "item_series", "series_id", &info.series)?;
        insert_relations(
            "characters",
            "item_characters",
            "character_id",
            &info.characters,
        )?;
        insert_relations("tags", "item_tags", "tag_id", &info.tags)?;

        tx.commit()?;
        Ok(())
    }

    pub fn search_items(&self, query_str: &str) -> rusqlite::Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();

        let mut base_query = String::from("SELECT DISTINCT i.path FROM items i WHERE 1=1");
        let mut params: Vec<String> = Vec::new();

        let terms = query_str.split_whitespace();

        // ワイルドカード文字をエスケープするクロージャ
        let escape_like = |s: &str| -> String {
            s.replace('\\', r"\\")
                .replace('%', r"\%")
                .replace('_', r"\_")
        };

        for term in terms {
            // 除外検索プレフィックスの処理
            let is_exclude = term.starts_with('-');
            let actual_term = if is_exclude { &term[1..] } else { &term };

            if actual_term.is_empty() {
                continue;
            }

            if let Some((prefix, value)) = actual_term.split_once(':') {
                // プレフィックスの処理
                let (table_rel, table_master, fk_col) = match prefix.to_lowercase().as_str() {
                    "tag" => ("item_tags", "tags", "tag_id"),
                    "artist" => ("item_artists", "artists", "artist_id"),
                    "group" => ("item_groups", "groups", "group_id"),
                    "series" => ("item_series", "series", "series_id"),
                    "character" => ("item_characters", "characters", "character_id"),
                    "language" => {
                        if is_exclude {
                            base_query.push_str(
                                " AND (i.language_id IS NULL OR i.language_id NOT IN (SELECT id FROM languages WHERE name = ?))"
                            );
                        } else {
                            base_query.push_str(
                                " AND i.language_id IN (SELECT id FROM languages WHERE name = ?)",
                            );
                        }
                        params.push(value.to_string());
                        continue;
                    }
                    "type" => {
                        if is_exclude {
                            base_query.push_str(
                                " AND (i.type_id IS NULL OR i.type_id NOT IN (SELECT id FROM types WHERE name = ?))"
                            );
                        } else {
                            base_query.push_str(
                                " AND i.type_id IN (SELECT id FROM types WHERE name = ?)",
                            );
                        }
                        params.push(value.to_string());
                        continue;
                    }
                    _ => {
                        // 未知のプレフィックスは通常のタイトル検索としてフォールバック
                        let condition = if is_exclude { "NOT LIKE" } else { "LIKE" };
                        base_query.push_str(&format!(" AND i.title {} ? ESCAPE '\\'", condition));
                        params.push(format!("%{}%", escape_like(actual_term)));
                        continue;
                    }
                };

                // 中間テーブルを経由する検索条件の構築
                let exists_clause = if is_exclude { "NOT EXISTS" } else { "EXISTS" };
                base_query.push_str(&format!(
                    " AND {} (SELECT 1 FROM {} rel JOIN {} m ON rel.{} = m.id WHERE rel.item_id = i.id AND m.name = ?)",
                    exists_clause, table_rel, table_master, fk_col
                ));
                params.push(value.to_string());
            } else {
                // プレフィックスなしの場合はタイトル検索
                let condition = if is_exclude { "NOT LIKE" } else { "LIKE" };
                base_query.push_str(&format!(" AND i.title {} ? ESCAPE '\\'", condition));
                params.push(format!("%{}%", escape_like(actual_term)));
            }
        }

        base_query.push_str(" ORDER BY i.id DESC");

        let mut stmt = conn.prepare(&base_query)?;

        // params_from_iterを使って動的パラメータをバインド
        let paths = stmt
            .query_map(rusqlite::params_from_iter(params), |row| row.get(0))?
            .filter_map(Result::ok)
            .collect::<Vec<String>>();

        Ok(paths)
    }
}
