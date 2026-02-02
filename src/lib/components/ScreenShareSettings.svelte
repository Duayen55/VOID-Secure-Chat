<script lang="ts">
  import { settings } from '$lib/stores';
  import { slide } from 'svelte/transition';

  export let onChangeSource: (() => void) | undefined = undefined;

  // Options
  const resolutions = [
    { label: '720p (HD)', value: '720p' },
    { label: '1080p (FHD)', value: '1080p' },
    { label: '1440p (2K)', value: '1440p' }
  ];

  // Bitrate Slider (0 - 5000 kbps)
  let bitrateValue = 3000; // Default

  function handleBitrateChange() {
     $settings.screenBitrate = bitrateValue * 1000;
  }
</script>

<div class="space-y-6 text-sm max-h-80 overflow-y-auto custom-scrollbar pr-2">
    
    <!-- Source Selection Hint -->
    <div class="space-y-2">
        {#if onChangeSource}
            <button 
                on:click={onChangeSource}
                class="w-full py-2 px-3 bg-blue-600 hover:bg-blue-500 text-white rounded-lg transition font-medium flex items-center justify-center space-x-2"
            >
                <span>🖥️</span>
                <span>Change Screen / Window</span>
            </button>
        {/if}

        <div class="p-3 bg-blue-900/20 border border-blue-800/50 rounded-lg flex items-start space-x-3">
            <span class="text-xl">💡</span>
            <div class="text-gray-300 text-xs leading-relaxed">
                Select <strong>Screen</strong>, <strong>Window</strong>, or <strong>Tab</strong> when the browser prompt appears. 
                System audio is only available when sharing a <strong>Tab</strong> or <strong>Entire Screen</strong> (on some platforms).
            </div>
        </div>
    </div>

    <!-- Resolution -->
    <div class="space-y-2">
        <label class="text-gray-400 font-medium text-xs uppercase tracking-wide">Max Resolution</label>
        <div class="grid grid-cols-2 gap-2">
            {#each resolutions as res}
                <button 
                    class="px-3 py-2 rounded-md border transition-all text-left flex justify-between items-center
                    {$settings.screenResolution === res.value 
                        ? 'bg-[#5865F2]/20 border-[#5865F2] text-[#5865F2]' 
                        : 'bg-[#2f3136] border-[#40444b] text-gray-400 hover:border-gray-500'}"
                    on:click={() => $settings.screenResolution = res.value}
                >
                    <span>{res.label}</span>
                    {#if $settings.screenResolution === res.value}
                        <span class="text-[#5865F2] text-xs">●</span>
                    {/if}
                </button>
            {/each}
        </div>
    </div>

    <!-- Bitrate Slider -->
    <div class="space-y-3">
        <div class="flex justify-between items-center">
             <label class="text-gray-400 font-medium text-xs uppercase tracking-wide">Max Bitrate</label>
             <span class="text-[#5865F2] font-mono text-xs">{bitrateValue} kbps</span>
        </div>

        <div class="relative pt-1">
             <input 
                type="range" 
                min="0" 
                max="5000" 
                step="100" 
                bind:value={bitrateValue}
                on:input={handleBitrateChange}
                class="w-full h-2 bg-[#2f3136] rounded-lg appearance-none cursor-pointer accent-[#5865F2]"
             />
             <div class="flex justify-between text-[10px] text-gray-500 mt-1 font-mono">
                 <span>0</span>
                 <span>2500</span>
                 <span>5000</span>
             </div>
        </div>
        <p class="text-[10px] text-gray-500">Higher bitrate = better quality but requires faster internet.</p>
    </div>

    <!-- Frame Rate -->
    <div class="space-y-2">
        <label class="text-gray-400 font-medium text-xs uppercase tracking-wide">Frame Rate Cap</label>
        <input 
            type="range" min="15" max="60" step="15"
            bind:value={$settings.screenFramerate}
            class="w-full h-1 bg-[#2f3136] rounded-lg appearance-none cursor-pointer accent-[#5865F2]"
        />
        <div class="flex justify-between text-xs text-gray-500">
            <span>15 FPS</span>
            <span>30 FPS</span>
            <span>60 FPS</span>
        </div>
    </div>

    <!-- Audio Toggle -->
    <div class="flex items-center justify-between p-3 bg-[#2f3136] rounded-lg border border-[#40444b]">
        <div class="flex flex-col">
            <span class="text-white font-medium">Share Audio</span>
            <span class="text-xs text-gray-500">Include system/tab audio</span>
        </div>
        <button 
           on:click={() => $settings.shareSystemAudio = !$settings.shareSystemAudio} 
           class={`w-12 h-6 rounded-full p-1 transition-colors duration-200 relative ${$settings.shareSystemAudio ? 'bg-[#5865F2]' : 'bg-[#202225]'}`}
        >
           <div class={`w-4 h-4 bg-white rounded-full shadow transform transition-transform duration-200 ${$settings.shareSystemAudio ? 'translate-x-6' : 'translate-x-0'}`}></div>
        </button>
    </div>

</div>

<style>
  .custom-scrollbar::-webkit-scrollbar {
    width: 4px;
  }
  .custom-scrollbar::-webkit-scrollbar-track {
    background: #1f2937; 
  }
  .custom-scrollbar::-webkit-scrollbar-thumb {
    background: #4b5563; 
    border-radius: 2px;
  }
</style>
