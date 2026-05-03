<script lang="ts">
    import { TauriAPI } from "../api";
    import { library } from "../library.svelte";
    import { viewer } from "../viewer.svelte";

    function handleSearch(e: SubmitEvent) {
        e.preventDefault();
        library.search();
    }
</script>

<div class="search-screen">
    <h1>Comic Search</h1>

    <form class="search-box" onsubmit={handleSearch}>
        <input
            type="text"
            bind:value={library.query}
            placeholder="検索キーワードを入力..."
        />
        <button type="submit">検索</button>
        <button
            type="button"
            class="import-btn"
            onclick={() => library.importDirectory()}
            disabled={library.isImporting}
        >
            {library.isImporting ? "インポート中..." : "DB更新 (Import)"}
        </button>
        <button
            type="button"
            class="mirror-btn"
            onclick={TauriAPI.openMirrorWindow}
        >
            ミラーを開く
        </button>
    </form>

    <ul class="results">
        {#each library.searchResults as path (path)}
            <li>
                <button type="button" onclick={() => viewer.load(path)}>
                    {path}
                </button>
            </li>
        {/each}
    </ul>
</div>

<style>
    .search-screen {
        padding: 2rem;
        max-width: 800px;
        margin: 0 auto;
        color: white;
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
    }
    .results button {
        width: 100%;
        text-align: left;
        padding: 0.5rem;
        color: white;
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
    }
</style>
