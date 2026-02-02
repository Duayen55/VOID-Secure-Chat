<script lang="ts">
  import OverlayContent from '$lib/components/OverlayContent.svelte';
  import { onMount } from 'svelte';

  onMount(() => {
      document.body.style.backgroundColor = 'transparent';
      document.documentElement.style.backgroundColor = 'transparent';
  });
</script>

<OverlayContent />

<style>
  :global(html), :global(body) {
      background: transparent !important;
      margin: 0;
      padding: 0;
      overflow: hidden;
  }
</style>