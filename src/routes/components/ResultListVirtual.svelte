<script lang="ts">
    import ResultItem from "./ResultItem.svelte";
    import type { ItemSummary } from "../api";

    let { items }: { items: ItemSummary[] } = $props();
</script>

<div class="viewport">
    {#each items as item (item.id)}
        <div class="row">
            <ResultItem {item} mode="list" />
        </div>
    {/each}
</div>

<style>
    .viewport {
        margin-top: 0.75rem;
        flex: 1;
        min-height: 0;
        overflow-y: auto;
        padding-right: 0.25rem;
    }
    .row {
        padding-bottom: 8px;
        content-visibility: auto;
        contain-intrinsic-size: auto 188px;
    }
</style>
