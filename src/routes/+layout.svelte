<script>
  import '../app.css';
  import UpdateSplash from '$lib/components/UpdateSplash.svelte';
  import GlobalCallListener from '$lib/components/GlobalCallListener.svelte';
  
  let { children } = $props();
  let checkingUpdates = $state(true);

  function handleUpdateComplete() {
    checkingUpdates = false;
  }
</script>

<GlobalCallListener />

{#if checkingUpdates}
  <UpdateSplash onComplete={handleUpdateComplete} />
{:else}
  {@render children()}
{/if}
