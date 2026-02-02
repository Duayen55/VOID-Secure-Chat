<script>
  import '../app.css';
  import UpdateSplash from '$lib/components/UpdateSplash.svelte';
  
  let { children } = $props();
  let checkingUpdates = $state(true);

  function handleUpdateComplete() {
    checkingUpdates = false;
  }
</script>

{#if checkingUpdates}
  <UpdateSplash onComplete={handleUpdateComplete} />
{:else}
  {@render children()}
{/if}
