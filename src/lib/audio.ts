import { get } from 'svelte/store';
import { settings, isSpeaking as isSpeakingStore } from './stores';

// --- Ringtone Logic ---
let ringToneContext: AudioContext | null = null;
let ringInterval: any = null;

export function playRingtone() {
  if (ringInterval) return; 
  
  if (!ringToneContext) {
    ringToneContext = new (window.AudioContext || (window as any).webkitAudioContext)();
  }
  if (ringToneContext.state === 'suspended') ringToneContext.resume();
  
  const playBeep = () => {
    if (!ringToneContext) return;
    const osc = ringToneContext.createOscillator();
    const gain = ringToneContext.createGain();
    
    osc.connect(gain);
    gain.connect(ringToneContext.destination);
    
    osc.type = 'sine';
    osc.frequency.setValueAtTime(440, ringToneContext.currentTime); 
    osc.frequency.exponentialRampToValueAtTime(880, ringToneContext.currentTime + 0.1);
    
    const vol = get(settings).ringVolume;
    
    gain.gain.setValueAtTime(0, ringToneContext.currentTime);
    gain.gain.linearRampToValueAtTime(vol, ringToneContext.currentTime + 0.1);
    gain.gain.exponentialRampToValueAtTime(0.01, ringToneContext.currentTime + 0.5);
    
    osc.start(ringToneContext.currentTime);
    osc.stop(ringToneContext.currentTime + 0.5);
  };

  playBeep();
  ringInterval = setInterval(playBeep, 2000); 
}

export function stopRingtone() {
  if (ringInterval) {
    clearInterval(ringInterval);
    ringInterval = null;
  }
  if (ringToneContext) {
    ringToneContext.close();
    ringToneContext = null;
  }
}

// --- Audio Engine (Core Processing) ---

export class AudioEngine {
    private context: AudioContext;
    private source: MediaStreamAudioSourceNode | null = null;
    private destination: MediaStreamAudioDestinationNode;
    private gainNode: GainNode;
    private analyser: AnalyserNode;
    private stream: MediaStream | null = null;
    private currentConstraints: MediaTrackConstraints | boolean = {
        echoCancellation: true,
        noiseSuppression: true,
        autoGainControl: true
    };
    
    // State
    private isPTT: boolean = false;
    private pttActive: boolean = false;
    private gateThreshold: number = -50;
    private isMuted: boolean = false;
    private micGain: number = 1.0;
    private wasSpeaking: boolean = false;

    // Visualizer Callback
    private levelListeners: Set<(db: number, isGated: boolean) => void> = new Set();
    // Ducking Callback
    private speakingListeners: Set<(isSpeaking: boolean) => void> = new Set();
    
    // Legacy support (setter)
    set onLevelChange(cb: ((db: number, isGated: boolean) => void) | null) {
        if (cb) this.levelListeners.add(cb);
    }
    set onSpeakingChange(cb: ((isSpeaking: boolean) => void) | null) {
        if (cb) this.speakingListeners.add(cb);
    }

    public addLevelListener(cb: (db: number, isGated: boolean) => void) {
        this.levelListeners.add(cb);
        return () => this.levelListeners.delete(cb);
    }

    public addSpeakingListener(cb: (isSpeaking: boolean) => void) {
        this.speakingListeners.add(cb);
        return () => this.speakingListeners.delete(cb);
    }

    private lastSignalTime: number = Date.now();
    private watchdogTimer: any = null;

    public startWatchdog() {
        if (this.watchdogTimer) clearInterval(this.watchdogTimer);
        this.lastSignalTime = Date.now();
        console.log("[AudioEngine] Watchdog started");

        this.watchdogTimer = setInterval(async () => {
            if (this.isMuted) {
                this.lastSignalTime = Date.now(); // Reset timer while muted
                return;
            }

            const silenceDuration = Date.now() - this.lastSignalTime;
            
            if (silenceDuration > 5000) {
                 console.warn("[AudioEngine] Watchdog: CRITICAL SILENCE detected (5s). Rolling back.");
                 await this.rollback();
                 this.lastSignalTime = Date.now();
            } else if (silenceDuration > 3000) {
                 console.warn("[AudioEngine] Watchdog: Silence detected (3s). Restarting.");
                 const success = await this.restart(this.currentConstraints);
                 if (success) this.lastSignalTime = Date.now();
            }
        }, 1000);
    }

    public stopWatchdog() {
        if (this.watchdogTimer) {
            clearInterval(this.watchdogTimer);
            this.watchdogTimer = null;
            console.log("[AudioEngine] Watchdog stopped");
        }
    }

