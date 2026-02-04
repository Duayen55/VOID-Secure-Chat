import { writable } from 'svelte/store';

export interface GameProfile {
  id: string;
  name: string;
  isPTTEnabled: boolean;
  noiseGateThreshold: number;
  micGain: number;
}

export interface Settings {
  isPTTEnabled: boolean;
  pttKey: string;
  ringVolume: number;
  shortcuts: {
    mute: string;
    deafen: string;
    end: string;
  };
  notifications: boolean;
  screenResolution: '720p' | '1080p' | '4k';
  screenBitrate: number;
  screenFramerate: number;
  shareSystemAudio: boolean;
  // Audio Settings
  selectedMicId?: string;
  selectedSpeakerId?: string;
  noiseSuppression: boolean;
  echoCancellation: boolean;
  autoGainControl: boolean;
  noiseGateThreshold: number;
  micGain: number;
  
  // Phase 2: Profiles & Ducking
  profiles: GameProfile[];
  activeProfileId: string | null;
  
  duckingEnabled: boolean;
  duckingAmount: number; // 0.0 to 1.0 (reduction factor)
  duckingRelease: number; // ms
}

const defaultSettings: Settings = {
  isPTTEnabled: false,
  pttKey: 'p',
  ringVolume: 0.5,
  shortcuts: {
    mute: 'm',
    deafen: 'd',
    end: 'escape'
  },
  notifications: true,
  screenResolution: '1080p',
  screenBitrate: 3000000, // 3 Mbps
  screenFramerate: 30,
  shareSystemAudio: false,
  // Audio Defaults
  noiseSuppression: true,
  echoCancellation: true,
  autoGainControl: true,
  noiseGateThreshold: -50,
  micGain: 1.0,
  
  profiles: [],
  activeProfileId: null,
  
  duckingEnabled: false,
  duckingAmount: 0.3, // Reduce to 30%
  duckingRelease: 1000
};

function createSettingsStore() {
  const { subscribe, set, update } = writable<Settings>(defaultSettings);

  return {
    subscribe,
    set,
    update,
    load: () => {
        if (typeof localStorage !== 'undefined') {
            const saved = localStorage.getItem('void_settings');
            if (saved) {
                try {
                    const parsed = JSON.parse(saved);
                    update(s => ({ ...s, ...parsed }));
                } catch (e) {
                    console.error("Failed to load settings", e);
                }
            }
        }
    },
    save: (s: Settings) => {
        if (typeof localStorage !== 'undefined') {
            localStorage.setItem('void_settings', JSON.stringify(s));
        }
    }
  };
}

export const settings = createSettingsStore();

export const isSpeaking = writable(false);
export const user = writable<any>(null);
export const activeChat = writable<any>(null);
export const pendingCallAccept = writable<string | null>(null); // Stores chatId to auto-accept on main page
