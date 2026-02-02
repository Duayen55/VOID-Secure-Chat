<script lang="ts">
  import { onMount } from 'svelte';

  export let left = 100;
  export let top = 100;
  export let width = "480px";
  export let height = "360px";
  export let title = "Window";
  export let zIndex = 50;
  export let onClose: (() => void) | undefined = undefined;
  
  let isDragging = false;
  let startX = 0, startY = 0;
  
  function handleMouseDown(e: MouseEvent) {
     isDragging = true;
     startX = e.clientX - left;
     startY = e.clientY - top;
     window.addEventListener('mousemove', handleMouseMove);
     window.addEventListener('mouseup', handleMouseUp);
  }

  function handleMouseMove(e: MouseEvent) {
     if(!isDragging) return;
     left = e.clientX - startX;
     top = e.clientY - startY;
  }

  function handleMouseUp() {
     isDragging = false;
     window.removeEventListener('mousemove', handleMouseMove);
     window.removeEventListener('mouseup', handleMouseUp);
  }
</script>

<div class="fixed bg-[#1e1e2f] border border-[#2f3136] shadow-2xl rounded-xl overflow-hidden flex flex-col"
     style="left: {left}px; top: {top}px; width: {width}; height: {height}; z-index: {zIndex}; box-shadow: 0 10px 30px rgba(0,0,0,0.5);">
    
    <!-- Title Bar -->
    <div class="h-8 bg-[#2f3136] border-b border-[#202225] flex items-center justify-between px-3 cursor-move select-none"
         on:mousedown={handleMouseDown}>
         <div class="flex items-center space-x-2">
             <span class="text-xs font-bold text-gray-300 uppercase tracking-wider">{title}</span>
         </div>
         {#if onClose}
            <button on:click={onClose} class="text-gray-400 hover:text-white transition">
                ✕
            </button>
         {/if}
    </div>

    <!-- Content -->
    <div class="flex-1 relative overflow-hidden bg-black group">
        <slot></slot>
        
        <!-- Resize Handle (Visual Only for now, or simplistic) -->
        <div class="absolute bottom-0 right-0 w-4 h-4 cursor-se-resize"></div>
    </div>
</div>
