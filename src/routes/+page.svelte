<script lang="ts">
    import { invoke, convertFileSrc } from "@tauri-apps/api/core";
    import { open } from "@tauri-apps/plugin-dialog";
    import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
    import { emit, listen } from "@tauri-apps/api/event";
    import { onMount } from "svelte";

    // 検索用の状態
    let query = $state("");
    let searchResults = $state<string[]>([]);

    // ビューアの状態
    let imagePaths = $state<string[]>([]);
    let currentIndex = $state(0);
    const currentImage = $derived(
        imagePaths.length > 0 ? convertFileSrc(imagePaths[currentIndex]) : null,
    );

    let isImporting = $state(false); // ローディング表示用の状態変数

    // --- 同期（ミラーリング）システムの構築 ---
    onMount(() => {
        // 1. 新しい画像群が読み込まれたとき、または閉じられたときの同期
        const unlistenState = listen<{
            imagePaths: string[];
            currentIndex: number;
        }>("sync-state", (event) => {
            imagePaths = event.payload.imagePaths;
            currentIndex = event.payload.currentIndex;
        });

        // 2. ページがめくられたときのインデックス同期
        const unlistenIndex = listen<{ currentIndex: number }>(
            "sync-index",
            (event) => {
                currentIndex = event.payload.currentIndex;
            },
        );

        // 3. 新規ウィンドウが立ち上がった際に、現在の状態を要求されるので応える
        const unlistenReq = listen("request-sync", () => {
            if (imagePaths.length > 0) {
                emit("sync-state", { imagePaths, currentIndex });
            }
        });

        // 自身が新しく開かれたウィンドウの場合、既存ウィンドウに状態を要求する
        emit("request-sync");

        return () => {
            unlistenState.then((f) => f());
            unlistenIndex.then((f) => f());
            unlistenReq.then((f) => f());
        };
    });

    // ミラーウィンドウを新規作成する関数
    async function openMirrorWindow() {
        const label = `mirror-${Date.now()}`;
        const mirror = new WebviewWindow(label, {
            url: "/",
            title: "Comic Viewer (Mirror)",
            width: 800,
            height: 600,
        });

        mirror.once("tauri://error", (e) => {
            console.error("ウィンドウの作成に失敗しました:", e);
        });
    }
    // ----------------------------------------

    async function importComicInfo() {
        try {
            // ユーザーにディレクトリを選択させる
            const selectedPath = await open({
                directory: true,
                multiple: false,
                title: "インポートするディレクトリを選択してください",
            });

            if (!selectedPath) {
                return; // キャンセルされた場合
            }

            isImporting = true;

            const importedCount = await invoke<number>("scan_and_import", {
                path: selectedPath,
            });

            alert(
                `インポート完了: ${importedCount} 件のコミックをデータベースに登録しました。`,
            );
        } catch (error) {
            console.error("Import failed:", error);
            alert(`インポート中にエラーが発生しました: ${error}`);
        } finally {
            isImporting = false;
        }
    }

    async function searchDb() {
        // 空文字の場合は検索せずに結果をクリア
        if (!query || !query.trim()) {
            searchResults = [];
            return;
        }

        try {
            searchResults = await invoke("search_items", { query });
        } catch (error) {
            console.error("Search failed:", error);
        }
    }

    async function loadDirectoryImages(dirPath: string) {
        try {
            const files = await invoke<string[]>("get_images_in_dir", {
                path: dirPath,
            });

            if (files.length > 0) {
                const sortedPaths = files.sort();
                imagePaths = sortedPaths;
                currentIndex = 0;

                // 開いた画像を他のウィンドウにも同期する
                emit("sync-state", {
                    imagePaths: sortedPaths,
                    currentIndex: 0,
                });
            } else {
                console.warn("このディレクトリには画像がありません");
            }
        } catch (error) {
            console.error("画像の読み込みに失敗しました:", error);
        }
    }

    // ビューア起動中のキー操作
    function handleKeydown(event: KeyboardEvent) {
        if (imagePaths.length === 0) return;

        let newIndex = currentIndex;

        switch (event.key) {
            case "ArrowLeft":
                newIndex = Math.min(currentIndex + 1, imagePaths.length - 1);
                break;
            case "ArrowRight":
                newIndex = Math.max(currentIndex - 1, 0);
                break;
            case "Escape":
                imagePaths = [];
                currentIndex = 0;
                // 閉じた状態を他のウィンドウにも同期する
                emit("sync-state", { imagePaths: [], currentIndex: 0 });
                return;
        }

        // ページがめくられた場合のみ、他のウィンドウへ同期イベントを送信
        if (newIndex !== currentIndex) {
            currentIndex = newIndex;
            emit("sync-index", { currentIndex });
        }
    }
    // ビューア起動中のマウス操作
    function handleMouseClick(event: MouseEvent) {
        if (imagePaths.length === 0) return;

        let newIndex = currentIndex;

        switch (event.button) {
            case 0:
                // 左クリック (button: 0): 次のページへ
                newIndex = Math.min(currentIndex + 1, imagePaths.length - 1);
                break;
            case 2:
                // 右クリック (button: 2): 前のページへ
                newIndex = Math.max(currentIndex - 1, 0);
                break;
        }

        // ページがめくられた場合のみ、他のウィンドウへ同期イベントを送信
        if (newIndex !== currentIndex) {
            currentIndex = newIndex;
            emit("sync-index", { currentIndex });
        }
    }
