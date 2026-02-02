<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import { fade, slide } from 'svelte/transition';
  import { settings } from '$lib/stores';
  import { updateVideoQuality } from '$lib/webrtc';
  import ScreenShareSettings from './ScreenShareSettings.svelte';

  export let stream: MediaStream | null = null;
  export let label = "You are sharing your screen";
  export let onClose: () => void;
  export let peerConnection: RTCPeerConnection | null = null;
  export let onChangeSource: (() => void) | undefined = undefined;

  const dispatch = createEventDispatcher();

  // Watch for bitrate AND resolution changes
  $: if ($settings.screenBitrate || $settings.screenResolution) {
      updateVideoQuality($settings.screenBitrate, $settings.screenResolution, peerConnection);
  }

  // Floating State
  let container: HTMLDivElement;
  let videoElement: HTMLVideoElement;
  let isDragging = false;
  let startX = 0, startY = 0;
  let left = 20, top = 120; // Default lower to avoid overlap with top call controls
  let width = 320;
  let height = 180;
  
  // Panel State
  let isMinimized = false;
  let showSettings = false;

  // Persistence & PiP
  onMount(() => {
      // Restore position
      const savedPos = localStorage.getItem('screenSharePanelPos');
      if (savedPos) {
          try {
             const parsed = JSON.parse(savedPos);
             left = parsed.left;
             top = parsed.top;
          } catch(e) {}
      }
  });

  function savePosition() {
      localStorage.setItem('screenSharePanelPos', JSON.stringify({ left, top }));
  }

  async function togglePiP() {
      try {
          if (document.pictureInPictureElement) {
              await document.exitPictureInPicture();
          } else if (videoElement) {
              await videoElement.requestPictureInPicture();
          }
      } catch(e) {
          console.error("PiP failed:", e);
      }
  }

  // Drag Logic
  function handleMouseDown(e: MouseEvent) {
    if ((e.target as HTMLElement).closest('.drag-handle')) {
        isDragging = true;
        startX = e.clientX - left;
        startY = e.clientY - top;
        window.addEventListener('mousemove', handleMouseMove);
        window.addEventListener('mouseup', handleMouseUp);
    }
  }

  function handleMouseMove(e: MouseEvent) {
    if (!isDragging) return;
    left = e.clientX - startX;
    top = e.clientY - startY;
  }

  function handleMouseUp() {
    isDragging = false;
    savePosition(); // Save on drop
    window.removeEventListener('mousemove', handleMouseMove);
    window.removeEventListener('mouseup', handleMouseUp);
  }

  // Reactive Video Source
  $: if (videoElement && stream) {
    videoElement.srcObject = stream;
  }
</script>

<div 
  bind:this={container}
  class="fixed z-[100] bg-[#1e1e2f] border border-[#2f3136] rounded-xl shadow-2xl flex flex-col overflow-hidden transition-shadow hover:shadow-black/50 ring-1 ring-white/5"
  style="left: {left}px; top: {top}px; width: {isMinimized ? 200 : width}px; height: {isMinimized ? 40 : 'auto'};"
  on:mousedown={handleMouseDown}
>
  <!-- Header / Drag Handle -->
  <div class="drag-handle bg-[#1e1e2f] p-2 flex justify-between items-center cursor-move select-none border-b border-[#2f3136] h-10">
    <div class="flex items-center space-x-2 px-1">
        <div class="w-2 h-2 bg-green-500 rounded-full animate-pulse shadow-[0_0_8px_rgba(34,197,94,0.6)]"></div>
        <span class="text-xs font-bold text-gray-200 truncate max-w-[120px]">{label}</span>
    </div>
    
    <div class="flex items-center space-x-1">
        <!-- PiP Toggle -->
        <button 
           on:click|stopPropagation={togglePiP}
           class="p-1 hover:bg-[#36393f] rounded text-gray-400 hover:text-white transition"
           title="Picture-in-Picture"
        >
           📺
        </button>

        <!-- Settings Toggle -->
        <button 
           on:click|stopPropagation={() => showSettings = !showSettings}
           class="p-1 hover:bg-[#36393f] rounded text-gray-400 hover:text-white transition"
           title="Stream Settings"
        >
           ⚙️
        </button>

        <!-- Minimize/Maximize -->
        <button 
           on:click|stopPropagation={() => isMinimized = !isMinimized}
           class="p-1 hover:bg-[#36393f] rounded text-gray-400 hover:text-white transition"
        >
           {isMinimized ? '□' : '_'}
        </button>

        <!-- Close -->
        <button 
           on:click|stopPropagation={onClose} 
           class="p-1 hover:bg-red-500/20 rounded text-gray-400 hover:text-red-400 transition"
           title="Stop Sharing"
        >
           ✕
        </button>
    </div>
  </div>

  <!-- Main Content (Video) -->
  {#if !isMinimized}
      <div class="relative bg-black group aspect-video w-full">
        <video 
          bind:this={videoElement} 
          autoplay 
          playsinline 
          muted 
          class="w-full h-full object-contain"
        ></video>

        <!-- Hover Overlay -->
        <div class="absolute inset-0 bg-black/60 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center pointer-events-none">
            <span class="text-white text-[10px] font-mono tracking-widest bg-black/50 px-2 py-1 rounded">
               {Math.round($settings.screenBitrate / 1000)} Kbps • {$settings.screenResolution}
            </span>
        </div>

        <!-- Settings Popover -->
        {#if showSettings}
            <div 
               class="absolute top-2 right-2 w-64 bg-[#1e1e2f]/95 backdrop-blur border border-[#2f3136] rounded-lg p-4 shadow-xl z-20 text-left" 
               transition:slide={{ duration: 200 }}
            >
               <ScreenShareSettings {onChangeSource} />
            </div>
        {/if}
      </div>

      <!-- Resize Handle (Bottom Right) -->
      <div 
         class="absolute bottom-0 right-0 w-4 h-4 cursor-nwse-resize z-10"
         on:mousedown|stopPropagation={(e) => {
             const startW = width;
             const startH = height; // Not used if aspect ratio maintained, but good for free resize
             const startX = e.clientX;
             
             function onMove(me: MouseEvent) {
                 width = Math.max(200, startW + (me.clientX - startX));
                 // Maintain 16:9 roughly if we wanted, or just let CSS handle height via aspect-video
             }
             
             function onUp() {
                 window.removeEventListener('mousemove', onMove);
                 window.removeEventListener('mouseup', onUp);
             }
             
             window.addEventListener('mousemove', onMove);
             window.addEventListener('mouseup', onUp);
         }}
      >
         <svg viewBox="0 0 24 24" class="w-4 h-4 text-gray-500 transform rotate-90 fill-current opacity-50 hover:opacity-100">
             <path d="M22 22L12 22L22 12z" />
         </svg>
      </div>
  {/if}
</div>
