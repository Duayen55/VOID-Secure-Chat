<script>
  import '../app.css';
  import UpdateChecker from '$lib/components/UpdateChecker.svelte';
  let { children } = $props();
</script>

<div class="w-screen h-screen bg-black text-white font-mono overflow-hidden selection:bg-white selection:text-black">
  {@render children()}
  <UpdateChecker />
</div>

<style>
  @reference "tailwindcss";
  :global(body) {
    @apply bg-black text-white m-0 p-0 overflow-hidden;
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
  }
  
  /* Custom Scrollbar */
  :global(::-webkit-scrollbar) {
    width: 6px;
    height: 6px;
  }
  :global(::-webkit-scrollbar-track) {
    @apply bg-black;
  }
  :global(::-webkit-scrollbar-thumb) {
    @apply bg-white/20 hover:bg-white/40;
  }
</style>
