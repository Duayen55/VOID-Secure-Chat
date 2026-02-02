import { initializeApp } from "firebase/app";
import { getAuth, signInAnonymously, updateProfile, onAuthStateChanged } from "firebase/auth";
import { getFirestore, collection, addDoc, doc, onSnapshot, updateDoc, setDoc, getDoc, arrayUnion, deleteField, deleteDoc, query, where, serverTimestamp } from "firebase/firestore";

// REPLACE WITH YOUR FIREBASE CONFIG
const firebaseConfig = {
  apiKey: "AIzaSyBZTVnCE-6qZUdlHe5_mbOfSTbpGE9WF4A",
  authDomain: "dbsdv-e4742.firebaseapp.com",
  projectId: "dbsdv-e4742",
  storageBucket: "dbsdv-e4742.firebasestorage.app",
  messagingSenderId: "334997740059",
  appId: "1:334997740059:web:7e251354bf78c44ffef6be"
};

const app = initializeApp(firebaseConfig);
export const auth = getAuth(app);
export const db = getFirestore(app);

export { signInAnonymously, updateProfile, onAuthStateChanged, collection, addDoc, doc, onSnapshot, updateDoc, setDoc, getDoc, arrayUnion, deleteField, deleteDoc, query, where, serverTimestamp };
