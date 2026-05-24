//! Tauriアプリケーションのバックエンドロジックとコマンドを提供するモジュール。

mod db;
use db::Info;
use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::RwLock,
};
use tauri::{Manager, State};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use walkdir::WalkDir;

/// サポートしている画像拡張子（大文字・小文字は区別しない）。
const SUPPORTED_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif"];

/// 指定したパスがサポート対象の画像拡張子を持つか判定します。
fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            SUPPORTED_EXTENSIONS
                .iter()
                .any(|&s| s.eq_ignore_ascii_case(ext))
        })
}

/// 指定ディレクトリ直下のサポート対象画像ファイルのパスリストを返します。
fn collect_image_paths(dir: &Path) -> io::Result<Vec<PathBuf>> {
    Ok(fs::read_dir(dir)?
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|ft| ft.is_file()))
        .map(|e| e.path())
        .filter(|p| is_supported_image(p))
        .collect())
}

/// 指定されたディレクトリ直下にある画像ファイルパスのリストを取得します。
///
/// * 非再帰的: サブディレクトリ内のファイルは検索されません。
/// * 拡張子: サポートしている拡張子は `png`, `jpg`, `jpeg`, `webp`, `gif` です。
/// * Case: 拡張子の判定において、大文字と小文字は区別されません（例: `.PNG` や `.Jpg` も対象となります）。
/// * エンコーディング: ファイルパスや拡張子が有効なUTF-8文字列に変換できないファイルは、結果からスキップされます。
/// * アクセス権限: 個別のファイルエントリの読み込みに失敗した場合は、そのファイルをスキップします。
///
/// # Arguments
///
/// * `path` - 画像を検索する対象のディレクトリパス。
///
/// # Returns
///
/// 成功した場合は、画像パスのリスト (`Vec<String>`) を返します。
///
/// # Errors
///
/// 以下の理由などにより、指定されたディレクトリ自体の読み込みに失敗した場合はエラーメッセージ（`String`）を返します。
/// * ディレクトリが存在しない
/// * ディレクトリに対する読み取り権限がない
#[tauri::command]
fn get_images_in_dir(path: std::path::PathBuf) -> Result<Vec<String>, String> {
    let image_paths = collect_image_paths(&path)
        .map_err(|e| format!("ディレクトリの読み込みに失敗しました: {e}"))?
        .into_iter()
        .filter_map(|p| p.to_str().map(str::to_owned))
        .collect();
    Ok(image_paths)
}

/// `info.txt` のテキスト内容を解析し、[`Info`] 構造体に変換します。
///
/// コロン `:` で区切られたキーと値のペアを読み取り、適切なフィールドに割り当てます。
///
/// # Arguments
///
/// * `content` - `info.txt` の文字列データ。
///
/// # Returns
///
/// パースされたギャラリー情報を保持する [`Info`] 構造体。
fn parse_info_txt(content: &str) -> Info {
    let mut info = Info::default();

    // カンマ区切りの文字列を分割するクロージャ
    let parse_csv = |val: &str| -> Vec<String> {
        val.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };

    for line in content.lines() {
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        let Some((key, value)) = line.split_once(':') else {
            continue;
        };

        let key = key.trim();
        let value = value.trim();

        if value == "N/A" {
            continue;
        }

        match key {
            "ギャラリーID" => info.gallery_id = value.parse::<i64>().ok(),
            "タイトル" => info.title = value.to_string(),
            "作者" => info.artists = parse_csv(value),
            "グループ" => info.groups = parse_csv(value),
            "種類" => info.type_name = value.to_string(),
            "シリーズ" => info.series = parse_csv(value),
            "キャラクター" => info.characters = parse_csv(value),
            "タグ" => info.tags = parse_csv(value),
            "言語" => info.language = value.to_string(),
            _ => {}
        }
    }

    info
}

/// 単一のギャラリー情報（`info.txt`）を読み込み、データベースにインポートします。
///
/// ファイルの内容を解析してデータベースに保存します。対象の `info.txt` に
/// ギャラリーIDが存在しない場合、データベースで採番された新しいIDを
/// 元のファイルに追記します。
///
/// # Arguments
///
/// * `path` - インポートする `info.txt` ファイルのパス。
/// * `db` - ギャラリー情報を保存するデータベースインスタンスへの参照。
///
/// # Returns
///
/// ファイルの読み書きやデータベースの挿入処理でエラーが発生した場合は、エラー文字列を返します。
fn import_single_gallery(path: &Path, db: &db::Database) -> Result<(), String> {
    let content = fs::read_to_string(path).map_err(|e| format!("ファイル読み込みエラー: {e}"))?;

    let info = parse_info_txt(&content);

    let dir_path = path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    // DBへの挿入
    let item_id = db
        .insert_info(&info, &dir_path)
        .map_err(|e| format!("DB挿入エラー: {e}"))?;

    // ギャラリーIDが新規採番された場合、info.txtに追記
    if info.gallery_id.is_none() {
        let mut file = OpenOptions::new()
            .append(true)
            .open(path)
            .map_err(|e| format!("info.txtオープンエラー: {e}"))?;

        writeln!(file, "ギャラリーID: {item_id}")
            .map_err(|e| format!("info.txt追記エラー: {e}"))?;
    }

    Ok(())
}

