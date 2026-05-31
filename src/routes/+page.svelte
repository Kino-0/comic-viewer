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
    <!-- SearchScreen は破棄せず display:none で隠す。
         破棄すると結果リストのスクロール位置が失われるため。 -->
    <div class="screen" class:hidden={viewer.isViewing}>
        <SearchScreen />
    </div>
    {#if viewer.isViewing}
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
    .screen {
        width: 100%;
        height: 100%;
    }
    .screen.hidden {
        display: none;
    }
</style>
