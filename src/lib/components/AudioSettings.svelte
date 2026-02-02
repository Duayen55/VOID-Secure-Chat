<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from 'svelte';
  import { settings } from '$lib/stores';
  import { audioEngine } from '$lib/audio';

  export let show = true;

  // Dispatcher for external integration if needed
  const dispatch = createEventDispatcher();

  // --- State ---
  let microphones: MediaDeviceInfo[] = [];
  let speakers: MediaDeviceInfo[] = [];
  
  // Noise Gate Runtime State
  let currentLevel = -100; // dB
  let isGated = false;

  // Monitoring
  let monitorAudio = false; // Toggle to hear yourself
  let audioOutputElement: HTMLAudioElement;

  // Visualizer
  let canvas: HTMLCanvasElement;
  let canvasCtx: CanvasRenderingContext2D | null = null;
  
  // Key listening for PTT bind
  let isListeningForKey = false;
  
  // Profile Management
  let newProfileName = "";
  
  let cleanupLevelListener: () => void;

  onMount(async () => {
    await getDevices();
    navigator.mediaDevices.addEventListener('devicechange', getDevices);
    
    // Use AudioEngine
    cleanupLevelListener = audioEngine.addLevelListener((db, gated) => {
        currentLevel = db;
        isGated = gated;
        drawVisualizer(db);
    });
    
    // Apply initial settings including gain
    audioEngine.updateSettings($settings);
    restartEngine();
  });

  // ... (rest of lifecycle)

  // Profile Logic
  function createProfile() {
      if (!newProfileName.trim()) return;
      const newProfile = {
          id: crypto.randomUUID(),
          name: newProfileName,
          isPTTEnabled: $settings.isPTTEnabled,
          noiseGateThreshold: $settings.noiseGateThreshold,
          micGain: $settings.micGain || 1.0
      };
      $settings.profiles = [...$settings.profiles, newProfile];
      $settings.activeProfileId = newProfile.id;
      newProfileName = "";
  }

  function deleteProfile() {
      if (!$settings.activeProfileId) return;
      $settings.profiles = $settings.profiles.filter(p => p.id !== $settings.activeProfileId);
      $settings.activeProfileId = null;
  }

  function loadProfile() {
      if (!$settings.activeProfileId) return;
      const profile = $settings.profiles.find(p => p.id === $settings.activeProfileId);
      if (profile) {
          $settings.isPTTEnabled = profile.isPTTEnabled;
          $settings.noiseGateThreshold = profile.noiseGateThreshold;
          $settings.micGain = profile.micGain;
          audioEngine.updateSettings($settings);
      }
  }

  // Auto-save changes to active profile
  $: if ($settings.activeProfileId && ($settings.isPTTEnabled !== undefined || $settings.noiseGateThreshold !== undefined || $settings.micGain !== undefined)) {
      const idx = $settings.profiles.findIndex(p => p.id === $settings.activeProfileId);
      if (idx !== -1) {
          const p = $settings.profiles[idx];
          // Check if changed to avoid loops? Svelte store update might trigger loop if not careful.
          // But here we are mutating the array inside the store.
          // We only want to update if values differ.
          if (p.isPTTEnabled !== $settings.isPTTEnabled || 
              p.noiseGateThreshold !== $settings.noiseGateThreshold || 
              p.micGain !== $settings.micGain) {
                  
              const updated = { ...p, 
                  isPTTEnabled: $settings.isPTTEnabled, 
                  noiseGateThreshold: $settings.noiseGateThreshold, 
                  micGain: $settings.micGain 
              };
              const newProfiles = [...$settings.profiles];
              newProfiles[idx] = updated;
              $settings.profiles = newProfiles;
          }
      }
  }
  
  // Sync AudioEngine when settings change (including Gain/Ducking)
  $: {
      audioEngine.updateSettings($settings);
  }

  onDestroy(() => {
    if (cleanupLevelListener) cleanupLevelListener();
    navigator.mediaDevices.removeEventListener('devicechange', getDevices);
  });

  async function getDevices() {
    try {
      // Request permission first to get labels
      // Note: This might interfere with running AudioEngine if not careful, 
      // but usually okay to request transient stream
      const permStream = await navigator.mediaDevices.getUserMedia({ audio: true });
      permStream.getTracks().forEach(t => t.stop());

      const devices = await navigator.mediaDevices.enumerateDevices();
      microphones = devices.filter(d => d.kind === 'audioinput');
      speakers = devices.filter(d => d.kind === 'audiooutput');

      // Set defaults if not set in store
      if (!$settings.selectedMicId && microphones.length > 0) {
          $settings.selectedMicId = microphones[0].deviceId;
      }
      if (!$settings.selectedSpeakerId && speakers.length > 0) {
          $settings.selectedSpeakerId = speakers[0].deviceId;
      }
    } catch (err) {
      console.error("Error enumerating devices:", err);
    }
  }

  async function restartEngine() {
      if (!$settings.selectedMicId) return;
      const constraints = {
        audio: {
          deviceId: { exact: $settings.selectedMicId },
          noiseSuppression: $settings.noiseSuppression,
          echoCancellation: $settings.echoCancellation,
          autoGainControl: $settings.autoGainControl
        }
      };
      await audioEngine.setInput(constraints.audio);
  }

  // Watch for setting changes that require restart
  $: if ($settings.selectedMicId || $settings.noiseSuppression || $settings.echoCancellation || $settings.autoGainControl) {
      // Debounce or just call? For now direct call.
      // But only if we are "active" (e.g. in this view or call). 
      // Actually, if we change mic in settings, we WANT it to change globally.
      restartEngine();
  }
  
  // PTT Key Bind
  function startKeyListen() {
      isListeningForKey = true;
      window.addEventListener('keydown', handleKeyBind);
  }
  
  function handleKeyBind(e: KeyboardEvent) {
      e.preventDefault();
      $settings.pttKey = e.key;
      isListeningForKey = false;
      window.removeEventListener('keydown', handleKeyBind);
  }

  function drawVisualizer(db: number) {
     if (!canvas) return;
     if (!canvasCtx) canvasCtx = canvas.getContext('2d');
     if (!canvasCtx) return;

     const width = canvas.width;
     const height = canvas.height;
     
     canvasCtx.clearRect(0, 0, width, height);
     
     // DB range: -60 to 0 (approx)
     // Normalize to 0-1
     let t = (db + 60) / 60; 
     if (t < 0) t = 0;
     if (t > 1) t = 1;
     
     const barWidth = width * t;
     
     canvasCtx.fillStyle = isGated ? '#ef4444' : '#22c55e'; // Red if gated, Green if open
     canvasCtx.fillRect(0, 0, barWidth, height);
     
     // Draw Threshold Line
     const threshT = ($settings.noiseGateThreshold + 60) / 60;
     const threshX = width * threshT;
     
     canvasCtx.fillStyle = '#ffffff';
     canvasCtx.fillRect(threshX, 0, 2, height);

     // Text Info
     canvasCtx.fillStyle = '#fff';
     canvasCtx.font = '10px sans-serif';
     canvasCtx.fillText(`Level: ${currentLevel.toFixed(1)} dB`, 5, 12);
     canvasCtx.fillText(`Gate: ${$settings.noiseGateThreshold} dB`, threshX + 5, 12);
  }

  // Handle changes
  function handleConstraintChange() {
    // Restart to apply constraints reliably
    restartEngine();
  }
  
  function handleDeviceChange() {
      restartEngine();
  }

  $: if (audioOutputElement) {
      if (monitorAudio) {
          const stream = audioEngine.getProcessedStream();
          if (audioOutputElement.srcObject !== stream) {
              audioOutputElement.srcObject = stream;
          }
          audioOutputElement.play().catch(() => {});
      } else {
          audioOutputElement.pause();
          audioOutputElement.srcObject = null;
      }
  }

  $: if ($settings.selectedSpeakerId && audioOutputElement && 'setSinkId' in audioOutputElement) {
      // @ts-ignore
      audioOutputElement.setSinkId($settings.selectedSpeakerId).catch(e => console.warn(e));
  }

