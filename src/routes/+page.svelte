<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { auth, db, signInAnonymously, updateProfile, doc, setDoc, getDoc, onSnapshot, collection, addDoc, updateDoc, arrayUnion, deleteField, query, where, deleteDoc } from '$lib/firebase';
  import { onAuthStateChanged } from 'firebase/auth';
  import { user, activeChat, settings } from '$lib/stores';
  import { playRingtone, stopRingtone } from '$lib/audio';
  import { replaceVideoTrack } from '$lib/webrtc';
  import Chat from '$lib/components/Chat.svelte';
  import VideoCall from '$lib/components/VideoCall.svelte';
  import ScreenShare from '$lib/components/ScreenShare.svelte';
  import Settings from '$lib/components/Settings.svelte';
  import MusicPlayer from '$lib/components/MusicPlayer.svelte';

  import DraggablePanel from '$lib/components/DraggablePanel.svelte';

  // --- State ---
  let showMusicPlayer = false;
  let username = "";
  let view: 'login' | 'main' | 'chat' = 'login';
  let myUid = "";
  let friendUidInput = "";
  let friends: any[] = [];
  $: filteredFriends = friendUidInput 
     ? friends.filter(f => (f.displayName || "").toLowerCase().includes(friendUidInput.toLowerCase()) || f.uid.includes(friendUidInput))
     : friends;
  
  // Settings
  let showSettings = false;

  // Web Audio Ringtone
  // (Moved to $lib/audio)

  let messages: any[] = [];
  
  // WebRTC & Call State
  let localStream: MediaStream | null = null;
  let remoteStream: MediaStream | null = null;
  let peerConnection: RTCPeerConnection | null = null;
  let dataChannel: RTCDataChannel | null = null;
  let currentCallChat: any = null; // Stores the chat session for the active call
  
  // Call Status Model: idle -> ringing -> accepted/connected -> ended
  let callState: 'idle' | 'ringing' | 'connected' = 'idle'; 
  let incomingCall: { caller: string, callerName: string, chatId: string } | null = null;
  let isCaller = false; // Am I the one who started the call?
  
  // Audio/Video Controls
  let isMuted = false;
  let isDeafened = false;
  let isScreenSharing = false;

  // UI/UX State
  let isPTTActive = false;

  // File Transfer State
  let receivedBuffers: ArrayBuffer[] = [];
  let receivedSize = 0;
  let incomingFileMeta: any = null;
  let transferProgress = 0;

  const servers = {
    iceServers: [
      { urls: ['stun:stun.l.google.com:19302'] }
    ]
  };

  let signalingUnsub: (() => void) | null = null;
  let globalCallUnsub: (() => void) | null = null;
  let messagesUnsub: (() => void) | null = null;
  let globalMessageUnsub: (() => void) | null = null;

  onMount(() => {
    settings.load();
    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('keyup', handleKeyUp);
    requestNotificationPermission();

    onAuthStateChanged(auth, (u) => {
      if (u) {
        $user = u;
        myUid = u.uid;
        view = 'main';
        loadFriends();
        saveUserToDb();
        subscribeToGlobalIncomingCalls();
        subscribeToGlobalMessages();
      } else {
        view = 'login';
        if (globalCallUnsub) globalCallUnsub();
        if (globalMessageUnsub) globalMessageUnsub();
      }
    });
  });

  onDestroy(() => {
    window.removeEventListener('keydown', handleKeyDown);
    window.removeEventListener('keyup', handleKeyUp);
    stopRingtone();
    
    cleanupCall();
    if (signalingUnsub) signalingUnsub();
    if (globalCallUnsub) globalCallUnsub();
    if (messagesUnsub) messagesUnsub();
    if (globalMessageUnsub) globalMessageUnsub();
  });

  // --- Global Listener ---
  function requestNotificationPermission() {
      if ('Notification' in window && Notification.permission !== 'granted') {
          Notification.requestPermission();
      }
  }

  function subscribeToGlobalMessages() {
    if (globalMessageUnsub) globalMessageUnsub();
    
    const q = query(
      collection(db, "chats"), 
      where("participants", "array-contains", myUid)
    );
    
    const initTime = Date.now();

    globalMessageUnsub = onSnapshot(q, (snap) => {
      snap.docChanges().forEach((change) => {
        if (change.type === "modified" || change.type === "added") {
          const data = change.doc.data();
          if (data.lastMessageTimestamp > initTime && data.lastMessageSender !== myUid) {
              if ($activeChat?.chatId !== change.doc.id) {
                   if (Notification.permission === 'granted') {
                       new Notification("New Message", {
                           body: data.lastMessage || "You have a new message"
                       });
                   }
              }
          }
        }
      });
    });
  }

  function subscribeToGlobalIncomingCalls() {
    if (globalCallUnsub) globalCallUnsub();
    
    const q = query(
      collection(db, "chats"), 
      where("to", "==", myUid), 
      where("status", "==", "ringing")
    );

    globalCallUnsub = onSnapshot(q, (snap) => {
      snap.docChanges().forEach((change) => {
        if (change.type === "added" || change.type === "modified") {
          const data = change.doc.data();
          // Check if we are already in a call?
          if (callState === 'idle') {
             // New Incoming Call
             $activeChat = { 
               uid: data.from, 
               displayName: data.callerName || "Unknown", 
               chatId: change.doc.id 
             };
             incomingCall = { caller: data.from, callerName: data.callerName || "Unknown", chatId: change.doc.id };
             playRingtone();
             
             // Also subscribe to this specific chat signaling for candidates etc.
             subscribeToSignaling(change.doc.id);
          }
        }
        // Handle removals/cancellations
        if (change.type === "removed" && incomingCall && change.doc.id === $activeChat?.chatId) {
           stopRingtone();
           incomingCall = null;

           // Note: We keep signalingUnsub active if we are in a call, 
           // but ideally signaling should be tied to currentCallChat.
           // If callState is idle, we can clean up signaling.
           if (callState === 'idle' && !incomingCall) {
               if (signalingUnsub) {
                   signalingUnsub();
                   signalingUnsub = null;
               }
               cleanupCall();
           }
        }
      });
      
      // If no docs are ringing, ensure we stop ringtone
      if (snap.empty && incomingCall) {
         stopRingtone();
         incomingCall = null;
      }
    });
  }


  async function login() {
    if (!username) return;
    try {
      const cred = await signInAnonymously(auth);
      await updateProfile(cred.user, { displayName: username });
    } catch (e) {
      console.error(e);
      alert("Login failed: " + e);
    }
  }

  async function saveUserToDb() {
    if(!$user) return;
    try {
      await setDoc(doc(db, "users", $user.uid), {
        uid: $user.uid,
        displayName: $user.displayName || username
      }, { merge: true });
    } catch (e) {
      console.error("Error saving user:", e);
    }
  }

  async function addFriend() {
    if (!friendUidInput || friendUidInput === myUid) return;
    
    try {
      const friendDoc = await getDoc(doc(db, "users", friendUidInput));
      if (!friendDoc.exists()) {
        alert("User not found! Make sure the UID is correct.");
        return;
      }
      const friendData = friendDoc.data();
  
      // Add to my friends
      await setDoc(doc(db, "users", myUid, "friends", friendUidInput), friendData);
      
      // Add me to their friends (auto-accept)
      await setDoc(doc(db, "users", friendUidInput, "friends", myUid), {
        uid: myUid,
        displayName: $user.displayName
      });
  
      friendUidInput = "";
    } catch (e) {
      alert("Error adding friend: " + e);
    }
  }

  function loadFriends() {
    onSnapshot(collection(db, "users", myUid, "friends"), (snap) => {
      friends = snap.docs.map(d => d.data());
    });
  }

  function getChatId(uid1: string, uid2: string) {
    return uid1 < uid2 ? `${uid1}_${uid2}` : `${uid2}_${uid1}`;
  }

  async function openChat(friend: any) {
    $activeChat = { ...friend, chatId: getChatId(myUid, friend.uid) };
    view = 'chat';
    
    // Subscribe to messages
    if (messagesUnsub) messagesUnsub();
    messagesUnsub = onSnapshot(collection(db, "chats", $activeChat.chatId, "messages"), (snap) => {
      messages = snap.docs
        .map(d => d.data())
        .sort((a, b) => a.timestamp - b.timestamp);
    });

    // Subscribe to signaling
    if (signalingUnsub) signalingUnsub();
    subscribeToSignaling($activeChat.chatId);
  }

  function goBack() {
    // Only cleanup if we are NOT in a call, or if user manually ended it (handled by endCall)
    // Here we just want to close the chat view.
    
    view = 'main';
    $activeChat = null;
    
    // Stop listening to messages for the specific chat
    if (messagesUnsub) {
        messagesUnsub();
        messagesUnsub = null;
    }

    // If callState is idle AND no incoming call, we can clean up signaling.
    if (callState === 'idle' && !incomingCall) {
        if (signalingUnsub) {
            signalingUnsub();
            signalingUnsub = null;
        }
        cleanupCall();
    }
  }

  // --- Messaging & File Transfer ---

  async function sendMessage(event: CustomEvent<string>) {
    const text = event.detail;
    if (!text || !$activeChat) return;

    const chatDocRef = doc(db, "chats", $activeChat.chatId);

    // 1. Add message to subcollection
    await addDoc(collection(chatDocRef, "messages"), {
      text: text,
      sender: myUid,
      timestamp: Date.now()
    });

    // 2. Update parent doc for global listeners
    await setDoc(chatDocRef, {
        lastMessage: text,
        lastMessageSender: myUid,
        lastMessageTimestamp: Date.now(),
        participants: [myUid, $activeChat.uid]
    }, { merge: true });
  }

  function setupDataChannel(channel: RTCDataChannel) {
    dataChannel = channel;
    dataChannel.onopen = () => console.log("DataChannel Open");
    dataChannel.onmessage = handleDataChannelMessage;
  }

  function handleDataChannelMessage(event: MessageEvent) {
    const data = event.data;
    if (typeof data === 'string') {
      const msg = JSON.parse(data);
      if (msg.type === 'file-meta') {
        incomingFileMeta = msg;
        receivedBuffers = [];
        receivedSize = 0;
      } else if (msg.type === 'file-end') {
        const blob = new Blob(receivedBuffers, { type: incomingFileMeta.mime });
        const url = URL.createObjectURL(blob);
        messages = [...messages, {
          text: `Received file: ${incomingFileMeta.name}`,
          imageUrl: incomingFileMeta.mime.startsWith('image/') ? url : null,
          sender: 'system',
          timestamp: Date.now()
        }];
        incomingFileMeta = null;
        receivedBuffers = [];
      }
    } else {
      receivedBuffers.push(data);
      receivedSize += data.byteLength;
      if (incomingFileMeta) transferProgress = (receivedSize / incomingFileMeta.size) * 100;
    }
  }

  async function handleSendFile(event: CustomEvent<File>) {
    const file = event.detail;
    if (callState !== 'connected') {
        alert("Please CALL first to send files (P2P).");
        return;
    }

    if (!dataChannel || dataChannel.readyState !== 'open') {
      alert("P2P Connection not ready.");
      return;
    }
    dataChannel.send(JSON.stringify({ type: 'file-meta', name: file.name, size: file.size, mime: file.type }));
    
    const chunkSize = 16384;
    const reader = new FileReader();
    let offset = 0;

    reader.onload = (e) => {
      if (e.target?.result) {
        dataChannel?.send(e.target.result as ArrayBuffer);
        offset += chunkSize;
        if (offset < file.size) {
          readSlice(offset);
        } else {
          dataChannel?.send(JSON.stringify({ type: 'file-end' }));
          messages = [...messages, { text: `Sent file: ${file.name}`, sender: 'system', timestamp: Date.now() }];
        }
      }
    };
    const readSlice = (o: number) => {
      const slice = file.slice(o, o + chunkSize);
      reader.readAsArrayBuffer(slice);
    };
    readSlice(0);
  }

  // --- Call Logic & State Management ---

  // 1. Start Call (Caller)
  async function startCall(video: boolean = false, existingStream?: MediaStream) {
    if (!$activeChat) return;
    currentCallChat = $activeChat;
    isCaller = true;
    callState = 'ringing';
    
    try {
      if (existingStream) {
         localStream = existingStream;
      } else {
         localStream = await navigator.mediaDevices.getUserMedia({ audio: true, video: video });
      }
      isScreenSharing = video;
      
      peerConnection = new RTCPeerConnection(servers);
      
      // Add Tracks
      localStream.getTracks().forEach(track => peerConnection?.addTrack(track, localStream!));
      
      // Apply PTT Mute if enabled
      if ($settings.isPTTEnabled) {
        setMicState(false);
      }
      
      // Data Channel
      const dc = peerConnection.createDataChannel("files");
      setupDataChannel(dc);

      // Listeners
      peerConnection.onnegotiationneeded = async () => {
         console.log("Negotiation needed");
         if (!isCaller || callState !== 'connected') return;
         
         try {
             const offer = await peerConnection!.createOffer();
             await peerConnection!.setLocalDescription(offer);
             
             await updateDoc(doc(db, "chats", currentCallChat.chatId), {
                 offer: { type: offer.type, sdp: offer.sdp },
                 status: 'connected' // Ensure status stays connected, trigger listener update
             });
         } catch(e) {
             console.error("Renegotiation error:", e);
         }
      };

      peerConnection.ontrack = (event) => {
        remoteStream = event.streams[0];
      };

      peerConnection.onicecandidate = (event) => {
        if (event.candidate) {
          updateDoc(doc(db, "chats", currentCallChat.chatId), {
            offerCandidates: arrayUnion(event.candidate.toJSON())
          });
        }
      };

      // Create Offer
      const offer = await peerConnection.createOffer();
      await peerConnection.setLocalDescription(offer);

      // Signal: Ringing
      await setDoc(doc(db, "chats", currentCallChat.chatId), {
        status: 'ringing',
        from: myUid,
        to: currentCallChat.uid,
        participants: [myUid, currentCallChat.uid],
        callerName: $user.displayName || username,
        offer: { type: offer.type, sdp: offer.sdp },
        offerCandidates: [],
        answerCandidates: []
      }, { merge: true });

    } catch (e) {
      console.error("Start call error:", e);
      callState = 'idle';
      alert("Could not start call: " + e);
    }
  }

  // 2. Incoming Call Listener
  function subscribeToSignaling(chatId: string) {
    signalingUnsub = onSnapshot(doc(db, "chats", chatId), async (snap) => {
      const data = snap.data();
      if (!data) return;

      // Detect Call Status
      if (data.status === 'ringing') {
        if (data.to === myUid && callState === 'idle') {
          // Incoming Call!
          incomingCall = { caller: data.from, callerName: data.callerName || "Unknown", chatId: chatId }; 
        }
      } 
      else if (data.status === 'accepted') {
        // Call Accepted
        if (callState === 'ringing' && isCaller) {
          callState = 'connected';
          if (data.answer && !peerConnection?.currentRemoteDescription) {
             await peerConnection?.setRemoteDescription(new RTCSessionDescription(data.answer));
          }
        }
      }
      else if (data.status === 'connected') {
          // Check for renegotiation offer/answer
          if (peerConnection) {
              const remoteDesc = peerConnection.remoteDescription;
              // If we are callee (not caller) and we see a new offer
              if (!isCaller && data.offer && (!remoteDesc || remoteDesc.sdp !== data.offer.sdp)) {
                  console.log("Received renegotiation offer");
                  await peerConnection.setRemoteDescription(new RTCSessionDescription(data.offer));
                  const answer = await peerConnection.createAnswer();
                  await peerConnection.setLocalDescription(answer);
                  await updateDoc(doc(db, "chats", chatId), {
                      answer: { type: answer.type, sdp: answer.sdp }
                  });
              }
              // If we are caller and we see a new answer
              else if (isCaller && data.answer && (!remoteDesc || remoteDesc.sdp !== data.answer.sdp)) {
                   console.log("Received renegotiation answer");
                   // Ensure we haven't already set this answer (avoid error)
                   if (peerConnection.signalingState === "have-local-offer") {
                        await peerConnection.setRemoteDescription(new RTCSessionDescription(data.answer));
                   }
              }
          }
      }
      else if (data.status === 'rejected') {
        if (callState === 'ringing' || incomingCall) {
          alert("Call rejected.");
          cleanupCall();
        }
      }
      else if (data.status === 'ended') {
        if (callState !== 'idle') {
          cleanupCall();
        }
      }

      // Handle ICE Candidates
      if (peerConnection && callState !== 'idle') {
        if (isCaller && data.answerCandidates) {
           data.answerCandidates.forEach((c: any) => peerConnection?.addIceCandidate(new RTCIceCandidate(c)));
        } else if (!isCaller && data.offerCandidates) {
           data.offerCandidates.forEach((c: any) => peerConnection?.addIceCandidate(new RTCIceCandidate(c)));
        }
      }
    });
  }

  // 3. Accept Call (Callee)
  async function acceptCall() {
    if (!incomingCall || !$activeChat) return;
    currentCallChat = $activeChat;
    
    // Get Offer Data first
    const chatDoc = await getDoc(doc(db, "chats", currentCallChat.chatId));
    const data = chatDoc.data();
    if (!data || !data.offer) {
      alert("Call expired or invalid.");
      cleanupCall();
      return;
    }

    incomingCall = null;
    callState = 'connected';
    isCaller = false;
    stopRingtone();

    try {
      const constraints = {
          audio: {
              deviceId: $settings.selectedMicId ? { exact: $settings.selectedMicId } : undefined,
              noiseSuppression: $settings.noiseSuppression,
              echoCancellation: $settings.echoCancellation,
              autoGainControl: $settings.autoGainControl
          },
          video: false
      };
      localStream = await navigator.mediaDevices.getUserMedia(constraints);
      peerConnection = new RTCPeerConnection(servers);

      localStream.getTracks().forEach(track => peerConnection?.addTrack(track, localStream!));

      // Apply PTT Mute if enabled
      if ($settings.isPTTEnabled) {
        setMicState(false);
      }

      peerConnection.ontrack = (event) => {
        remoteStream = event.streams[0];
      };

      peerConnection.ondatachannel = (event) => setupDataChannel(event.channel);

      peerConnection.onicecandidate = (event) => {
        if (event.candidate) {
          updateDoc(doc(db, "chats", currentCallChat.chatId), {
            answerCandidates: arrayUnion(event.candidate.toJSON())
          });
        }
      };

      await peerConnection.setRemoteDescription(new RTCSessionDescription(data.offer));
      const answer = await peerConnection.createAnswer();
      await peerConnection.setLocalDescription(answer);

      // Signal: Accepted
      await updateDoc(doc(db, "chats", currentCallChat.chatId), {
        status: 'accepted',
        answer: { type: answer.type, sdp: answer.sdp }
      });

    } catch (e) {
      console.error("Accept error:", e);
      cleanupCall();
    }
  }

  // 4. Reject Call
  async function rejectCall() {
    if (!incomingCall && !$activeChat) return;
    const cid = incomingCall?.chatId || $activeChat?.chatId;
    if (!cid) return;

    incomingCall = null;
    await updateDoc(doc(db, "chats", cid), {
      status: 'rejected'
    });
    cleanupCall();
  }

  // 5. End Call
  async function endCall() {
    const chat = currentCallChat || $activeChat;
    if (!chat) return;
    
    try {
        await updateDoc(doc(db, "chats", chat.chatId), {
            status: 'ended'
        });
        
        setTimeout(async () => {
            try {
              await deleteDoc(doc(db, "chats", chat.chatId));
            } catch (e) { console.warn("Delete doc error", e); }
        }, 2000);

    } catch(e) { console.warn("Could not update status to ended", e); }

    cleanupCall();
  }

  // Helper: Cleanup
  function cleanupCall() {
    callState = 'idle';
    currentCallChat = null;
    incomingCall = null;
    isCaller = false;
    isMuted = false;
    isDeafened = false;
    isScreenSharing = false;
    isPTTActive = false;
    showMusicPlayer = false; // Auto-close music player
    stopRingtone();

    if (peerConnection) {
      peerConnection.close();
      peerConnection = null;
    }
    if (localStream) {
      localStream.getTracks().forEach(t => t.stop());
      localStream = null;
    }
    remoteStream = null;
  }

  // --- In-Call Controls ---

  function toggleMute() {
    if (localStream) {
      const audioTrack = localStream.getAudioTracks()[0];
      if (audioTrack) {
        audioTrack.enabled = !audioTrack.enabled;
        isMuted = !audioTrack.enabled;
      }
    }
  }

  function toggleDeaf() {
    isDeafened = !isDeafened;
    // Note: Actual deafen logic usually involves muting remote audio elements.
    // VideoCall component handles this visually, but we need to mute the remote stream?
    // VideoCall component has 'remoteAudioElement.volume = remoteVolume'.
    // If deafened, volume should be 0. VideoCall should handle this if we pass isDeafened, 
    // or we can handle it here by setting volume to 0. 
    // Let's rely on VideoCall component which takes isDeafened prop.
    // Wait, VideoCall receives isDeafened but does it mute audio?
    // In VideoCall: $: if (remoteAudioElement) remoteAudioElement.volume = remoteVolume;
    // It doesn't seem to use isDeafened for volume control in the code I read.
    // I should update VideoCall later to respect isDeafened.
  }

  async function switchCamera() {
    if (!localStream) return;
    
    const currentVideoTrack = localStream.getVideoTracks()[0];
    // If no video track, maybe we want to add one? For now just return.
    if (!currentVideoTrack) return;

    const currentFacingMode = currentVideoTrack.getSettings().facingMode;
    // Simple toggle: if user -> environment, else -> user
    const newFacingMode = currentFacingMode === 'user' ? 'environment' : 'user';

    try {
        const newStream = await navigator.mediaDevices.getUserMedia({
          video: { facingMode: newFacingMode }
        });
        const newVideoTrack = newStream.getVideoTracks()[0];

        // Replace track in PeerConnection
        if (peerConnection) {
          const sender = peerConnection.getSenders().find(s => s.track?.kind === 'video');
          if (sender) {
              await sender.replaceTrack(newVideoTrack);
          } else {
              peerConnection.addTrack(newVideoTrack, localStream);
          }
        }

        // Stop old track
        currentVideoTrack.stop();
        localStream.removeTrack(currentVideoTrack);
        localStream.addTrack(newVideoTrack);
        
        // Force reactivity
        localStream = localStream; 

    } catch (e) {
        console.error("Error switching camera:", e);
        alert("Could not switch camera: " + e);
    }
  }

  async function startScreenShare() {
     // Check for browser support
     if (!navigator.mediaDevices || !navigator.mediaDevices.getDisplayMedia) {
         alert("Screen sharing is not supported in this browser. Please use Chrome, Edge, or Firefox.");
         return;
     }

     // 1. Try with User Settings
     let constraints: any = {
        video: {
            frameRate: { ideal: $settings.screenFramerate, max: 60 }
        },
        audio: $settings.shareSystemAudio
     };

     // 2. Fallback logic
     let stream: MediaStream | null = null;

     try {
        try {
            stream = await navigator.mediaDevices.getDisplayMedia(constraints);
        } catch (err: any) {
            console.warn("Preferred screen share constraints failed, trying basic...", err);
            try {
                // Fallback 1: Basic video + requested audio
                stream = await navigator.mediaDevices.getDisplayMedia({
                    video: true,
                    audio: $settings.shareSystemAudio
                });
            } catch (err2) {
                console.warn("Basic constraints failed, trying video only...", err2);
                // Fallback 2: Video only (no audio)
                stream = await navigator.mediaDevices.getDisplayMedia({
                    video: true
                });
            }
        }

        if (!stream) throw new Error("No stream obtained");

        const videoTrack = stream.getVideoTracks()[0];

        videoTrack.onended = async () => {
              isScreenSharing = false;
              if (callState === 'idle') return;
              
              if (localStream) {
                  // 1. Remove Screen Video
                  localStream.removeTrack(videoTrack);
                  
                  // 2. Restore Mic if we shared system audio
                  if ($settings.shareSystemAudio) {
                      try {
                          const micStream = await navigator.mediaDevices.getUserMedia({ audio: true });
                          const micTrack = micStream.getAudioTracks()[0];
                          
                          const audioSender = peerConnection?.getSenders().find(s => s.track?.kind === 'audio');
                          if (audioSender) await audioSender.replaceTrack(micTrack);
                          
                          const oldAudio = localStream.getAudioTracks()[0];
                          if (oldAudio) {
                              oldAudio.stop();
                              localStream.removeTrack(oldAudio);
                          }
                          localStream.addTrack(micTrack);
                      } catch(e) { console.warn("Failed to restore mic:", e); }
                  }

                  localStream = localStream; // Trigger reactivity
              }
         };
 
         if (callState === 'idle') {
             // Start new call with this stream
             
             if (stream.getAudioTracks().length === 0) {
                  try {
                      const micStream = await navigator.mediaDevices.getUserMedia({ audio: true });
                      stream.addTrack(micStream.getAudioTracks()[0]);
                  } catch(e) { console.warn("No mic access", e); }
             }
             
             await startCall(true, stream); 
         } else {
             // Already in call: Replace video track
             if (peerConnection) {
                  // Ensure localStream is available
                  if (!localStream) {
                      console.warn("No localStream found, creating one from screen share stream");
                      localStream = stream;
                  }
                  
                  await replaceVideoTrack(videoTrack, peerConnection, localStream);
                  
                  // Handle Audio (Replace Mic with System Audio if present)
                  const newAudioTrack = stream.getAudioTracks()[0];
                  if (newAudioTrack) {
                      const audioSender = peerConnection.getSenders().find(s => s.track?.kind === 'audio');
                      if (audioSender) {
                          await audioSender.replaceTrack(newAudioTrack);
                          // Update local stream logic
                          const oldAudio = localStream?.getAudioTracks()[0];
                          if (oldAudio && localStream) {
                              oldAudio.stop();
                              localStream.removeTrack(oldAudio);
                              localStream.addTrack(newAudioTrack);
                          }
                      }
                  }
                  
                  // Update local view (Video)
                  if (localStream) {
                      const oldVideo = localStream.getVideoTracks()[0];
                      if (oldVideo) {
                          oldVideo.stop();
                          localStream.removeTrack(oldVideo);
                      }
                      localStream.addTrack(videoTrack);
                      localStream = localStream; 
                  }
                  isScreenSharing = true;
             }
         }

     } catch (e: any) {
         console.error("Screen share error:", e);
         if (e.name === 'NotSupportedError') {
             alert("Screen sharing is not supported in this browser/environment (e.g. IDE Preview). Please open the app in a full browser (Chrome/Edge/Firefox).");
         } else if (e.name !== 'NotAllowedError') {
             alert("Could not start screen share: " + e.message);
         }
     }
  }
  
  function setMicState(enabled: boolean) {
      if (localStream) {
          const track = localStream.getAudioTracks()[0];
          if(track) {
             track.enabled = enabled;
             isMuted = !enabled;
          }
      }
  }
  
  function togglePTT() {
      if ($settings.isPTTEnabled) {
          isPTTActive = !isPTTActive;
          setMicState(isPTTActive);
      }
  }

  // --- Accessibility & Helpers ---

  function handleKeyDown(e: KeyboardEvent) {
    if (e.repeat) return;
    const key = e.key.toLowerCase();
    
    // Shortcuts
    if (incomingCall && key === $settings.shortcuts.end) {
       rejectCall();
       return;
    }

    if (callState === 'connected' || (callState === 'ringing' && isCaller)) {
      if (key === $settings.shortcuts.mute) toggleMute();
      if (key === $settings.shortcuts.deafen) toggleDeaf();
      if (key === $settings.shortcuts.end) endCall();
      if (key === $settings.pttKey && $settings.isPTTEnabled) {
         isPTTActive = true;
         setMicState(true); // Unmute
      }
    }
  }

  function handleKeyUp(e: KeyboardEvent) {
      const key = e.key.toLowerCase();
      if (callState === 'connected' && key === $settings.pttKey && $settings.isPTTEnabled) {
          isPTTActive = false;
          setMicState(false); // Mute
      }
  }
