<script>
  import { getCurrentWindow } from '@tauri-apps/api/window';
  
  let isMuted = $state(false);
  let isDeaf = $state(false);

  function toggleMute() {
    isMuted = !isMuted;
  }
  
  function toggleDeaf() {
    isDeaf = !isDeaf;
  }

  async function expand() {
    // Logic to show main window and hide this one would go here
    // or just maximize this one if it's the same window (but we're using multi-window)
    const win = getCurrentWindow();
    // await win.hide(); 
    // This part requires communication with main window to show it.
  }
</script>

<div data-tauri-drag-region class="w-full h-full bg-black/90 backdrop-blur border border-white/20 flex flex-col items-center justify-center p-2 rounded-lg overflow-hidden">
  <div class="text-[10px] font-bold text-white/50 mb-2 tracking-widest pointer-events-none">VOID MINI</div>
  
  <div class="flex gap-4">
    <button 
      class="w-10 h-10 rounded-full border border-white/20 flex items-center justify-center hover:bg-white/10 transition-colors {isMuted ? 'text-red-500 border-red-500/50' : 'text-white'}"
      onclick={toggleMute}
    >
      {#if isMuted}
        <!-- Mute Icon -->
        <div class="w-3 h-3 bg-current"></div>
      {:else}
        <!-- Mic Icon -->
        <div class="w-3 h-3 border border-current rounded-sm"></div>
      {/if}
    </button>
    
    <button 
      class="w-10 h-10 rounded-full border border-white/20 flex items-center justify-center hover:bg-white/10 transition-colors {isDeaf ? 'text-red-500 border-red-500/50' : 'text-white'}"
      onclick={toggleDeaf}
    >
      {#if isDeaf}
        <!-- Deaf Icon -->
        <div class="w-3 h-3 bg-current rounded-full"></div>
      {:else}
        <!-- Headphone Icon -->
        <div class="w-3 h-3 border-2 border-current rounded-t-full border-b-0"></div>
      {/if}
    </button>
  </div>
</div>

<style>
  :global(body) {
    background: transparent !important;
  }
</style>
