<script lang="ts">
  import StickyBoard from '$lib/components/tools/StickyBoard.svelte';
  import { onMount } from 'svelte';

  let targetPeerId = $state('');

  onMount(() => {
      const params = new URLSearchParams(window.location.search);
      targetPeerId = params.get('peerId') || '';
  });
</script>

<div class="w-screen h-screen overflow-hidden bg-transparent">
  <StickyBoard {targetPeerId} isDetached={true} />
</div>

<style>
  :global(body) {
      background: transparent !important;
  }
</style>
