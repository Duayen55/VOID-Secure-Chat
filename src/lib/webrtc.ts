import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

// Sinyal veri tiplerini tanımlayalım (Kodun daha sağlam olması için)
interface SignalData {
    type: 'offer' | 'answer' | 'candidate';
    sdp?: RTCSessionDescriptionInit;
    candidate?: RTCIceCandidateInit;
}

let pc: RTCPeerConnection | null = null;
let localStream: MediaStream | null = null;

// --- EKRAN PAYLAŞIMI BAŞLATMA (GÖNDEREN) ---
export async function startScreenShare(targetPeerId: string) {
    console.log("Starting Screen Share to", targetPeerId);
    try {
        // TypeScript hatasını düzeltmek için 'as any' kullanıyoruz
        // Çünkü standart tip tanımlarında 'cursor' özelliği bazen eksik olabiliyor.
        const displayMediaOptions = {
            video: { cursor: "always" } as any, 
            audio: false
        };

        localStream = await navigator.mediaDevices.getDisplayMedia(displayMediaOptions);

        pc = new RTCPeerConnection({
            iceServers: [
                { urls: "stun:stun.l.google.com:19302" }
            ]
        });

        // Ekran akışını bağlantıya ekle
        localStream.getTracks().forEach(track => {
            pc!.addTrack(track, localStream!);
            
            // Kullanıcı "Paylaşımı Durdur" dediğinde
            track.onended = () => {
                console.log("Screen share ended by user");
                stopScreenShare();
            };
        });

        // ICE Adaylarını Rust üzerinden karşıya yolla
        pc.onicecandidate = (event) => {
            if (event.candidate) {
                sendSignal(targetPeerId, { 
                    type: 'candidate', 
                    candidate: event.candidate.toJSON() 
                });
            }
        };

        // Teklif (Offer) oluştur ve gönder
        const offer = await pc.createOffer();
        await pc.setLocalDescription(offer);
        
        await sendSignal(targetPeerId, {
            type: 'offer',
            sdp: offer
        });
        
        return localStream;

    } catch (e) {
        console.error("Error starting screen share:", e);
        throw e;
    }
}

// --- GELEN SİNYALLERİ İŞLEME (ALICI & GÖNDEREN) ---
export async function handleIncomingSignal(peerId: string, signal: SignalData, onStream?: (s: MediaStream) => void) {
    console.log("Handling Signal form:", peerId, "Type:", signal.type);

    // Eğer PeerConnection yoksa ve gelen sinyal 'offer' ise yeni bağlantı oluştur (ALICI TARAFI)
    if (!pc) {
        if (signal.type !== 'offer') return; // Offer olmadan candidate gelirse yoksay

        pc = new RTCPeerConnection({
            iceServers: [
                { urls: "stun:stun.l.google.com:19302" }
            ]
        });
        
        // Gelen video akışını yakala
        pc.ontrack = (event) => {
            console.log("Track received!", event.streams[0]);
            if (onStream && event.streams[0]) {
                onStream(event.streams[0]);
            }
        };

        pc.onicecandidate = (event) => {
            if (event.candidate) {
                sendSignal(peerId, { 
                    type: 'candidate', 
                    candidate: event.candidate.toJSON() 
                });
            }
        };
    }

    try {
        if (signal.type === 'offer' && signal.sdp) {
            await pc.setRemoteDescription(new RTCSessionDescription(signal.sdp));
            const answer = await pc.createAnswer();
            await pc.setLocalDescription(answer);
            
            await sendSignal(peerId, {
                type: 'answer',
                sdp: answer
            });

        } else if (signal.type === 'answer' && signal.sdp) {
            await pc.setRemoteDescription(new RTCSessionDescription(signal.sdp));

        } else if (signal.type === 'candidate' && signal.candidate) {
            await pc.addIceCandidate(new RTCIceCandidate(signal.candidate));
        }
    } catch (e) {
        console.error("WebRTC Handling Error:", e);
    }
}

// --- DİNLEME VE ABONELİK ---
export async function subscribeToSignals(targetPeerId: string, onStream: (s: MediaStream) => void): Promise<UnlistenFn> {
    return await listen('signal-event', (event: any) => {
        const payload = event.payload;
        // payload yapısının Rust tarafındaki struct ile eşleştiğinden emin ol:
        // { peer_id: String, payload: String }
        
        const senderId = payload.peerId || payload.peer_id; // Rust isimlendirmesine dikkat
        const msgString = payload.payload;

        // Sadece hedeflediğimiz kişiden gelen mesajları işle
        if (senderId !== targetPeerId) return;

        try {
            const signal: SignalData = JSON.parse(msgString);
            handleIncomingSignal(senderId, signal, onStream);
        } catch (e) {
            console.error("Failed to parse signal:", e);
        }
    });
}

// --- YARDIMCI FONKSİYONLAR ---

// Paylaşımı temiz bir şekilde durdurmak için
export function stopScreenShare() {
    if (localStream) {
        localStream.getTracks().forEach(track => track.stop());
        localStream = null;
    }
    if (pc) {
        pc.close();
        pc = null;
    }
}

// Tauri üzerinden sinyal gönderme yardımcısı
async function sendSignal(peerId: string, data: SignalData) {
    try {
        await invoke('send_signal', { 
            peerId: peerId, 
            payload: JSON.stringify(data) 
        });
    } catch (e) {
        console.error("Signal Send Error:", e);
    }
}