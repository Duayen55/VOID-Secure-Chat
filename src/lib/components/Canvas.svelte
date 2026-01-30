<script lang="ts">
  import { onMount } from 'svelte';

  let canvas: HTMLDivElement;
  let scale = $state(1);
  let translateX = $state(0);
  let translateY = $state(0);
  let isDragging = false;
  let startX = 0;
  let startY = 0;

  function handleWheel(e: WheelEvent) {
    if (e.ctrlKey) {
      e.preventDefault();
      const s = Math.exp(-e.deltaY * 0.01);
      scale = Math.min(Math.max(0.1, scale * s), 5);
    } else {
      translateX -= e.deltaX;
      translateY -= e.deltaY;
    }
  }

  function handleMouseDown(e: MouseEvent) {
    if (e.button === 1 || (e.button === 0 && e.altKey)) { // Middle mouse or Alt+Left
      isDragging = true;
      startX = e.clientX - translateX;
      startY = e.clientY - translateY;
      e.preventDefault();
    }
  }

  function handleMouseMove(e: MouseEvent) {
    if (isDragging) {
      translateX = e.clientX - startX;
      translateY = e.clientY - startY;
    }
  }

  function handleMouseUp() {
    isDragging = false;
  }
</script>

<div 
  class="w-full h-full overflow-hidden bg-[#050505] relative cursor-crosshair select-none"
  onwheel={handleWheel}
  onmousedown={handleMouseDown}
  onmousemove={handleMouseMove}
  onmouseup={handleMouseUp}
  onmouseleave={handleMouseUp}
  role="application"
  aria-label="Infinite Canvas"
>
  <div 
    bind:this={canvas}
    class="absolute origin-top-left transition-transform duration-75 ease-out will-change-transform"
    style="transform: translate({translateX}px, {translateY}px) scale({scale});"
  >
    <!-- Grid Pattern -->
    <div class="absolute -top-[5000px] -left-[5000px] w-[10000px] h-[10000px] pointer-events-none opacity-20"
      style="background-image: radial-gradient(#333 1px, transparent 1px); background-size: 40px 40px;"
    ></div>

    <!-- Content Slot -->
    <div class="relative">
      <slot />
    </div>
  </div>

  <!-- HUD / Controls -->
  <div class="absolute bottom-4 right-4 bg-black/50 backdrop-blur border border-white/10 p-2 text-xs font-mono text-white/50">
    XYZ: {Math.round(translateX)}, {Math.round(translateY)} | Z: {scale.toFixed(2)}x
  </div>
</div>