</script>

<div class="h-screen w-screen bg-gray-900 text-white font-sans flex flex-col overflow-hidden relative">
  
  <!-- View: Login -->
  {#if view === 'login'}
    <div class="flex flex-col items-center justify-center h-full space-y-4">
      <h1 class="text-4xl font-bold text-blue-500 tracking-widest">VOID</h1>
      <div class="bg-gray-800 p-8 rounded-lg shadow-lg flex flex-col space-y-4 w-80">
        <input type="text" bind:value={username} placeholder="Username" class="p-3 rounded bg-gray-700 border border-gray-600 text-white outline-none focus:border-blue-500 transition" />
        <button on:click={login} class="bg-blue-600 p-3 rounded font-bold hover:bg-blue-500 transition transform hover:scale-105">ENTER VOID</button>
      </div>
    </div>
  {/if}

  <!-- View: Main -->
  {#if view === 'main'}
    <div class="flex flex-col h-full p-4 max-w-md mx-auto w-full">
      <div class="flex space-x-2 mb-6">
        <input type="text" bind:value={friendUidInput} placeholder="Search Friends or Enter UID" class="flex-1 p-3 rounded bg-gray-800 border border-gray-700 focus:border-blue-500 outline-none transition" />
        <button on:click={addFriend} class="bg-green-600 px-6 rounded font-bold hover:bg-green-500 transition shadow-lg">+ ADD</button>
      </div>

      <div class="flex justify-between items-center mb-6 border-b border-gray-800 pb-4">
        <div>
           <h2 class="text-xl font-bold text-white">{$user?.displayName}</h2>
           <div class="flex items-center space-x-2 mt-1">
             <span class="text-xs text-gray-500 bg-gray-800 px-2 py-1 rounded select-all">{myUid}</span>
             <span class="text-[10px] text-gray-600 uppercase">Your UID</span>
           </div>
        </div>
        <button on:click={() => showSettings = true} class="p-2 bg-gray-800 rounded-lg hover:bg-gray-700 transition text-gray-400 hover:text-white" title="Settings">
           ⚙️
        </button>
      </div>

      <h3 class="text-gray-500 uppercase text-xs font-bold mb-3 tracking-wider">Friends</h3>
      <div class="flex-1 overflow-y-auto space-y-2 pr-2 custom-scrollbar">
        {#each filteredFriends as friend}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div on:click={() => openChat(friend)} class="p-4 bg-gray-800/50 rounded-lg cursor-pointer hover:bg-gray-700/80 transition flex items-center border border-transparent hover:border-gray-600">
            <div class="w-3 h-3 bg-green-500 rounded-full mr-4 shadow-[0_0_8px_rgba(34,197,94,0.6)]"></div>
            <span class="font-medium text-lg">{friend.displayName}</span>
          </div>
        {/each}
        {#if friends.length === 0}
          <div class="text-gray-600 text-center mt-10 italic">No friends yet. Share your UID.</div>
        {/if}
      </div>
    </div>
  {/if}

  <!-- View: Chat -->
  {#if view === 'chat'}
    <div class="flex flex-col h-full relative">
      <!-- Header -->
      <div class="h-16 bg-gray-800 flex items-center justify-between px-4 shadow-md z-10 border-b border-gray-700">
        <button on:click={goBack} class="text-gray-400 hover:text-white transition flex items-center font-medium">
          <span class="mr-1">←</span> Back
        </button>
        <div class="flex flex-col items-center">
           <span class="font-bold text-lg">{$activeChat?.displayName}</span>
           {#if callState === 'connected'}
             <span class="text-[10px] text-green-400 font-mono tracking-wider animate-pulse">● SECURE CONNECTION</span>
           {/if}
        </div>
        
        <!-- Call Buttons (Only visible if idle) -->
        {#if callState === 'idle'}
          <div class="flex space-x-2">
             <button on:click={() => startCall(false)} class="p-2 bg-gray-700 rounded-full hover:bg-green-600 transition" title="Voice Call">📞</button>
             <button on:click={startScreenShare} class="p-2 bg-gray-700 rounded-full hover:bg-blue-600 transition" title="Screen Share">🖥️</button>
          </div>
        {:else}
           <div class="w-16"></div> <!-- Spacer -->
        {/if}
      </div>

      <!-- Chat Component -->
      <div class="flex-1 relative overflow-hidden">
         <Chat 
           {messages} 
           {transferProgress} 
           on:sendMessage={sendMessage} 
           on:sendFile={handleSendFile} 
         />
      </div>
    </div>
  {/if}

  <!-- SETTINGS MODAL -->
  <Settings bind:show={showSettings} />

  <!-- INCOMING CALL MODAL -->
  {#if incomingCall}
    <div class="fixed inset-0 bg-black/80 backdrop-blur-sm z-50 flex flex-col items-center justify-center animate-fade-in">
       <div class="bg-gray-800 p-8 rounded-2xl shadow-2xl flex flex-col items-center space-y-6 border border-gray-700 w-80">
          <div class="w-24 h-24 bg-gray-700 rounded-full flex items-center justify-center animate-pulse">
             <span class="text-4xl">📞</span>
          </div>
          <div class="text-center">
             <h3 class="text-2xl font-bold text-white">{incomingCall.callerName}</h3>
             <p class="text-gray-400">is calling...</p>
          </div>
          <div class="flex space-x-8 w-full justify-center">
             <button on:click={rejectCall} class="flex flex-col items-center space-y-1 group">
                <div class="w-14 h-14 bg-red-600 rounded-full flex items-center justify-center group-hover:bg-red-500 transition shadow-lg">
                   <span class="text-2xl">✖</span>
                </div>
                <span class="text-xs text-gray-400">Decline</span>
             </button>
             <button on:click={acceptCall} class="flex flex-col items-center space-y-1 group">
                <div class="w-14 h-14 bg-green-600 rounded-full flex items-center justify-center group-hover:bg-green-500 transition shadow-lg animate-bounce">
                   <span class="text-2xl">✔</span>
                </div>
                <span class="text-xs text-gray-400">Accept</span>
             </button>
          </div>
       </div>
    </div>
  {/if}

  <!-- ACTIVE CALL CONTROLS -->
  {#if callState === 'connected' || (callState === 'ringing' && isCaller)}
    <DraggablePanel 
        title={isCaller ? "Calling..." : "On Call"} 
        left={window.innerWidth - 700} 
        top={80} 
        width="680px" 
        height="520px"
        zIndex={60}
    >
        <VideoCall
            {callState}
            {isCaller}
            {remoteStream}
            {localStream}
            {isMuted}
            {isDeafened}
            {isPTTActive}
            on:togglePTT={togglePTT}
            on:toggleMute={toggleMute}
            on:toggleDeaf={toggleDeaf}
            on:endCall={endCall}
            on:startScreenShare={startScreenShare}
            on:switchCamera={switchCamera}
            on:toggleMusic={() => showMusicPlayer = !showMusicPlayer}
        />
    </DraggablePanel>
  {/if}
  
  <!-- Screen Share Preview (Floating) -->
  {#if isScreenSharing && localStream}
           <ScreenShare 
              stream={localStream} 
              label="You are sharing" 
              onClose={() => {
                  endCall();
              }} 
              {peerConnection}
              onChangeSource={() => startScreenShare()}
           />
        {/if}

  <!-- Background Music Player -->
  {#if $user && (showMusicPlayer || ((currentCallChat || $activeChat) && callState !== 'idle'))}
      <div class:hidden={!showMusicPlayer} class="absolute top-0 left-0 z-[70]">
          <MusicPlayer 
            friends={friends} 
            activeChat={currentCallChat || $activeChat} 
            on:close={() => showMusicPlayer = false}
          />
      </div>
  {/if}

</div>

<style>
  .custom-scrollbar::-webkit-scrollbar {
    width: 6px;
  }
  .custom-scrollbar::-webkit-scrollbar-track {
    background: transparent;
  }
  .custom-scrollbar::-webkit-scrollbar-thumb {
    background-color: #4b5563;
    border-radius: 20px;
  }
  
  @keyframes fade-in {
    from { opacity: 0; transform: translateY(10px); }
    to { opacity: 1; transform: translateY(0); }
  }
  .animate-fade-in {
    animation: fade-in 0.3s ease-out forwards;
  }
</style>