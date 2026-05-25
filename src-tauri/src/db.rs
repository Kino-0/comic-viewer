//! データベース操作を管理するモジュール。
//!
//! `SQLite`を使用してコミックのメタデータ情報を保存および検索するための機能を提供します。

use rusqlite::{Connection, Result, params};
use std::{collections::HashMap, fmt::Write, path::Path, sync::Mutex};

/// コミックのメタ情報を保持する構造体。
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

/// サジェストデータの構造を表す型エイリアス
pub type SuggestionMap = HashMap<String, Vec<(String, String, usize)>>;

/// クォートを考慮して検索クエリ文字列をトークンに分割します。
///
/// `"` で囲まれた範囲内の空白は区切りとして扱わず、クォート文字自体はトークンから除去します。
/// 空のトークンは生成しません。
fn tokenize_query(query_str: &str) -> Vec<String> {
    let mut terms = Vec::new();
    let mut current_term = String::new();
    let mut in_quotes = false;

    for c in query_str.chars() {
        match c {
            '"' => in_quotes = !in_quotes,
            _ if c.is_whitespace() && !in_quotes => {
                if !current_term.is_empty() {
                    terms.push(std::mem::take(&mut current_term));
                }
            }
            _ => current_term.push(c),
        }
    }
    if !current_term.is_empty() {
        terms.push(current_term);
    }

    terms
}

