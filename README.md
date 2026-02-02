# VOID - Realtime Chat & Call

## Setup

1. **Firebase Setup**:
   - Create a Firebase Project.
   - Enable **Authentication** (Anonymous).
   - Enable **Firestore** (Test mode / Open rules).
   - Enable **Storage** (Test mode / Open rules).
   - Copy your Web App Config keys.
   - Paste them into `src/lib/firebase.ts`.

2. **Install Dependencies**:
   ```bash
   npm install
   ```

3. **Run Development**:
   ```bash
   npm run tauri dev
   ```

4. **Build for Windows**:
   ```bash
   npm run tauri build
   ```

## Testing

- **Instance 1**: Run the Tauri App.
- **Instance 2**: Open `http://localhost:1420` in Chrome/Edge.
- Login with different usernames.
- Copy UID from one to the other to add friend.
- Start Chat and Call.