</script>

<svelte:window onkeydown={handleKeydown} />

<main class="container">
    {#if imagePaths.length === 0}
        <div class="search-screen">
            <h1>Comic Search</h1>
            <div class="search-box">
                <input
                    type="text"
                    bind:value={query}
                    placeholder="検索キーワードを入力..."
                    onkeydown={(e) => {
                        e.key === "Enter" && searchDb();
                    }}
                />
                <button onclick={searchDb}>検索</button>
                <button
                    class="import-btn"
                    onclick={importComicInfo}
                    disabled={isImporting}
                >
                    {isImporting ? "インポート中..." : "DB更新 (Import)"}
                </button>
                <!-- ミラーウィンドウを開くボタン -->
                <button class="mirror-btn" onclick={openMirrorWindow}>
                    ミラーを開く
                </button>
            </div>

            <ul class="results">
                {#each searchResults as path (path)}
                    <li>
                        <button
                            type="button"
                            onclick={() => loadDirectoryImages(path)}
                            >{path}</button
                        >
                    </li>
                {/each}
            </ul>
        </div>
    {:else}
        <div
            class="viewer"
            role="button"
            tabindex="0"
            onmousedown={handleMouseClick}
            oncontextmenu={(e) => e.preventDefault()}
        >
            <img src={currentImage} alt="Comic page" />
            <div class="info">Press Esc to return to search</div>
        </div>
    {/if}
</main>

<style>
    :global(body) {
        margin: 0;
    }
    .search-screen {
        padding: 2rem;
        max-width: 800px;
        margin: 0 auto;
    }
    .search-box {
        display: flex;
        gap: 0.5rem;
        margin-bottom: 1rem;
    }
    .search-box input {
        flex-grow: 1;
        padding: 1rem;
        font-size: 1.2rem;
        background: #2f2f2f;
        color: white;
        border: 1px solid #444;
        border-radius: 8px;
    }
    .search-box button {
        padding: 1rem 2rem;
        font-size: 1.2rem;
        background: #396cd8;
        color: white;
        border: none;
        border-radius: 8px;
        cursor: pointer;
        transition: background 0.2s;
    }
    .search-box button:hover:not(:disabled) {
        background: #2a51a9;
    }
    .search-box button.import-btn {
        background: #2f855a;
    }
    .search-box button.import-btn:hover:not(:disabled) {
        background: #22543d;
    }
    .search-box button.mirror-btn {
        background: #805ad5;
    }
    .search-box button.mirror-btn:hover:not(:disabled) {
        background: #6b46c1;
    }
    .search-box button:disabled {
        background: #555;
        cursor: not-allowed;
    }

    .results {
        list-style: none;
        padding: 0;
        margin-top: 1rem;
        max-height: 70vh;
        overflow-y: auto;
        text-align: left;
    }
    .results button {
        width: 100%;
        text-align: left;
        padding: 0.5rem;
        background: none;
        border: none;
        border-bottom: 1px solid #333;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        cursor: pointer;
    }
    .results button:hover {
        background: #396cd8;
        color: white;
    }
    .viewer {
        width: 100vw;
        height: 100vh;
        display: flex;
        justify-content: center;
        align-items: center;
        background: black;
    }
    img {
        height: 100%;
        width: 100%;
        object-fit: contain;
    }
    .info {
        position: absolute;
        top: 10px;
        color: oklch(50% 0 0 / 30%);
        font-size: 0.8rem;
    }
</style>
