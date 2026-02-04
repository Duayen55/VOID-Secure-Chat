<script lang="ts">
  import MusicPlayer from '$lib/components/MusicPlayer.svelte';
  import { slide } from 'svelte/transition';
  import { settings, isSpeaking } from '$lib/stores';
  import { audioEngine } from '$lib/audio';
  
  export let activeChat: any = null;
  export let callState: 'idle' | 'ringing' | 'connected' = 'idle';
  export let participants: any[] = []; 
  export let friends: any[] = []; 
  export let activeProfile: string = 'Default'; // 'FPS', 'MMO', 'Casual'
  
  // Logic to resolve participant names
  $: resolveName = (uid: string) => {
      const f = friends.find(x => x.uid === uid);
      return f ? f.displayName : 'Unknown';
  }

  let showPresets = false;
  let fileInput: HTMLInputElement;

  const PRESETS = {
      'FPS Night': {
          noiseGateThreshold: -60, // Open easily
          micGain: 1.5, // Boost quiet sounds
          noiseSuppression: false // Hear raw details
      },
      'Casual': {
          noiseGateThreshold: -50,
          micGain: 1.0,
          noiseSuppression: true
      },
      'MMO Raid': {
          noiseGateThreshold: -40, // Strict gate (keyboard clicks)
          micGain: 1.0,
          noiseSuppression: true
      }
  };

  function applyProfile(name: string) {
      activeProfile = name;
      const preset = PRESETS[name as keyof typeof PRESETS];
      if (preset) {
          settings.update(s => ({
              ...s,
              noiseGateThreshold: preset.noiseGateThreshold,
              micGain: preset.micGain,
              noiseSuppression: preset.noiseSuppression
          }));
          // AudioEngine watches settings store or needs manual update?
          // AudioEngine usually needs explicit update or it reads from settings.
          // Let's force update
          audioEngine.updateSettings({
              isPTTEnabled: $settings.isPTTEnabled,
              noiseGateThreshold: preset.noiseGateThreshold,
              micGain: preset.micGain
          });
      }
      showPresets = false;
  }

  function exportPresets() {
      const data = JSON.stringify($settings, null, 2);
      const blob = new Blob([data], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = 'void_settings.json';
      a.click();
      URL.revokeObjectURL(url);
  }

  function triggerImport() {
      fileInput.click();
  }

  function handleImport(e: Event) {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (!file) return;
      
      const reader = new FileReader();
      reader.onload = (e) => {
          try {
              const data = JSON.parse(e.target?.result as string);
              settings.update(s => ({ ...s, ...data }));
              audioEngine.updateSettings(data);
              alert("Settings imported successfully!");
          } catch (err) {
              alert("Invalid settings file");
          }
      };
      reader.readAsText(file);
  }
</script>

<input type="file" bind:this={fileInput} class="hidden" accept=".json" on:change={handleImport} />

<aside class="w-80 bg-[#0B0F15] border-l border-white/5 flex flex-col h-full shrink-0 z-10 transition-all duration-300">
    
    <!-- Header: Audio Profile -->
    <div class="p-4 border-b border-white/5">
        <div class="flex items-center justify-between mb-2">
            <h3 class="text-[10px] font-bold text-gray-500 uppercase tracking-widest">Audio Profile</h3>
            <div class="flex items-center space-x-2">
                <button class="text-[10px] text-gray-600 hover:text-white transition-fast" on:click={exportPresets} title="Export Settings">
                    EXPORT
                </button>
                <button class="text-xs text-accent hover:text-white transition-fast" on:click={() => showPresets = !showPresets}>
                    {activeProfile} ▾
                </button>
            </div>
        </div>
        
        {#if showPresets}
            <div class="grid grid-cols-2 gap-2 mb-2" transition:slide>
                {#each Object.keys(PRESETS) as preset}
                    <button 
                        class="text-xs bg-white/5 hover:bg-accent/10 hover:text-accent border border-white/5 rounded p-2 transition-fast text-left {activeProfile === preset ? 'border-accent text-accent' : ''}"
                        on:click={() => applyProfile(preset)}
                    >
                        {preset}
                    </button>
                {/each}
                 <button 
                    class="text-xs bg-white/5 hover:bg-white/10 border border-white/5 rounded p-2 transition-fast text-left text-gray-400"
                    on:click={triggerImport}
                 >
                        Import JSON...
                 </button>
            </div>
        {/if}

        <!-- Mini Visualizer (Fake for now, just CSS animation) -->
        <div class="h-1 flex items-end space-x-0.5 opacity-50">
            {#each Array(20) as _, i}
                <div 
                    class="w-full bg-accent/50 rounded-t transition-all duration-75" 
                    style="height: {$isSpeaking ? Math.random() * 100 : 5}%"
                ></div>
            {/each}
        </div>
    </div>

    <!-- Section: Participants (Only if in call or active chat) -->
    {#if activeChat}
        <div class="p-4 border-b border-white/5 flex-1 overflow-y-auto custom-scrollbar">
            <h3 class="text-[10px] font-bold text-gray-500 uppercase tracking-widest mb-3">Participants</h3>
            <div class="space-y-2">
                <!-- Self -->
                <div class="flex items-center space-x-3 p-2 rounded-lg bg-surface-hover/50 border border-white/5 relative overflow-hidden group">
                     <!-- Volumetric Glow -->
                     <div class="absolute inset-0 bg-accent/5 opacity-0 group-hover:opacity-100 transition-fast"></div>
                     
                    <div class="w-8 h-8 rounded-full bg-accent/20 flex items-center justify-center text-accent text-xs font-bold ring-1 ring-accent/50 shadow-[0_0_10px_rgba(0,229,255,0.2)] {$isSpeaking ? 'animate-pulse ring-accent shadow-[0_0_15px_rgba(0,229,255,0.6)]' : ''}">
                        ME
                    </div>
                    <div class="flex-1">
                         <span class="text-sm font-medium text-gray-200 block">You</span>
                         <div class="h-0.5 bg-gray-700 w-full mt-1 rounded-full overflow-hidden">
                             <div class="h-full bg-green-500 w-[60%]"></div>
                         </div>
                    </div>
                </div>
                
                <!-- Others -->
                {#if activeChat.participants}
                    {#each activeChat.participants as uid}
                         {#if uid !== activeChat.updatedBy} <!-- Filter logic need refinement -->
                            <div class="flex items-center space-x-3 p-2 rounded-lg hover:bg-white/5 transition-fast relative group">
                                <div class="w-8 h-8 rounded-full bg-gray-700 flex items-center justify-center text-gray-300 text-xs font-bold group-hover:ring-1 ring-white/20 transition-all">
                                    {resolveName(uid).slice(0,2)}
                                </div>
                                <div class="flex-1">
                                    <span class="text-sm font-medium text-gray-400 group-hover:text-gray-200 transition-colors">{resolveName(uid)}</span>
                                </div>
                                <!-- Status Icon (Muted/Deaf) -->
                            </div>
                         {/if}
                    {/each}
                {/if}
            </div>
        </div>
    {:else}
        <div class="flex-1"></div>
    {/if}

    <!-- Section: Music Player -->
    <div class="border-t border-white/5 bg-[#080A0E] relative z-20">
        <MusicPlayer 
            {friends} 
            {activeChat} 
            isMinimized={false} 
            embedded={true}
        /> 
    </div>

</aside>

<style>
    .custom-scrollbar::-webkit-scrollbar { width: 4px; }
    .custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
    .custom-scrollbar::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.1); border-radius: 2px; }
    .custom-scrollbar::-webkit-scrollbar-thumb:hover { background: rgba(255,255,255,0.2); }
</style>