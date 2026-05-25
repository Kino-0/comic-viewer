<script lang="ts">
    import { library, type SearchPrefix } from "../library.svelte";

    interface Props {
        prefix: SearchPrefix;
        value: string;
    }

    let { prefix, value }: Props = $props();

    // チップのクリックは検索欄追加のみ。行クリック（ビューア起動）には伝播させない。
    function handleClick(e: MouseEvent) {
        e.stopPropagation();
        library.appendToQuery(prefix, value);
    }
</script>

<button
    type="button"
    class="chip chip-{prefix}"
    title={`${prefix}:${value} を検索欄に追加`}
    onclick={handleClick}
>
    {value}
</button>

<style>
    .chip {
        display: inline-block;
        max-width: 100%;
        padding: 0.15rem 0.5rem;
        font-size: 0.8rem;
        line-height: 1.4;
        color: #ddd;
        background: #3a3a3a;
        border: 1px solid #4a4a4a;
        border-radius: 999px;
        cursor: pointer;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        transition:
            background 0.15s,
            border-color 0.15s;
    }
    .chip:hover {
        background: #396cd8;
        border-color: #396cd8;
        color: white;
    }
    /* 属性ごとに左ボーダーで色分けし、種別を識別しやすくする */
    .chip-artist {
        border-left: 3px solid #e08a3c;
    }
    .chip-group {
        border-left: 3px solid #d05cc0;
    }
    .chip-series {
        border-left: 3px solid #4caf50;
    }
    .chip-character {
        border-left: 3px solid #f2c14e;
    }
    .chip-tag {
        border-left: 3px solid #5b8def;
    }
    .chip-type {
        border-left: 3px solid #888;
    }
    .chip-language {
        border-left: 3px solid #00bcd4;
    }
</style>