/// サジェスチョン内容の各エントリを表す型（名前、元の名前、出現回数）。
pub type SuggestionEntry = (String, String, usize);

/// サジェスチョン内容を保持するインメモリキャッシュ。
pub struct SuggestionCache(pub RwLock<HashMap<String, Vec<SuggestionEntry>>>);

/// キャッシュされたサジェストデータから、キーワードに一致するものを取得します。
///
/// # Arguments
///
/// * `prefix` - 検索対象のカテゴリ（`tag`, `artist` など）。
/// * `keyword` - 検索キーワード（部分一致）。
/// * `cache` - キャッシュされているサジェストデータ。
///
/// # Returns
///
/// 最大20件のサジェスト結果を返します。
#[tauri::command]
fn get_suggestions(
    prefix: String,
    keyword: String,
    cache: State<'_, SuggestionCache>,
) -> Result<Vec<String>, String> {
    let data_guard = cache.0.read().map_err(|_| "Cache Lock Error")?;

    let Some(list) = data_guard.get(&prefix.to_lowercase()) else {
        return Ok(vec![]);
    };

    let keyword_lower = keyword.to_lowercase();

    let results = list
        .iter()
        .filter(|(_, lower_name, _)| lower_name.contains(&keyword_lower))
        .take(20)
        .map(|(name, _, _)| name.clone())
        .collect();

    Ok(results)
}

/// 指定されたパスを再帰的にスキャンし、見つかった `info.txt` をデータベースにインポートします。
///
/// # Arguments
///
/// * `path` - スキャンを開始するディレクトリのパス。
/// * `db` - アプリケーションのステートとして管理されているデータベースインスタンス。
/// * `cache` - サジェストデータのキャッシュ。スキャン完了後に更新されます。
///
/// # Returns
///
/// 正常にインポートされたギャラリーの総数を返します。
#[tauri::command]
fn scan_and_import(
    path: String,
    db: State<'_, db::Database>,
    cache: State<'_, SuggestionCache>,
) -> Result<usize, String> {
    let mut imported_count = 0;

    for entry in WalkDir::new(&path)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        if entry.file_type().is_file() && entry.file_name() == "info.txt" {
            match import_single_gallery(entry.path(), &db) {
                Ok(()) => imported_count += 1,
                Err(e) => eprintln!("インポート失敗 ({}): {e}", entry.path().display()),
            }
        }
    }

    // インポート成功時のみキャッシュを更新
    if imported_count > 0
        && let Ok(new_data) = db.get_aggregated_suggestions()
    {
        let mut write_guard = cache.0.write().map_err(|_| "Cache Write Error")?;
        *write_guard = new_data;
    }

    Ok(imported_count)
}

/// クエリ文字列に基づいてデータベースを検索し、一致するアイテムのパスを取得します。
///
/// # Arguments
///
/// * `query` - 検索クエリ文字列。
/// * `db` - アプリケーションのステートとして管理されているデータベースインスタンス。
///
/// # Returns
///
/// 検索にヒットしたパスのリストを返し、データベースエラーが発生した場合はエラー文字列を返します。
#[tauri::command]
fn search_items(query: String, db: tauri::State<'_, db::Database>) -> Result<Vec<String>, String> {
    db.search_items(&query).map_err(|e| e.to_string())
}

/// Tauriアプリケーションを初期化し、実行します。
///
/// データベースの接続設定、各種プラグインの初期化、およびTauriコマンドの登録を行います。
/// データベースの初期化に失敗した場合、エラーダイアログを表示してアプリケーションを終了します。
///
/// # Panics
///
/// 以下の状況でパニックが発生する可能性があります：
/// * ローカルデータディレクトリの取得に失敗した場合。
/// * ローカルデータディレクトリの作成に失敗した場合。
/// * Tauriアプリケーションの実行中に致命的なエラーが発生した場合。
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // アプリケーション固有のデータディレクトリを取得する
            let mut db_path = app
                .path()
                .app_local_data_dir()
                .expect("Failed to get local data dir");

            // ディレクトリが存在しない場合は作成する
            fs::create_dir_all(&db_path).expect("Failed to create local data dir");

            // データベースのファイル名をパスに追加する
            db_path.push("comic_viewer.db");

            let db = match db::Database::new(&db_path) {
                Ok(db) => db,
                Err(e) => {
                    let err_msg = format!("データベースの初期化に失敗しました: {e}");
                    eprintln!("{err_msg}");
                    // エラーメッセージダイアログを表示する
                    let app_handle = app.handle().clone();
                    app.dialog()
                        .message(&err_msg)
                        .title("初期化エラー")
                        .kind(MessageDialogKind::Error)
                        .show(move |_| {
                            app_handle.exit(1);
                        });
                    return Ok(());
                }
            };

            let initial_data = db.get_aggregated_suggestions().unwrap_or_default();
            app.manage(db);
            app.manage(SuggestionCache(RwLock::new(initial_data)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_images_in_dir,
            scan_and_import,
            search_items,
            get_suggestions
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
