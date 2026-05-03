<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { viewer } from "./viewer.svelte";
    import SearchScreen from "./components/SearchScreen.svelte";
    import ViewerScreen from "./components/ViewerScreen.svelte";

    onMount(() => {
        viewer.initListeners();
    });

    onDestroy(() => {
        viewer.destroyListeners();
    });
</script>

<main>
    {#if !viewer.isViewing}
        <SearchScreen />
    {:else}
        <ViewerScreen />
    {/if}
</main>

<style>
    :global(body) {
        margin: 0;
        background: black;
    }
    main {
        width: 100vw;
        height: 100vh;
        overflow: hidden;
    }
</style>