/// `SQLite` データベースの接続を管理する構造体。
///
/// スレッドセーフにアクセスできるように、`Mutex`で接続をラップしています。
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// 新しいデータベース接続を作成、または既存のデータベースを開きます。
    ///
    /// 外部キー制約を有効にし、スキーマの初期化を行います。
    ///
    /// # Arguments
    ///
    /// * `path` - データベースファイルのパス。
    ///
    /// # Errors
    ///
    /// データベースのオープンや初期化処理に失敗した場合にエラーを返します。
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute("PRAGMA foreign_keys = ON;", [])?;
        conn.execute_batch(include_str!("./schema.sql"))?;
        // 既存DBへのマイグレーション: page_count 列が無ければ追加（列が既存の場合はエラーを無視）
        let _ = conn.execute(
            "ALTER TABLE items ADD COLUMN page_count INTEGER NOT NULL DEFAULT 0",
            [],
        );
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// テーブルから `name` に対応するIDを取得し、レコードが存在しなければ新規作成してIDを返します。
    ///
    /// # Arguments
    ///
    /// * `tx` - 実行中のトランザクション。
    /// * `table` - 対象となるマスターテーブルの名前。
    /// * `name` - 検索または挿入する名前。
    ///
    /// # Errors
    ///
    /// SQLクエリの実行やデータの挿入に失敗した場合にエラーを返します。
    fn get_or_create_master_id(
        tx: &rusqlite::Transaction,
        table: &str,
        name: &str,
    ) -> rusqlite::Result<i64> {
        let select_sql = format!("SELECT id FROM {table} WHERE name = ?1");
        let insert_sql = format!("INSERT INTO {table} (name) VALUES (?1)");

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
    }

    /// 中間テーブルへリレーションデータを挿入するヘルパー関数。
    ///
    /// # Arguments
    ///
    /// * `tx` - 実行中のトランザクション。
    /// * `table_master` - 参照先のマスターテーブルの名前。
    /// * `table_rel` - データを挿入する中間テーブルの名前。
    /// * `master_col` - 中間テーブルにおけるマスターIDの列名。
    /// * `item_id` - 紐づけるアイテムのID。
    /// * `names` - 登録する名前のリスト。
    ///
    /// # Errors
    ///
    /// SQLの実行に失敗した場合にエラーを返します。
    fn insert_relations(
        tx: &rusqlite::Transaction,
        table_master: &str,
        table_rel: &str,
        master_col: &str,
        item_id: i64,
        names: &[String],
    ) -> Result<()> {
        if names.is_empty() {
            return Ok(());
        }

        let insert_rel_sql =
            format!("INSERT OR IGNORE INTO {table_rel} ({master_col}, item_id) VALUES (?1, ?2)");
        let mut stmt_insert_rel = tx.prepare_cached(&insert_rel_sql)?;

        for name in names {
            if name.is_empty() {
                continue;
            }

            let master_id = Self::get_or_create_master_id(tx, table_master, name)?;

            // 中間テーブルへの挿入
            stmt_insert_rel.execute(params![master_id, item_id])?;
        }
        Ok(())
    }

    /// ギャラリー情報をデータベースに登録します。
    ///
    /// メインテーブルへの挿入後、各属性（作者、タグなど）の情報を中間テーブルに紐づけます。
    /// `info.gallery_id` が存在しない場合は、本家のIDとの衝突を避けるために
    /// データベース内で一意の**負のID**を自動採番して登録します。
    ///
    /// # Arguments
    ///
    /// * `info` - 挿入するギャラリーのメタデータ情報。
    /// * `dir_path` - ギャラリーの画像が保存されているディレクトリパス。
    /// * `page_count` - ギャラリーディレクトリ内の画像枚数。
    ///
    /// # Returns
    ///
    /// 登録された、または既存のアイテムID (i64) を返します。
    ///
    /// # Errors
    ///
    /// トランザクションの実行やコミットに失敗した場合にエラーを返します。
    pub fn insert_info(&self, info: &Info, dir_path: &str, page_count: usize) -> Result<i64> {
        let mut conn = self.conn.lock().expect("Database lock poisoned");
        let tx = conn.transaction()?;

        // 1. types テーブル
        let type_id: i64 = if info.type_name.is_empty() {
            Self::get_or_create_master_id(&tx, "types", "unknown")?
        } else {
            Self::get_or_create_master_id(&tx, "types", &info.type_name)?
        };

        // 2. languages テーブル
        let language_id: Option<i64> = if info.language.is_empty() {
            None
        } else {
            Some(Self::get_or_create_master_id(
                &tx,
                "languages",
                &info.language,
            )?)
        };

        // 3. items テーブルへの挿入
        let item_id = if let Some(id) = info.gallery_id {
            tx.execute(
                "INSERT OR IGNORE INTO items (id, title, type_id, language_id, path, page_count) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![id, info.title, type_id, language_id, dir_path, page_count as i64],
            )?;
            id
        } else {
            // 本家のギャラリーIDとの衝突を避けるため、負の数で採番する。
            let min_negative_new_id: i64 = tx
                .query_row("SELECT MIN(id) FROM items WHERE id < 0", [], |row| {
                    row.get::<_, Option<i64>>(0)
                })?
                .unwrap_or(0)
                - 1;

            tx.execute(
                "INSERT INTO items (id, title, type_id, language_id, path, page_count) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![min_negative_new_id, info.title, type_id, language_id, dir_path, page_count as i64],
            )?;
            min_negative_new_id
        };

        Self::insert_relations(
            &tx,
            "artists",
            "item_artists",
            "artist_id",
            item_id,
            &info.artists,
        )?;
        Self::insert_relations(
            &tx,
            "groups",
            "item_groups",
            "group_id",
            item_id,
            &info.groups,
        )?;
        Self::insert_relations(
            &tx,
            "series",
            "item_series",
            "series_id",
            item_id,
            &info.series,
        )?;
        Self::insert_relations(
            &tx,
            "characters",
            "item_characters",
            "character_id",
            item_id,
            &info.characters,
        )?;
        Self::insert_relations(&tx, "tags", "item_tags", "tag_id", item_id, &info.tags)?;

        tx.commit()?;
        Ok(item_id)
    }

    /// 全マスタデータの件数を一括取得し、サジェスト用のデータを構築します。
    ///
    /// タグ、作者、グループ、シリーズ、キャラクター、種類、言語の各カテゴリについて、
    /// 名前、小文字化された名前、およびその属性を持つアイテムの件数を集計します。
    /// 結果はカテゴリごとに分類され、件数の降順（同じ件数の場合は名前の昇順）でソートされます。
    ///
    /// # Returns
    ///
    /// カテゴリ名をキーとし、`(表示名, 検索用小文字名, 使用件数)` のタプルのリストを値とする `HashMap` を返します。
    ///
    /// # Errors
    ///
    /// SQLクエリの準備や実行に失敗した場合にエラーを返します。
    pub fn get_aggregated_suggestions(&self) -> rusqlite::Result<SuggestionMap> {
        let conn = self.conn.lock().expect("DB Lock Error");
        let mut result_map = HashMap::new();

        // プレフィックスと集計SQLの定義
        let queries = vec![
            (
                "tag",
                "SELECT m.name, COUNT(r.item_id) as c FROM tags m JOIN item_tags r ON m.id = r.tag_id GROUP BY m.id",
            ),
            (
                "artist",
                "SELECT m.name, COUNT(r.item_id) as c FROM artists m JOIN item_artists r ON m.id = r.artist_id GROUP BY m.id",
            ),
            (
                "group",
                "SELECT m.name, COUNT(r.item_id) as c FROM groups m JOIN item_groups r ON m.id = r.group_id GROUP BY m.id",
            ),
            (
                "series",
                "SELECT m.name, COUNT(r.item_id) as c FROM series m JOIN item_series r ON m.id = r.series_id GROUP BY m.id",
            ),
            (
                "character",
                "SELECT m.name, COUNT(r.item_id) as c FROM characters m JOIN item_characters r ON m.id = r.character_id GROUP BY m.id",
            ),
            (
                "type",
                "SELECT m.name, COUNT(i.id) as c FROM types m JOIN items i ON m.id = i.type_id GROUP BY m.id",
            ),
            (
                "language",
                "SELECT m.name, COUNT(i.id) as c FROM languages m JOIN items i ON m.id = i.language_id GROUP BY m.id",
            ),
        ];

        for (prefix, sql) in queries {
            let mut stmt = conn.prepare(sql)?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    usize::try_from(row.get::<_, i64>(1)?).unwrap_or(0),
                ))
            })?;
            let mut list: Vec<(String, String, usize)> = rows
                .flatten()
                .map(|(name, count)| {
                    let lower_name = name.to_lowercase();
                    (name, lower_name, count)
                })
                .collect();
            list.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
            result_map.insert(prefix.to_string(), list);
        }

        Ok(result_map)
    }

    /// 検索クエリに基づいてアイテムのパス一覧を取得します。
    ///
    /// プレフィックス（例: `tag:`, `artist:`）を用いた条件指定や、
    /// ハイフンマイナス(`-`)による除外検索、
    /// タイトルの部分一致検索に対応しています。
    /// ワイルドカード（`%` や `_`）はエスケープされ、リテラルとして検索されます。
    ///
    /// # Arguments
    ///
    /// * `query_str` - 空白区切りの検索クエリ文字列。
    ///
    /// # Errors
    ///
    /// クエリのパースやSQLの実行に失敗した場合にエラーを返します。
    pub fn search_items(&self, query_str: &str) -> rusqlite::Result<Vec<String>> {
        let conn = self.conn.lock().expect("Database lock poisoned");

        let mut base_query = String::from("SELECT DISTINCT i.path FROM items i WHERE 1=1");
        let mut params: Vec<String> = Vec::new();

        let terms = tokenize_query(query_str);

        // ワイルドカード文字をエスケープするクロージャ
        let escape_like = |s: &str| -> String {
            s.replace('\\', r"\\")
                .replace('%', r"\%")
                .replace('_', r"\_")
        };

        for term in &terms {
            // 除外検索プレフィックスの処理
            let is_exclude = term.starts_with('-');
            let actual_term = if is_exclude {
                &term[1..]
            } else {
                term.as_str()
            };

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
                        let _ = write!(base_query, " AND i.title {condition} ? ESCAPE '\\'");
                        params.push(format!("%{}%", escape_like(actual_term)));
                        continue;
                    }
                };

                // 中間テーブルを経由する検索条件の構築
                let exists_clause = if is_exclude { "NOT EXISTS" } else { "EXISTS" };
                let _ = write!(
                    base_query,
                    " AND {exists_clause} (SELECT 1 FROM {table_rel} rel JOIN {table_master} m ON rel.{fk_col} = m.id WHERE rel.item_id = i.id AND m.name = ?)"
                );
                params.push(value.to_string());
            } else {
                // プレフィックスなしの場合はタイトル検索
                let condition = if is_exclude { "NOT LIKE" } else { "LIKE" };
                let _ = write!(base_query, " AND i.title {condition} ? ESCAPE '\\'");
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