    constructor() {
        this.context = new (window.AudioContext || (window as any).webkitAudioContext)();
        this.destination = this.context.createMediaStreamDestination();
        
        this.gainNode = this.context.createGain();
        this.analyser = this.context.createAnalyser();
        
        this.analyser.fftSize = 256;
        this.analyser.smoothingTimeConstant = 0.3;

        // Chain: Source -> Analyser -> Gain -> Destination
        // We connect Source later
        this.analyser.connect(this.gainNode);
        this.gainNode.connect(this.destination);
        
        this.startAnalysisLoop();
    }

    async setInput(audioConstraints: MediaTrackConstraints | boolean) {
        this.currentConstraints = audioConstraints;

        if (this.stream) {
            this.stream.getTracks().forEach(t => t.stop());
        }
        
        if (this.source) {
            this.source.disconnect();
        }

        this.stream = await navigator.mediaDevices.getUserMedia({ audio: audioConstraints });
        this.source = this.context.createMediaStreamSource(this.stream);
        this.source.connect(this.analyser);
        
        // Ensure context is running
        if (this.context.state === 'suspended') {
            await this.context.resume();
        }

        this.startWatchdog();

        return this.destination.stream;
    }

    stopInput() {
        this.stopWatchdog();
        if (this.stream) {
            this.stream.getTracks().forEach(t => t.stop());
            this.stream = null;
        }
        if (this.source) {
            this.source.disconnect();
            this.source = null;
        }
    }

    getProcessedStream() {
        return this.destination.stream;
    }
    
    getRawStream() {
        return this.stream;
    }

    async restart(newConstraints: MediaTrackConstraints | boolean): Promise<boolean> {
        console.log("[AudioEngine] Restarting/Updating with:", newConstraints);
        const previousMute = this.isMuted;
        
        // Try to apply constraints first without stopping stream
        if (this.stream && this.stream.active && typeof newConstraints === 'object') {
            const track = this.stream.getAudioTracks()[0];
            if (track) {
                try {
                    // Check if deviceId changed - if so, we MUST restart
                    const currentSettings = track.getSettings();
                    const newDeviceId = (newConstraints as any).deviceId;
                    
                    if (!newDeviceId || (newDeviceId.exact === currentSettings.deviceId) || (newDeviceId === currentSettings.deviceId)) {
                         console.log("[AudioEngine] Applying constraints to existing track...");
                         await track.applyConstraints(newConstraints);
                         this.currentConstraints = newConstraints;
                         return true;
                    }
                } catch (e) {
                    console.warn("[AudioEngine] applyConstraints failed, falling back to full restart", e);
                }
            }
        }

        this.setMute(true); // Temp mute

        try {
            // Stop existing input
            if (this.stream) {
                this.stream.getTracks().forEach(t => t.stop());
            }
            if (this.source) {
                this.source.disconnect();
            }

            // Get new stream
            const stream = await navigator.mediaDevices.getUserMedia({ audio: newConstraints });
            
            // Connect
            const source = this.context.createMediaStreamSource(stream);
            source.connect(this.analyser);
            
            // Resume context if needed
            if (this.context.state === 'suspended') await this.context.resume();

            // Validate Signal (2 seconds)
            const isValid = await this.validateSignal(2000);
            
            if (isValid) {
                console.log("[AudioEngine] Restart success");
                this.stream = stream;
                this.source = source;
                this.currentConstraints = newConstraints;
                this.setMute(previousMute); 
                return true;
            } else {
                console.warn("[AudioEngine] No signal detected, rolling back...");
                stream.getTracks().forEach(t => t.stop());
                throw new Error("No signal detected");
            }

        } catch (err) {
            console.error("[AudioEngine] Restart failed:", err);
            await this.rollback();
            this.setMute(previousMute);
            return false;
        }
    }

    private async rollback() {
        try {
            console.log("[AudioEngine] Rolling back to:", this.currentConstraints);
            if (this.stream) {
                this.stream.getTracks().forEach(t => t.stop());
            }
            this.stream = await navigator.mediaDevices.getUserMedia({ audio: this.currentConstraints });
            this.source = this.context.createMediaStreamSource(this.stream);
            this.source.connect(this.analyser);
        } catch (e) {
            console.error("[AudioEngine] Rollback CRITICAL FAILURE", e);
        }
    }

    private validateSignal(durationMs: number): Promise<boolean> {
        return new Promise(resolve => {
            const start = Date.now();
            const check = () => {
                if (Date.now() - start > durationMs) {
                    resolve(false); 
                    return;
                }
                
                if (!this.analyser) { resolve(false); return; }

                const bufferLength = this.analyser.frequencyBinCount;
                const data = new Uint8Array(bufferLength);
                this.analyser.getByteTimeDomainData(data);
                
                let hasSignal = false;
                for(let i=0; i<bufferLength; i++) {
                    if (Math.abs(data[i] - 128) > 2) { 
                        hasSignal = true;
                        break;
                    }
                }
                
                if (hasSignal) {
                    resolve(true);
                } else {
                    requestAnimationFrame(check);
                }
            };
            check();
        });
    }