</script>

<div class="bg-gray-900 text-gray-200 p-6 rounded-xl w-full max-w-2xl mx-auto shadow-xl border border-gray-700">
    <div class="mb-6 border-b border-gray-700 pb-4">
        <h2 class="text-xl font-bold text-white flex items-center">
            <span class="mr-2">🎙️</span> Audio Settings
        </h2>
        <p class="text-sm text-gray-400 mt-1">Configure input, output, and processing.</p>
    </div>

    <div class="space-y-6">
        <!-- Game Profiles -->
        <div class="bg-blue-900/20 rounded-lg p-4 border border-blue-500/30">
            <div class="flex justify-between items-center mb-3">
                <label class="block text-xs font-semibold uppercase tracking-wider text-blue-400">Game Profile</label>
                {#if $settings.activeProfileId}
                    <span class="text-xs text-green-400 font-mono">ACTIVE</span>
                {/if}
            </div>
            
            <div class="flex gap-2 mb-3">
                <select 
                    bind:value={$settings.activeProfileId} 
                    on:change={loadProfile}
                    class="flex-1 bg-gray-800 border border-gray-600 rounded-lg px-3 py-2 text-sm focus:ring-2 focus:ring-blue-500 outline-none text-white"
                >
                    <option value={null}>Default / Custom</option>
                    {#each $settings.profiles as profile}
                        <option value={profile.id}>{profile.name}</option>
                    {/each}
                </select>
                {#if $settings.activeProfileId}
                    <button 
                        on:click={deleteProfile}
                        class="px-3 py-2 bg-red-900/50 hover:bg-red-900/80 text-red-200 rounded-lg border border-red-800 transition-colors text-sm"
                    >
                        Delete
                    </button>
                {/if}
            </div>

            <div class="flex gap-2">
                <input 
                    type="text" 
                    bind:value={newProfileName} 
                    placeholder="New Profile Name..." 
                    class="flex-1 bg-gray-800 border border-gray-600 rounded-lg px-3 py-2 text-sm focus:ring-2 focus:ring-blue-500 outline-none text-white placeholder-gray-500"
                    on:keydown={(e) => e.key === 'Enter' && createProfile()}
                />
                <button 
                    on:click={createProfile}
                    disabled={!newProfileName.trim()}
                    class="px-4 py-2 bg-blue-600 hover:bg-blue-500 disabled:opacity-50 disabled:cursor-not-allowed text-white rounded-lg font-medium transition-colors text-sm"
                >
                    Create
                </button>
            </div>
        </div>

        <!-- Device Selection -->
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <!-- Microphone -->
            <div class="space-y-2">
                <label class="block text-xs font-semibold uppercase tracking-wider text-gray-400">Microphone</label>
                <select 
                    bind:value={$settings.selectedMicId} 
                    on:change={handleDeviceChange}
                    class="w-full bg-gray-800 border border-gray-600 rounded-lg px-3 py-2 text-sm focus:ring-2 focus:ring-blue-500 outline-none text-white"
                >
                    {#each microphones as mic}
                        <option value={mic.deviceId}>{mic.label || `Microphone ${mic.deviceId.slice(0,5)}...`}</option>
                    {/each}
                </select>
            </div>

            <!-- Speaker -->
            <div class="space-y-2">
                <label class="block text-xs font-semibold uppercase tracking-wider text-gray-400">Speaker</label>
                <select 
                    bind:value={$settings.selectedSpeakerId} 
                    class="w-full bg-gray-800 border border-gray-600 rounded-lg px-3 py-2 text-sm focus:ring-2 focus:ring-blue-500 outline-none text-white"
                >
                    {#each speakers as speaker}
                        <option value={speaker.deviceId}>{speaker.label || `Speaker ${speaker.deviceId.slice(0,5)}...`}</option>
                    {/each}
                </select>
            </div>
        </div>

        <!-- Input Mode -->
        <div class="bg-gray-800/50 rounded-lg p-4 space-y-4 border border-gray-700/50">
            <label class="block text-xs font-semibold uppercase tracking-wider text-gray-400 mb-2">Input Mode</label>
            
            <div class="flex flex-col space-y-3">
                <div class="flex items-center gap-4">
                    <label class="flex items-center space-x-2 cursor-pointer">
                        <input type="radio" name="inputMode" checked={!$settings.isPTTEnabled} on:change={() => $settings.isPTTEnabled = false} class="text-blue-500 bg-gray-700 border-gray-600 focus:ring-blue-500/50">
                        <span class="text-sm">Voice Activity</span>
                    </label>
                    <label class="flex items-center space-x-2 cursor-pointer">
                        <input type="radio" name="inputMode" checked={$settings.isPTTEnabled} on:change={() => $settings.isPTTEnabled = true} class="text-blue-500 bg-gray-700 border-gray-600 focus:ring-blue-500/50">
                        <span class="text-sm">Push to Talk</span>
                    </label>
                </div>

                {#if $settings.isPTTEnabled}
                    <div class="flex items-center gap-4 bg-gray-900/50 p-3 rounded-lg border border-gray-700">
                        <span class="text-sm text-gray-300">Shortcut:</span>
                        <button 
                            on:click={startKeyListen}
                            class="px-4 py-1.5 bg-gray-700 hover:bg-gray-600 rounded text-sm font-mono border border-gray-600 transition-colors min-w-[100px] text-center"
                            class:animate-pulse={isListeningForKey}
                        >
                            {isListeningForKey ? 'Press Key...' : ($settings.pttKey || 'None')}
                        </button>
                    </div>
                {/if}
            </div>
        </div>

        <!-- Processing Features -->
        <div class="bg-gray-800/50 rounded-lg p-4 space-y-3 border border-gray-700/50">
            <label class="block text-xs font-semibold uppercase tracking-wider text-gray-400 mb-2">Enhancements</label>
            <div class="flex flex-wrap gap-4">
                <label class="flex items-center space-x-2 cursor-pointer select-none">
                    <input type="checkbox" bind:checked={$settings.noiseSuppression} on:change={handleConstraintChange} class="rounded bg-gray-700 border-gray-600 text-blue-500 focus:ring-blue-500/50">
                    <span class="text-sm">Noise Suppression</span>
                </label>
                <label class="flex items-center space-x-2 cursor-pointer select-none">
                    <input type="checkbox" bind:checked={$settings.echoCancellation} on:change={handleConstraintChange} class="rounded bg-gray-700 border-gray-600 text-blue-500 focus:ring-blue-500/50">
                    <span class="text-sm">Echo Cancellation</span>
                </label>
                <label class="flex items-center space-x-2 cursor-pointer select-none">
                    <input type="checkbox" bind:checked={$settings.autoGainControl} on:change={handleConstraintChange} class="rounded bg-gray-700 border-gray-600 text-blue-500 focus:ring-blue-500/50">
                    <span class="text-sm">Auto Gain</span>
                </label>
            </div>
        </div>

        <!-- Noise Gate & Gain -->
        <div class="space-y-6">
            <!-- Mic Gain -->
            <div class="space-y-2">
                <div class="flex justify-between items-center">
                    <label class="block text-xs font-semibold uppercase tracking-wider text-gray-400">Microphone Gain</label>
                    <span class="text-xs font-mono text-blue-400">{Math.round(($settings.micGain || 1.0) * 100)}%</span>
                </div>
                <input 
                    type="range" 
                    min="0" max="5" step="0.1" 
                    bind:value={$settings.micGain}
                    class="w-full h-2 bg-gray-700 rounded-lg appearance-none cursor-pointer accent-blue-500"
                >
            </div>

            <!-- Noise Gate -->
            <div class="space-y-2">
                <div class="flex justify-between items-center">
                    <label class="block text-xs font-semibold uppercase tracking-wider text-gray-400">Noise Gate Threshold</label>
                    <span class="text-xs font-mono text-blue-400">{$settings.noiseGateThreshold} dB</span>
                </div>
                
                <div class="relative h-24 bg-black rounded-lg overflow-hidden border border-gray-700 shadow-inner">
                    <canvas bind:this={canvas} width="600" height="100" class="w-full h-full block"></canvas>
                </div>
                
                <input 
                    type="range" 
                    min="-100" max="0" step="1" 
                    bind:value={$settings.noiseGateThreshold}
                    class="w-full h-2 bg-gray-700 rounded-lg appearance-none cursor-pointer accent-blue-500 mt-2"
                >
                <p class="text-xs text-gray-500">Audio below this level will be silenced.</p>
            </div>
        </div>

        <!-- Ducking (Smart Audio Reduction) -->
        <div class="bg-gray-800/50 rounded-lg p-4 space-y-4 border border-gray-700/50">
            <div class="flex items-center justify-between">
                <label class="block text-xs font-semibold uppercase tracking-wider text-gray-400">Smart Ducking</label>
                <label class="relative inline-flex items-center cursor-pointer">
                    <input type="checkbox" bind:checked={$settings.duckingEnabled} class="sr-only peer">
                    <div class="w-9 h-5 bg-gray-700 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-purple-600"></div>
                </label>
            </div>
            
            {#if $settings.duckingEnabled}
                <div class="space-y-4 pt-2">
                     <!-- Amount -->
                     <div class="space-y-1">
                        <div class="flex justify-between text-xs text-gray-400">
                            <span>Reduction Amount</span>
                            <span>{Math.round((1 - $settings.duckingAmount) * 100)}%</span>
                        </div>
                        <input type="range" min="0" max="1" step="0.05" bind:value={$settings.duckingAmount} class="w-full h-1 bg-gray-700 rounded-lg appearance-none cursor-pointer accent-purple-500">
                     </div>
                     
                     <!-- Release -->
                     <div class="space-y-1">
                        <div class="flex justify-between text-xs text-gray-400">
                            <span>Release Time</span>
                            <span>{$settings.duckingRelease}ms</span>
                        </div>
                        <input type="range" min="100" max="3000" step="100" bind:value={$settings.duckingRelease} class="w-full h-1 bg-gray-700 rounded-lg appearance-none cursor-pointer accent-purple-500">
                     </div>
                     <p class="text-xs text-gray-500">Lowers music volume when you speak.</p>
                </div>
            {/if}
        </div>

        <!-- Monitoring -->
        <div class="flex items-center justify-between bg-gray-800/50 p-4 rounded-lg border border-gray-700/50">
            <div class="flex items-center space-x-3">
                <div class="p-2 bg-gray-700 rounded-lg">
                    🎧
                </div>
                <div>
                    <h4 class="text-sm font-medium text-white">Test Audio</h4>
                    <p class="text-xs text-gray-400">Listen to your microphone input</p>
                </div>
            </div>
            <label class="relative inline-flex items-center cursor-pointer">
                <input type="checkbox" bind:checked={monitorAudio} class="sr-only peer">
                <div class="w-11 h-6 bg-gray-700 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-800 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-600"></div>
            </label>
        </div>
        
        <!-- Hidden Audio Element for Monitoring -->
        <audio bind:this={audioOutputElement} class="hidden"></audio>
    </div>
</div>