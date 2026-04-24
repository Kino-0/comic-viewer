<script lang="ts">
    import { invoke, convertFileSrc } from "@tauri-apps/api/core";

    // 検索用の状態
    let query = $state("");
    let searchResults = $state<string[]>([]);

    // ビューアの状態
    let imagePaths = $state<string[]>([]);
    let currentIndex = $state(0);
    const currentImage = $derived(
        imagePaths.length > 0 ? convertFileSrc(imagePaths[currentIndex]) : null,
    );

    // plocate で検索
    async function searchWithPlocate() {
        // 空文字の場合は検索せずに結果をクリア
        if (!query || !query.trim()) {
            searchResults = [];
            return;
        }

        try {
            searchResults = await invoke("search_with_plocate", {
                query,
                dbPath: "/var/lib/plocate/HDD2.db",
            });
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
                imagePaths = files.sort();
                currentIndex = 0;
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
        switch (event.key) {
            case "ArrowRight":
                currentIndex = Math.min(
                    currentIndex + 1,
                    imagePaths.length - 1,
                );
                break;
            case "ArrowLeft":
                currentIndex = Math.max(currentIndex - 1, 0);
                break;
            case "Escape":
                imagePaths = [];
                break;
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
                    placeholder="ディレクトリ検索..."
                    onkeydown={(e) => {
                        e.key === "Enter" && searchWithPlocate();
                    }}
                />
                <button onclick={searchWithPlocate}>🔍</button>
            </div>

            <ul class="results">
                {#each searchResults as path}
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
        <div class="viewer" role="button" tabindex="0">
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
    .search-box button:hover {
        background: #2a51a9;
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
        max-height: 100%;
        max-width: 100%;
    }
    .info {
        position: absolute;
        top: 10px;
        color: #555;
        font-size: 0.8rem;
    }
</style>
