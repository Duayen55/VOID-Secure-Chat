<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from 'svelte';
  import { slide } from 'svelte/transition';
  import { tweened } from 'svelte/motion';
  import { cubicOut } from 'svelte/easing';
  import { db, doc, updateDoc, onSnapshot, serverTimestamp } from '$lib/firebase';
  import { user, settings } from '$lib/stores';
  import { audioEngine } from '$lib/audio';

  const dispatch = createEventDispatcher();

  // --- Types ---
  interface Track {
    id: string;
    title: string;
    artist: string;
    url: string; // URL or Blob URL
    duration?: number;
    isYouTube?: boolean;
    thumbnail?: string;
  }

  // --- Props ---
  export let friends: any[] = [];
  export let activeChat: any = null;

  // --- State ---
  let playlist: Track[] = []; 
  
  // Input State
  let inputUrl = "";
  let isAdding = false;

  let currentTrackIndex = 0;
  let isPlaying = false;
  let volume = 0.5;
  let isShuffle = false;
  let isLoop = false;
  let isMinimized = false; 
  let showPlaylist = true;
  let quality = 'high'; // 'high' | 'low'
  
  let audio: HTMLAudioElement;
  let currentTime = 0;
  let duration = 0;

  // Position for floating window
  let position = { x: 20, y: 80 };
  let isDragging = false;
  let dragOffset = { x: 0, y: 0 };
  
  // Embedded Mode
  export let embedded = false;

  // --- Sync State ---
  let syncUnsub: (() => void) | null = null;
  let lastSyncUpdate = 0;
  let syncLatencyThreshold = 0.5; // Reduced for tighter sync
  let isRemoteUpdate = false; 
  let lastServerStateStr = ""; // To prevent processing stale updates
  let ytTimer: any = null;
  let animationFrame: any = null;
  
  // Anchor Point State (Host Authority)
  let anchorTime = 0; // The timestamp when the track started/resumed (Server Time / Host Time)
  let anchorPosition = 0; // The track position at anchorTime
  
  // --- Ducking State ---
  let duckingFactor = tweened(1.0, { duration: 300, easing: cubicOut });
  let releaseTimer: any = null;
  let cleanupSpeaking: (() => void) | null = null;

  // --- Persistence ---
  const STORAGE_KEY = 'void_music_player_settings';
  const DEBUG = true; // Enabled for debugging as requested

  function log(msg: string, data?: any) {
      if (DEBUG) console.log(`[Music] ${msg}`, data || '');
  }

  function sendYT(func: string, args: any[] = []) {
      // 1. Check strict conditions
      if (!playlist[currentTrackIndex]?.isYouTube) return;
      if (!ytIframe) return;
      if (typeof window === 'undefined') return;

      // 2. Debug Log
      log("sendYT", { func, args });

      // 3. Safe Execution
      try {
          ytIframe.contentWindow?.postMessage(JSON.stringify({
              event: 'command',
              func: func,
              args: args
          }), '*');
      } catch (e) {
          console.warn("[Music] sendYT error", e);
      }
  }

  onMount(() => {
    log("initialized");
    // Load settings
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved) {
      const parsed = JSON.parse(saved);
      volume = parsed.volume ?? 0.5;
      isShuffle = parsed.isShuffle ?? false;
      isLoop = parsed.isLoop ?? false;
      quality = parsed.quality ?? 'high';
    }

    // Default position
    if (window.innerWidth > 0) {
        position = { x: window.innerWidth - 380, y: 100 };
    }

    window.addEventListener('keydown', handleGlobalKeydown);
    window.addEventListener('mouseup', stopDrag);
    window.addEventListener('mousemove', handleDrag);
    window.addEventListener('message', handleYTEvents);
    
    // Ducking Listener
    cleanupSpeaking = audioEngine.addSpeakingListener((speaking) => {
        if (!$settings.duckingEnabled) {
            duckingFactor.set(1);
            return;
        }

        if (speaking) {
            clearTimeout(releaseTimer);
            duckingFactor.set(1 - $settings.duckingAmount, { duration: 100 });
        } else {
            clearTimeout(releaseTimer);
            releaseTimer = setTimeout(() => {
                duckingFactor.set(1, { duration: 500 });
            }, $settings.duckingRelease);
        }
    });
  });

  // Apply Volume (Master * Ducking)
  $: effectiveVolume = volume * $duckingFactor;
  
  // Watch volume changes to update audio elements
  $: if (audio) {
      audio.volume = Math.max(0, Math.min(1, effectiveVolume));
  }
  
  // Update YouTube volume if active
  $: if (playlist[currentTrackIndex]?.isYouTube && effectiveVolume !== undefined) {
      const vol = Math.max(0, Math.min(100, effectiveVolume * 100));
      // Safe call via helper
      sendYT('setVolume', [vol]);
  }

  // Watch volume to persist (Raw volume only)
  $: if (typeof localStorage !== 'undefined') {
      const settings = { volume, isShuffle, isLoop, quality };
      localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
  }

  onDestroy(() => {
    window.removeEventListener('keydown', handleGlobalKeydown);
    window.removeEventListener('mouseup', stopDrag);
    window.removeEventListener('mousemove', handleDrag);
    window.removeEventListener('message', handleYTEvents);
    if (syncUnsub) syncUnsub();
    if (cleanupSpeaking) cleanupSpeaking();
    if (releaseTimer) clearTimeout(releaseTimer);
  });

  // Handle YouTube PostMessage Events (Duration, etc.)
  function handleYTEvents(e: MessageEvent) {
      try {
          if (!playlist[currentTrackIndex]?.isYouTube) return;
          
          const data = JSON.parse(e.data);
          
          // YouTube State Change
          // 0 = Ended, 1 = Playing, 2 = Paused
          if (data.event === 'onStateChange') {
             if (data.info === 0) { // Ended
                 onEnded();
             }
             // Re-apply volume on state change (e.g. start)
             sendYT('setVolume', [volume * 100]);
          }

          // YouTube often sends 'infoDelivery' with info.duration
          if (data.event === 'infoDelivery' && data.info) {
              if (data.info.duration) {
                  const d = parseFloat(data.info.duration);
                  // Allow duration updates even if small difference to fix 0.00
                  if (d > 0) {
                      const wasZero = duration === 0 || !playlist[currentTrackIndex]?.duration;
                      duration = d;
                      // Persist duration to playlist
                      if (playlist[currentTrackIndex]) {
                          playlist[currentTrackIndex].duration = d;
                      }
                      // If we just discovered duration, broadcast it immediately so everyone gets the fix
                      if (wasZero) broadcastState(true);
                  }
              }
              // Also sync current time if provided
              if (data.info.currentTime && !isDragging) {
                  // Only sync if significant drift to avoid jitter
                  if (Math.abs(currentTime - data.info.currentTime) > 2) {
                       currentTime = data.info.currentTime;
                  }
              }
              // Detect ended via currentTime near duration (fallback)
              if (duration > 0 && currentTime >= duration - 1) {
                  onEnded();
              }
          }
      } catch (err) {
          // Ignore non-JSON messages
      }
  }

  // --- Sync Logic (Automatic in Call) ---
  $: if (activeChat?.chatId) {
      subscribeToSync(activeChat.chatId);
  } else {
      if (syncUnsub) { syncUnsub(); syncUnsub = null; }
  }

  function subscribeToSync(chatId: string) {
      if (syncUnsub) syncUnsub();
      lastServerStateStr = ""; // Reset on new subscription
      
      syncUnsub = onSnapshot(doc(db, "chats", chatId), (snap) => {
          const data = snap.data();
          if (data?.musicStatus) {
              // Deduplication: Ignore if state is identical to what we last processed
              // This prevents stale updates (e.g. from unrelated doc changes) from reverting local optimistic updates
              const currentStatusStr = JSON.stringify(data.musicStatus);
              if (currentStatusStr === lastServerStateStr) return;

              // Always apply sync if we are not the updater
              if (data.musicStatus.updatedBy !== $user?.uid) {
                  applySyncState(data.musicStatus);
                  lastServerStateStr = currentStatusStr;
              }
          }
      });
  }

  // Fire-and-forget broadcast (Optimistic UI)
  function broadcastState(force = false) {
      if (!activeChat || !$user || isRemoteUpdate) return;
      
      const track = playlist[currentTrackIndex];
      if (!track) return;

      // Calculate Anchor
      // If playing, Anchor Time is Now, Anchor Position is Current Time
      // This establishes a new reference point
      const now = Date.now();
      anchorTime = now;
      anchorPosition = currentTime;

      const state = {
          updatedBy: $user.uid,
          isPlaying,
          trackTitle: track.title,
          trackArtist: track.artist,
          trackUrl: track.url,
          isYouTube: track.isYouTube || false,
          duration: track.duration || duration || 0,
          anchorTime: serverTimestamp(), // Use server time for reference
          anchorPosition, 
          // We DO NOT send currentTime. Clients calculate it.
          playbackSpeed: 1.0 
      };

      // No await - fire and forget
      updateDoc(doc(db, "chats", activeChat.chatId), {
          musicStatus: state
      }).catch(e => console.warn("Sync failed:", e));
      
      lastSyncUpdate = now;
  }

  function applySyncState(state: any) {
      isRemoteUpdate = true;
      const now = Date.now();

      // 1. Check Track
      if (state.trackUrl !== playlist[currentTrackIndex]?.url) {
          const existingIdx = playlist.findIndex(t => t.url === state.trackUrl);
          if (existingIdx !== -1) {
              currentTrackIndex = existingIdx;
              if (state.duration > 0) {
                  playlist[currentTrackIndex].duration = state.duration;
                  duration = state.duration;
              }
          } else {
              playlist = [...playlist, {
                  id: 'sync-' + now,
                  title: state.trackTitle || 'Synced Track',
                  artist: state.trackArtist || 'Remote',
                  url: state.trackUrl,
                  isYouTube: state.isYouTube,
                  duration: state.duration || 0
              }];
              currentTrackIndex = playlist.length - 1;
              if (state.duration > 0) duration = state.duration;
          }
      }

      // 2. Play/Pause State
      if (state.isPlaying !== isPlaying) {
          isPlaying = state.isPlaying;
          if (audio && !state.isYouTube) {
              isPlaying ? audio.play().catch(() => {}) : audio.pause();
          }
          if (state.isYouTube) {
               isPlaying ? startYTTimer() : stopYTTimer();
          }
      }

      // 3. Deterministic Time Sync (Drift Calculation)
      if (state.anchorTime) {
          // Convert Server Timestamp to Millis
          const serverTime = state.anchorTime.toMillis ? state.anchorTime.toMillis() : now;
          
          // Calculate Target Time
          // Target = AnchorPos + (IsPlaying ? (Now - ServerAnchorTime) : 0)
          let targetTime = state.anchorPosition;
          
          if (state.isPlaying) {
              const elapsed = (now - serverTime) / 1000;
              targetTime += elapsed * (state.playbackSpeed || 1.0);
          }

          // Apply Drift Correction
          const drift = Math.abs(currentTime - targetTime);
          
          if (drift > syncLatencyThreshold) {
              console.log(`[Sync] Drift detected: ${drift.toFixed(3)}s. Seeking to ${targetTime.toFixed(3)}s`);
              currentTime = targetTime;
              
              if (audio && !state.isYouTube) {
                  audio.currentTime = currentTime;
              }
              
              if (state.isYouTube && ytIframe) {
                   ytIframe.contentWindow?.postMessage(JSON.stringify({
                       event: 'command',
                       func: 'seekTo',
                       args: [currentTime, true]
                   }), '*');
              }
          }
      }

      isRemoteUpdate = false;
  }

  // --- Audio Logic ---
  $: if (audio) {
    audio.volume = volume;
    audio.loop = isLoop && playlist.length === 1; 
  }
  
  // Update YouTube volume when volume changes
  $: if (ytIframe && typeof window !== 'undefined') {
      ytIframe.contentWindow?.postMessage(JSON.stringify({
          event: 'command',
          func: 'setVolume',
          args: [volume * 100]
      }), '*');
      
      // Update Quality
      const ytQuality = quality === 'high' ? 'hd720' : 'small';
      ytIframe.contentWindow?.postMessage(JSON.stringify({
          event: 'command',
          func: 'setPlaybackQuality',
          args: [ytQuality]
      }), '*');
  }

  function togglePlay() {
    if (isPlaying) {
      if (audio) audio.pause();
    } else {
      if (audio) audio.play().catch(e => console.error("Play error:", e));
    }
    isPlaying = !isPlaying;

    if (playlist[currentTrackIndex]?.isYouTube) {
        if (isPlaying) startYTTimer();
        else stopYTTimer();
    }

    broadcastState();
  }

  function startYTTimer() {
      stopYTTimer();
      // Local timer for UI progress only
      ytTimer = setInterval(() => {
          if (duration === 0 || currentTime < duration) currentTime += 1;
      }, 1000);
  }

  function stopYTTimer() {
      if (ytTimer) clearInterval(ytTimer);
      ytTimer = null;
  }

  function handleSeek() {
      // Optimistic update
      if (audio) audio.currentTime = currentTime;
      if (playlist[currentTrackIndex]?.isYouTube && ytIframe) {
          ytIframe.contentWindow?.postMessage(JSON.stringify({
              event: 'command',
              func: 'seekTo',
              args: [currentTime, true]
          }), '*');
      }
      // Broadcast new anchor immediately
      broadcastState();
  }

  function nextTrack() {
    if (isShuffle) {
      currentTrackIndex = Math.floor(Math.random() * playlist.length);
    } else {
      currentTrackIndex = (currentTrackIndex + 1) % playlist.length;
    }
    playTrack(currentTrackIndex);
  }

  function prevTrack() {
    if (currentTime > 3 && audio) {
      audio.currentTime = 0;
    } else {
      currentTrackIndex = (currentTrackIndex - 1 + playlist.length) % playlist.length;
      playTrack(currentTrackIndex);
    }
  }

  function playTrack(index: number) {
    currentTrackIndex = index;
    isPlaying = true;
    
    stopYTTimer();
    // Initialize duration from saved playlist data if available
    duration = playlist[index].duration || 0;

    if (playlist[index].isYouTube) {
        currentTime = 0;
        // duration is already set if known, otherwise 0
        startYTTimer();
    }

    setTimeout(() => {
        if(audio && !playlist[index].isYouTube) audio.play();
    }, 0);
    broadcastState();
  }

  function onEnded() {
    if (isLoop && playlist.length === 1) return; 
    
    if (isLoop && !isShuffle) {
       nextTrack();
    } else if (currentTrackIndex < playlist.length - 1 || isShuffle) {
       nextTrack();
    } else {
       isPlaying = false;
       broadcastState();
    }
  }

  function handleTimeUpdate() {
    if (!audio) return;
    currentTime = audio.currentTime;
    duration = audio.duration;
    
    // Persist duration for local files too
    if (duration > 0 && playlist[currentTrackIndex] && !playlist[currentTrackIndex].duration) {
        playlist[currentTrackIndex].duration = duration;
    }
    
    // NO broadcast on timeupdate - relies on Anchor Time
  }

  // --- Playlist Management ---
  function removeTrack(index: number) {
      if (index === currentTrackIndex) {
          if (playlist.length > 1) {
              nextTrack();
              // Adjust index if nextTrack moved it or just remove
              // Actually nextTrack changes currentTrackIndex.
              // We need to wait for state update or just remove and fix index.
              // Safer approach: Stop, Remove, Play same index or 0
              // But nextTrack is smoother.
              // Let's just stop if it's the only one.
          } else {
              isPlaying = false;
              stopYTTimer();
          }
      }
      
      const wasPlaying = isPlaying;
      playlist = playlist.filter((_, i) => i !== index);
      
      if (index < currentTrackIndex) {
          currentTrackIndex--;
      } else if (index === currentTrackIndex) {
          // If we removed the current track, we are now pointing to the next one (which shifted down)
          // or we are out of bounds if it was the last one.
          if (currentTrackIndex >= playlist.length) {
              currentTrackIndex = 0;
          }
          if (playlist.length > 0 && wasPlaying) {
             playTrack(currentTrackIndex);
          }
      }
  }

  function addFiles(e: Event) {
    const input = e.target as HTMLInputElement;
    if (input.files) {
      for (let i = 0; i < input.files.length; i++) {
        const file = input.files[i];
        const url = URL.createObjectURL(file);
        playlist = [...playlist, {
          id: crypto.randomUUID(),
          title: file.name.replace(/\.[^/.]+$/, ""),
          artist: 'Local File',
          url: url
        }];
      }
    }
  }

  function addFromUrl() {
      if (!inputUrl.trim()) return;
      
      let title = "External Track";
      let artist = "URL";
      let isYT = false;
      let thumbnail = "";
      let finalUrl = inputUrl;
      let videoId = "";

      // Simple YouTube Detection
      if (inputUrl.includes('youtube.com') || inputUrl.includes('youtu.be')) {
          isYT = true;
          title = "YouTube Video";
          artist = "YouTube";
          
          const regExp = /^.*(youtu.be\/|v\/|u\/\w\/|embed\/|watch\?v=|&v=)([^#&?]*).*/;
          const match = inputUrl.match(regExp);
          if (match && match[2].length === 11) {
              videoId = match[2];
              // Add origin to enable proper JS API support
              const origin = typeof window !== 'undefined' ? window.location.origin : '';
              finalUrl = `https://www.youtube.com/embed/${videoId}?autoplay=1&enablejsapi=1&origin=${origin}`;
              thumbnail = `https://img.youtube.com/vi/${videoId}/mqdefault.jpg`;
          }
      }

      const newTrackId = crypto.randomUUID();
      const newTrack: Track = {
          id: newTrackId,
          title,
          artist,
          url: finalUrl,
          isYouTube: isYT,
          thumbnail,
          duration: isYT ? duration : 0 // Save initial duration
      };

      playlist = [...playlist, newTrack];
      
      inputUrl = "";
      playTrack(playlist.length - 1);

      // Background Fetch Metadata
      if (isYT && videoId) {
          fetchYouTubeMetadata(videoId, newTrackId);
      }
  }

  async function fetchYouTubeMetadata(videoId: string, trackId: string) {
      try {
          const res = await fetch(`https://noembed.com/embed?url=https://www.youtube.com/watch?v=${videoId}`);
          const data = await res.json();
          
          // Update playlist
          playlist = playlist.map(t => {
              if (t.id === trackId) {
                  return { 
                      ...t, 
                      title: data.title || t.title, 
                      artist: data.author_name || t.artist 
                  };
              }
              return t;
          });

          // If currently playing, broadcast update
          if (playlist[currentTrackIndex]?.id === trackId) {
              broadcastState();
          }
      } catch (err) { console.warn("Metadata fetch failed", err); }
  }
  
  // Control YouTube via postMessage
  let ytIframe: HTMLIFrameElement;
  $: if (playlist[currentTrackIndex]?.isYouTube && ytIframe && typeof window !== 'undefined') {
      const action = isPlaying ? 'playVideo' : 'pauseVideo';
      setTimeout(() => {
        ytIframe?.contentWindow?.postMessage(JSON.stringify({
            event: 'command',
            func: action,
            args: []
        }), '*');
      }, 500);
  }

  // --- Dragging ---
  function startDrag(e: MouseEvent) {
    if ((e.target as HTMLElement).closest('button') || (e.target as HTMLElement).closest('input')) return;
    isDragging = true;
    dragOffset = { x: e.clientX - position.x, y: e.clientY - position.y };
  }
  function stopDrag() { isDragging = false; }
  function handleDrag(e: MouseEvent) {
    if (!isDragging) return;
    position = { x: e.clientX - dragOffset.x, y: e.clientY - dragOffset.y };
  }

  // --- Shortcuts ---
  function handleGlobalKeydown(e: KeyboardEvent) {
    if ((e.target as HTMLElement).tagName === 'INPUT') return;
    switch(e.code) {
      case 'Space': e.preventDefault(); togglePlay(); break;
      case 'KeyN': nextTrack(); break;
      case 'KeyB': prevTrack(); break;
    }
  }

  function closePlayer() {
      dispatch('close');
  }
</script>

<!-- Hidden Audio Element (Only for non-YouTube) -->
{#if playlist[currentTrackIndex] && !playlist[currentTrackIndex].isYouTube}
    <audio
      bind:this={audio}
      src={playlist[currentTrackIndex].url}
      on:play={() => { isPlaying = true; broadcastState(); }}
      on:pause={() => { isPlaying = false; broadcastState(); }}
      on:ended={onEnded}
      on:timeupdate={handleTimeUpdate}
      on:loadedmetadata={() => { 
          if(audio) {
              duration = audio.duration;
              if (duration > 0) {
                  // Persist for local track
                  if (playlist[currentTrackIndex]) playlist[currentTrackIndex].duration = duration;
                   // Broadcast immediate duration fix
                   broadcastState(true);
               }
          }
      }}
      on:error={(e) => { console.error("Audio error", e); isPlaying = false; }}
    ></audio>
{/if}

<!-- Player Container -->
<div 
  class="{embedded ? 'w-full h-full bg-transparent flex flex-col' : 'fixed z-[100] bg-[#161A22]/95 backdrop-blur-xl border border-gray-700/50 rounded-2xl shadow-2xl ring-1 ring-white/10'} overflow-hidden text-white transition-all"
  style={embedded ? '' : `left: ${position.x}px; top: ${position.y}px; width: ${isMinimized ? '280px' : '340px'}; max-height: 80vh; display: flex; flex-direction: column;`}
  on:mousedown={embedded ? null : startDrag}
>
  <!-- Header (Visible in Mini Mode) -->
  <div class="p-3 bg-white/5 flex items-center justify-between {embedded ? '' : 'cursor-move'} select-none relative border-b border-white/5 shrink-0">
    {#if activeChat}
        <div class="absolute top-0 left-0 w-full h-[1px] bg-green-500 shadow-[0_0_10px_rgba(34,197,94,0.5)]" title="Syncing with Call"></div>
    {/if}

    <div class="flex items-center space-x-3 overflow-hidden flex-1 mr-2">
        <div class="w-10 h-10 rounded-lg bg-black/40 flex items-center justify-center shrink-0 border border-white/10 overflow-hidden relative group">
            {#if playlist[currentTrackIndex]?.thumbnail}
                <img src={playlist[currentTrackIndex].thumbnail} alt="Art" class="w-full h-full object-cover opacity-80 group-hover:opacity-100 transition" />
            {:else if isPlaying}
                <span class="animate-pulse text-xs">🎵</span>
            {:else}
                <span class="text-xs">⏸</span>
            {/if}
        </div>
        
        <!-- Marquee / Title Area -->
        <div class="flex flex-col overflow-hidden w-full relative mask-gradient">
            {#if isPlaying || isMinimized}
                <div class="whitespace-nowrap {isPlaying ? 'animate-marquee' : ''}">
                    <span class="text-xs font-bold text-gray-200 mr-8">{playlist[currentTrackIndex]?.title || 'Void Player'}</span>
                    <span class="text-xs font-bold text-gray-200 mr-8">{playlist[currentTrackIndex]?.title || 'Void Player'}</span>
                </div>
            {:else}
                 <span class="text-xs font-bold truncate text-gray-200">{playlist[currentTrackIndex]?.title || 'Void Player'}</span>
            {/if}
            <span class="text-[10px] text-gray-400 truncate">{playlist[currentTrackIndex]?.artist || 'Ready'}</span>
            
            <!-- Mini Progress Bar (Only visible when minimized) -->
            {#if isMinimized}
                <div class="h-0.5 bg-gray-800 rounded-full mt-1 w-full overflow-hidden">
                    <div class="h-full bg-purple-500 transition-all duration-1000" style="width: {(currentTime / (duration || 1)) * 100}%"></div>
                </div>
            {/if}
        </div>
    </div>
    
    <div class="flex items-center space-x-1 shrink-0">
        {#if isMinimized}
             <button class="p-1.5 hover:bg-white/10 rounded-full" on:click|stopPropagation={togglePlay}>
                 {#if isPlaying}
                     <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zM7 8a1 1 0 012 0v4a1 1 0 11-2 0V8zm5-1a1 1 0 00-1 1v4a1 1 0 102 0V8a1 1 0 00-1-1z" clip-rule="evenodd" /></svg>
                 {:else}
                     <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM9.555 7.168A1 1 0 008 8v4a1 1 0 001.555.832l3-2a1 1 0 000-1.664l-3-2z" clip-rule="evenodd" /></svg>
                 {/if}
             </button>
        {/if}
        <button class="p-1.5 hover:bg-white/10 rounded-lg text-gray-400 hover:text-white transition" on:click={() => isMinimized = !isMinimized}>
            {isMinimized ? '🔽' : '🔼'}
        </button>
        <button class="p-1.5 hover:bg-red-500/20 rounded-lg text-gray-400 hover:text-red-400 transition" on:click={closePlayer}>
            ✕
        </button>
    </div>
  </div>

  <!-- Content -->
  {#if !isMinimized}
  <div class="p-4 space-y-4 overflow-y-auto scrollbar-thin scrollbar-thumb-gray-700 scrollbar-track-transparent" transition:slide>
    
    <!-- YouTube Embed (Hidden/Audio Only) -->
    {#if playlist[currentTrackIndex]?.isYouTube}
        <div class="w-1 h-1 opacity-0 overflow-hidden absolute pointer-events-none">
            <iframe 
                bind:this={ytIframe}
                src={playlist[currentTrackIndex].url} 
                title="YouTube video player" 
                frameborder="0" 
                allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture" 
                allowfullscreen
            ></iframe>
        </div>
        <div class="w-full h-24 bg-black/40 rounded-xl flex items-center justify-center border border-white/5 relative overflow-hidden group">
             <div class="absolute inset-0 bg-gradient-to-br from-purple-900/20 to-blue-900/20"></div>
             <span class="text-4xl relative z-10 opacity-50 group-hover:opacity-100 transition-opacity">YouTube Audio</span>
             <span class="absolute bottom-2 text-[10px] text-gray-500">Video hidden for audio-only experience</span>
        </div>
    {/if}

    <!-- Progress -->
    <div class="space-y-1.5">
        <div class="relative group">
            <input 
                type="range" 
                min="0" max={duration || 100} 
                bind:value={currentTime} 
                on:change={handleSeek} 
                class="w-full h-1 bg-gray-700 rounded-lg appearance-none cursor-pointer accent-purple-500 hover:accent-purple-400"
            />
        </div>
        <div class="flex justify-between text-[10px] text-gray-500 font-mono tracking-wider">
            <span>{Math.floor(currentTime/60)}:{Math.floor(currentTime%60).toString().padStart(2,'0')}</span>
            <span>{Math.floor(duration/60)}:{Math.floor(duration%60).toString().padStart(2,'0')}</span>
        </div>
    </div>

    <!-- Main Controls -->
    <div class="flex items-center justify-between px-2">
        <button class="text-gray-500 hover:text-purple-400 transition" on:click={() => isShuffle = !isShuffle} class:text-purple-400={isShuffle} title="Shuffle">
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4" /></svg>
        </button>
        
        <div class="flex items-center space-x-4">
            <button class="text-gray-400 hover:text-white transition transform active:scale-95" on:click={prevTrack}>
                 <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="currentColor" viewBox="0 0 24 24"><path d="M6 6h2v12H6zm3.5 6l8.5 6V6z"/></svg>
            </button>
            <button class="w-10 h-10 bg-white text-black rounded-full flex items-center justify-center hover:scale-105 transition shadow-lg shadow-white/10" on:click={togglePlay}>
                {#if isPlaying}
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zM7 8a1 1 0 012 0v4a1 1 0 11-2 0V8zm5-1a1 1 0 00-1 1v4a1 1 0 102 0V8a1 1 0 00-1-1z" clip-rule="evenodd" /></svg>
                {:else}
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 ml-0.5" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM9.555 7.168A1 1 0 008 8v4a1 1 0 001.555.832l3-2a1 1 0 000-1.664l-3-2z" clip-rule="evenodd" /></svg>
                {/if}
            </button>
            <button class="text-gray-400 hover:text-white transition transform active:scale-95" on:click={nextTrack}>
                <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="currentColor" viewBox="0 0 24 24"><path d="M6 18l8.5-6L6 6v12zM16 6v12h2V6h-2z"/></svg>
            </button>
        </div>

        <button class="text-gray-500 hover:text-purple-400 transition" on:click={() => isLoop = !isLoop} class:text-purple-400={isLoop} title="Loop">
             <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" /></svg>
        </button>
    </div>

    <!-- Volume & Quality -->
    <div class="flex items-center justify-between px-1 pt-1">
        <div class="flex items-center space-x-2 flex-1 mr-4">
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 text-gray-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.536 8.464a5 5 0 010 7.072m2.828-9.9a9 9 0 010 12.728M5.586 15H4a1 1 0 01-1-1v-4a1 1 0 011-1h1.586l4.707-4.707C10.923 3.663 12 4.109 12 5v14c0 .891-1.077 1.337-1.707.707L5.586 15z" />
            </svg>
            <input 
                type="range" 
                min="0" max="1" step="0.05"
                bind:value={volume} 
                class="w-full h-1 bg-gray-700 rounded-lg appearance-none cursor-pointer accent-gray-400"
            />
        </div>
        
        <!-- Quality Toggle -->
        <button 
            class="text-[9px] font-bold px-1.5 py-0.5 rounded border transition {quality === 'high' ? 'bg-purple-500/20 text-purple-300 border-purple-500/30' : 'bg-white/5 text-gray-500 border-white/10'}"
            on:click={() => quality = quality === 'high' ? 'low' : 'high'}
            title="Toggle Audio Quality"
        >
            {quality === 'high' ? 'HQ' : 'LQ'}
        </button>
    </div>

    <!-- Playlist List (Conditional) -->
    {#if showPlaylist && playlist.length > 0}
    <div class="max-h-40 overflow-y-auto space-y-1 pr-1 scrollbar-thin scrollbar-thumb-gray-700">
        {#each playlist as track, i}
            <div 
                class="flex items-center justify-between p-2 rounded-lg text-xs group transition cursor-pointer {i === currentTrackIndex ? 'bg-purple-500/20 text-purple-200 border border-purple-500/30' : 'hover:bg-white/5 text-gray-400'}"
                on:click={() => playTrack(i)}
            >
                <div class="flex items-center space-x-2 truncate">
                    <span class="w-4 text-center opacity-50">{i + 1}</span>
                    <span class="truncate max-w-[180px]">{track.title}</span>
                </div>
                <button 
                    class="opacity-0 group-hover:opacity-100 p-1 hover:text-red-400 transition"
                    on:click|stopPropagation={() => removeTrack(i)}
                    title="Remove"
                >
                    ✕
                </button>
            </div>
        {/each}
    </div>
    {/if}

    <!-- Add & Playlist -->
    <div class="pt-3 border-t border-white/5 space-y-3">
        <!-- Input -->
        <div class="flex space-x-2">
            <input 
                type="text" 
                bind:value={inputUrl} 
                placeholder="Paste MP3 or YouTube URL..." 
                class="flex-1 bg-black/20 text-xs p-2 rounded-lg border border-white/5 outline-none focus:border-purple-500/50 transition text-gray-300 placeholder-gray-600"
                on:keydown={(e) => e.key === 'Enter' && addFromUrl()}
            />
            <button on:click={addFromUrl} class="bg-purple-600/80 px-3 rounded-lg text-xs font-bold hover:bg-purple-500 transition shadow-lg shadow-purple-900/20">ADD</button>
        </div>


        
        <label class="block text-center text-[10px] text-gray-600 hover:text-gray-400 cursor-pointer transition">
            Import Local Files
            <input type="file" multiple accept="audio/*" class="hidden" on:change={addFiles} />
        </label>
    </div>

  </div>
  {/if}
</div>

<style>
  .custom-scrollbar::-webkit-scrollbar { width: 3px; }
  .custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
  .custom-scrollbar::-webkit-scrollbar-thumb { background: #374151; border-radius: 2px; }
  .custom-scrollbar::-webkit-scrollbar-thumb:hover { background: #4b5563; }

  .mask-gradient {
      mask-image: linear-gradient(to right, transparent, black 5%, black 95%, transparent);
      -webkit-mask-image: linear-gradient(to right, transparent, black 5%, black 95%, transparent);
  }

  @keyframes marquee {
      0% { transform: translateX(0); }
      100% { transform: translateX(-50%); }
  }
  
  .animate-marquee {
      display: inline-block;
      animation: marquee 10s linear infinite;
  }
</style>