    isHealthy(): boolean {
        return this.context.state === 'running' && !!this.stream && this.stream.active;
    }

    async updateSettings(s: any) {
        this.isPTT = s.isPTTEnabled;
        this.gateThreshold = s.noiseGateThreshold;
        this.micGain = s.micGain !== undefined ? s.micGain : 1.0;
        
        // Check for constraint changes
        const newConstraints = {
            deviceId: s.selectedMicId ? { exact: s.selectedMicId } : undefined,
            noiseSuppression: s.noiseSuppression,
            echoCancellation: s.echoCancellation,
            autoGainControl: s.autoGainControl
        };

        // Deep compare (simple version)
        const current = this.currentConstraints as any;
        const changed = 
            (newConstraints.noiseSuppression !== current.noiseSuppression) ||
            (newConstraints.echoCancellation !== current.echoCancellation) ||
            (newConstraints.autoGainControl !== current.autoGainControl) ||
            (JSON.stringify(newConstraints.deviceId) !== JSON.stringify(current.deviceId));

        if (changed) {
             console.log("[AudioEngine] Constraints changed, restarting...", newConstraints);
             await this.restart(newConstraints);
        } else {
             // Re-apply mute logic immediately if no restart
             this.updateGain();
        }
    }

    setPTTActive(active: boolean) {
        this.pttActive = active;
        this.updateGain();
    }

    setMute(muted: boolean) {
        this.isMuted = muted;
        this.updateGain();
    }

    private updateGain() {
        const now = this.context.currentTime;
        
        if (this.isMuted) {
            this.gainNode.gain.setTargetAtTime(0, now, 0.05);
            return;
        }

        if (this.isPTT) {
            // PTT Mode: Open only if key is held
            const target = this.pttActive ? this.micGain : 0;
            this.gainNode.gain.setTargetAtTime(target, now, 0.05);
        } else {
            // VAD / Noise Gate Mode (Handled in loop, but we reset here just in case)
        }
    }

    private startAnalysisLoop() {
        const update = () => {
            if (!this.analyser || !this.gainNode) return;
            
            const bufferLength = this.analyser.frequencyBinCount;
            const dataArray = new Uint8Array(bufferLength);
            this.analyser.getByteTimeDomainData(dataArray);

            // RMS
            let sum = 0;
            for(let i = 0; i < bufferLength; i++) {
                const x = (dataArray[i] - 128) / 128.0;
                sum += x * x;
            }
            const rms = Math.sqrt(sum / bufferLength);
            const db = 20 * Math.log10(rms + 0.00001);

            // Watchdog Signal Check (Threshold ~ -60dB)
            if (rms > 0.001) {
                this.lastSignalTime = Date.now();
            }

            let isGated = false;

            if (!this.isMuted) {
                if (this.isPTT) {
                    // PTT Control is manual (in updateGain)
                    isGated = !this.pttActive;
                } else {
                    // Noise Gate / VAD Logic
                    if (db < this.gateThreshold) {
                        // Close
                        this.gainNode.gain.setTargetAtTime(0, this.context.currentTime, 0.1);
                        isGated = true;
                    } else {
                        // Open
                        this.gainNode.gain.setTargetAtTime(this.micGain, this.context.currentTime, 0.05);
                        isGated = false;
                    }
                }
            } else {
                isGated = true;
            }

            // Ducking Trigger
            const isSpeaking = !isGated && !this.isMuted;
            if (isSpeaking !== this.wasSpeaking) {
                this.wasSpeaking = isSpeaking;
                this.speakingListeners.forEach(cb => cb(isSpeaking));
                isSpeakingStore.set(isSpeaking);
            }

            this.levelListeners.forEach(cb => cb(db, isGated));
            
            requestAnimationFrame(update);
        };
        requestAnimationFrame(update);
    }

    cleanup() {
        this.stopWatchdog();
        if (this.stream) this.stream.getTracks().forEach(t => t.stop());
        this.context.close();
    }
}

// Singleton instance
export const audioEngine = new AudioEngine();

// Hook for WebRTC integration
export function attachStreamToPeer(peerConnection: RTCPeerConnection, stream: MediaStream) {
    stream.getTracks().forEach(track => {
        const sender = peerConnection.getSenders().find(s => s.track?.kind === track.kind);
        if (sender) {
            sender.replaceTrack(track);
        } else {
            peerConnection.addTrack(track, stream);
        }
    });
}
