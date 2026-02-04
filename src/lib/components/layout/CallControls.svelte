<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  export let isMuted = false;
  export let isDeafened = false;
  export let isScreenSharing = false;
  export let isMusicActive = false;

  const dispatch = createEventDispatcher();
</script>

<div class="h-24 bg-[#080A0E] border-t border-white/5 flex items-center justify-center space-x-6 z-30 shrink-0 relative shadow-[0_-10px_40px_rgba(0,0,0,0.5)]">
    
    <!-- Volumetric Glow Top -->
    <div class="absolute -top-10 left-0 w-full h-10 bg-gradient-to-t from-[#080A0E] to-transparent pointer-events-none"></div>

    <button 
        class="p-4 rounded-2xl transition-all duration-200 cursor-pointer {isMuted ? 'bg-red-500/20 text-red-500 ring-1 ring-red-500/50' : 'bg-surface hover:bg-surface-hover text-white ring-1 ring-white/10 hover:ring-white/30'}"
        on:click={() => dispatch('toggleMute')}
        title="Toggle Mic"
    >
        {#if isMuted}
            <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z" /><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 3l18 18" /></svg>
        {:else}
            <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z" /></svg>
        {/if}
    </button>

    <button 
        class="p-4 rounded-2xl transition-all duration-200 cursor-pointer {isDeafened ? 'bg-red-500/20 text-red-500 ring-1 ring-red-500/50' : 'bg-surface hover:bg-surface-hover text-white ring-1 ring-white/10 hover:ring-white/30'}"
        on:click={() => dispatch('toggleDeafen')}
        title="Toggle Deafen"
    >
        {#if isDeafened}
            <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5.586 15H4a1 1 0 01-1-1v-4a1 1 0 011-1h1.586l4.707-4.707C10.923 3.663 12 4.109 12 5v14c0 .891-1.077 1.337-1.707.707L5.586 15z" /><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2" /></svg>
        {:else}
            <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.536 8.464a5 5 0 010 7.072m2.828-9.9a9 9 0 010 12.728M5.586 15H4a1 1 0 01-1-1v-4a1 1 0 011-1h1.586l4.707-4.707C10.923 3.663 12 4.109 12 5v14c0 .891-1.077 1.337-1.707.707L5.586 15z" /></svg>
        {/if}
    </button>

    <button 
        class="p-4 rounded-2xl transition-all duration-200 cursor-pointer {isScreenSharing ? 'bg-accent/20 text-accent ring-1 ring-accent/50 shadow-[0_0_15px_rgba(0,229,255,0.3)]' : 'bg-surface hover:bg-surface-hover text-white ring-1 ring-white/10 hover:ring-white/30'}"
        on:click={() => dispatch('toggleScreenShare')}
        title="Screen Share"
    >
        <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" /></svg>
    </button>

    <button 
        class="p-4 rounded-2xl transition-all duration-200 cursor-pointer {isMusicActive ? 'bg-accent/20 text-accent ring-1 ring-accent/50 shadow-[0_0_15px_rgba(0,229,255,0.3)]' : 'bg-surface hover:bg-surface-hover text-white ring-1 ring-white/10 hover:ring-white/30'}"
        on:click={() => dispatch('toggleMusic')}
        title="Music Player"
    >
        <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3" /></svg>
    </button>

    <button 
        class="p-4 rounded-2xl bg-red-600 hover:bg-red-500 text-white shadow-lg shadow-red-900/20 hover:shadow-red-900/40 hover:scale-105 transition-all duration-200 ring-1 ring-red-400/50 cursor-pointer"
        on:click={() => dispatch('endCall')}
        title="End Call"
    >
        <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 8l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2M5 3a2 2 0 00-2 2v1c0 8.284 6.716 15 15 15h1a2 2 0 002-2v-3.28a1 1 0 00-.684-.948l-5-1.08a1 1 0 00-1.215.539l-1.356 3.817a15.015 15.015 0 01-6.943-6.943l3.816-1.356a1 1 0 00.539-1.215l-1.08-5.05A1 1 0 006.28 4H5z" /></svg>
    </button>

</div>
