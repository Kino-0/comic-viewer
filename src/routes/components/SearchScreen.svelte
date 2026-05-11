<script lang="ts">
    import { onDestroy } from "svelte";
    import { TauriAPI } from "../api";
    import { library } from "../library.svelte";
    import { viewer } from "../viewer.svelte";

    let inputRef: HTMLInputElement;
    let debounceTimer: ReturnType<typeof setTimeout>;

    // コンポーネント破棄時にタイマーをクリアし、メモリリークや予期せぬ状態更新を防止
    onDestroy(() => {
        clearTimeout(debounceTimer);
    });

    function handleSearch(e: SubmitEvent) {
        e.preventDefault();
        library.clearSuggestions();
        library.search();
    }

    function handleInput() {
        clearTimeout(debounceTimer);

        const cursor = inputRef.selectionStart || 0;
        const textBefore = library.query.slice(0, cursor);
        const textAfter = library.query.slice(cursor);

        // カーソル前後の文字列から、現在の単語を抽出
        const prefixMatch = textBefore.match(/\S+$/) || [""];
        const suffixMatch = textAfter.match(/^\S+/) || [""];
        const currentWord = prefixMatch[0] + suffixMatch[0];
        const cleanWord = currentWord.replace(/"/g, "");
        const match = cleanWord.replace(/^-/, "").match(/^([^:]+):(.*)$/);

        if (match) {
            const [, prefix, keyword] = match;
            debounceTimer = setTimeout(() => {
                library.fetchSuggestions(prefix, keyword);
            }, 100);
        } else {
            library.clearSuggestions();
        }
    }

    async function insertSuggestion(suggestion: string) {
        const query = library.query;
        const cursor = inputRef.selectionStart || 0;

        // カーソルの直前から前に向かって一番近い空白を探す（単語の始点）
        const start = query.lastIndexOf(" ", cursor - 1) + 1;
        // カーソル位置から後ろに向かって一番近い空白を探す（単語の終点）
        const end =
            query.indexOf(" ", cursor) === -1
                ? query.length
                : query.indexOf(" ", cursor);

        const currentWord = query.slice(start, end);
        const colonIndex = currentWord.indexOf(":");

        if (colonIndex !== -1) {
            const prefix = currentWord.slice(0, colonIndex + 1);
            const rawPrefix = prefix.replace(/"/g, ""); // 既にクォーテーションがある場合は除去

            // 空白を含むサジェストの場合はダブルクォーテーションで囲む
            const suggestionText = suggestion.includes(" ")
                ? `"${rawPrefix}${suggestion}"`
                : `${rawPrefix}${suggestion}`;

            library.query = `${query.slice(0, start)}${suggestionText} ${query.slice(end).trimStart()}`;
        }

        library.clearSuggestions();
        inputRef.focus();
    }

    function handleKeyDown(e: KeyboardEvent) {
        if (e.isComposing) return;

        const len = library.suggestions.length;
        if (!library.showSuggestions || len === 0) return;

        if (e.key === "ArrowDown") {
            e.preventDefault();
            library.activeSuggestionIndex =
                (library.activeSuggestionIndex + 1) % len;
        } else if (e.key === "ArrowUp") {
            e.preventDefault();
            library.activeSuggestionIndex =
                (library.activeSuggestionIndex - 1 + len) % len;
        } else if (e.key === "Enter" || e.key === "Tab") {
            e.preventDefault();
            insertSuggestion(
                library.suggestions[library.activeSuggestionIndex],
            );
        } else if (e.key === "Escape") {
            library.clearSuggestions();
        }
    }

    // 入力欄からフォーカスが外れたらサジェスチョンを閉じる
    function handleBlur() {
        library.clearSuggestions();
    }
</script>

<div class="search-screen">
    <h1>Comic Search</h1>
    <form class="search-box" onsubmit={handleSearch}>
        <div class="input-container">
            <input
                bind:this={inputRef}
                type="text"
                bind:value={library.query}
                oninput={handleInput}
                onkeydown={handleKeyDown}
                onblur={handleBlur}
                autocomplete="off"
                placeholder="タグや作者で検索..."
            />
            {#if library.showSuggestions}
                <ul class="suggestions">
                    {#each library.suggestions as suggestion, i}
                        <!-- svelte-ignore a11y_click_events_have_key_events -->
                        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
                        <li
                            class={i === library.activeSuggestionIndex
                                ? "active"
                                : ""}
                            onmousedown={(e) => {
                                e.preventDefault();
                                insertSuggestion(suggestion);
                            }}
                        >
                            {suggestion}
                        </li>
                    {/each}
                </ul>
            {/if}
        </div>

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
    .input-container {
        position: relative;
        flex-grow: 1;
        display: flex;
    }
    .input-container input {
        flex-grow: 1;
        padding: 1rem;
        font-size: 1.2rem;
        background: #2f2f2f;
        color: white;
        border: 1px solid #444;
        border-radius: 8px;
    }
    .suggestions {
        position: absolute;
        top: 100%;
        left: 0;
        right: 0;
        background: #2f2f2f;
        border: 1px solid #444;
        border-top: none;
        border-radius: 0 0 8px 8px;
        margin: 0;
        padding: 0;
        list-style: none;
        z-index: 10;
        max-height: 200px;
        overflow-y: auto;
        box-shadow: 0 4px 6px rgba(0, 0, 0, 0.3);
    }
    .suggestions li {
        padding: 0.8rem 1rem;
        cursor: pointer;
        color: white;
    }
    .suggestions li:hover,
    .suggestions li.active {
        background: #396cd8;
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
