# 🚀 AuraScribe Setup & Installation Guide

Get AuraScribe running in **less than 10 minutes** using this quick setup guide.

## 📋 First Things You Need

Before starting, make sure you have:

✅ **Window, Mac, or Linux**  
✅ **Node.js 18 or higher** ([Download](https://nodejs.org/))  
✅ **Rust installed** ([Install Rust](https://rustup.rs/))  
✅ **npm or yarn** (comes with Node.js)  
✅ **Git** ([Install Git](https://git-scm.com/))

---

## ⚡ Quick Start (Recommended for First Use)

### Step 1: Install Rust (if needed)

```bash
# Use this command line (copy and paste)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

When prompted:
1. Press `1` for default installation
2. Press `Enter`
3. Close and reopen your terminal after installation

### Step 2: Run AuraScribe Locally

```bash
# Navigate to your project
cd C:\Users\jeswi\OneDrive\Desktop\Projects\AuraScribe

# Install dependencies
npm install

# Start development mode (runs both frontend and backend)
npm run dev
```

**What happens next:**
1. Browser opens with the AuraScribe UI
2. Desktop app launches
3. Whisper model will be downloaded automatically (~74MB)
4. First setup dialog appears

---

## 📦 Installation Options

### Option 1: Run in Development Mode (Best for Testing)

```bash
npm run dev
```

**Pros:** Fast startup, live code reloading  
**Cons:** Slower than production builds

### Option 2: Build for Production

```bash
# Full production build
npm run build

# This creates:
# - dist/ folder (front-end compiled files)
# - src-tauri/target/release/ (optimized Rust binaries)
```

**Pros:** Best performance, reduced file size  
**Cons:** Manual installation required

### Option 3: Download Pre-built App (Easy)

Coming soon! We'll publish release builds to GitHub releases.

---

## 🎯 First-Time Setup

When first running AuraScribe, you'll need to:

### 1. Choose a Whisper Model

Select one based on your needs:

| Model | Size | What it's best for |
|-------|------|-------------------|
| **base.en** (default) | 74MB | General use, English |
| **small** | 244MB | Better accuracy |
| **tiny** | 39MB | Fastest, but less accurate |

**Recommendation:** Use `base.en` - it's the best balance of speed and accuracy.

### 2. Configure Hotkey

Press `Ctrl+Space` (default):
- **Toggle Mode**: Tap once to start, tap again to stop
- **Hold Mode**: Hold to speak, release to insert text

### 3. Enable AI Cleanup (Optional)

For enhanced text:
1. Get free API key at [openrouter.ai/keys](https://openrouter.ai/keys)
2. Paste key in Settings → AI Cleanup
3. Choose a model (Nemotron 3 Ultra is free)

**Privacy Note:** AI cleanup only processes text, never raw audio. Data stays local.

---

## 🔧 Common Issues & Fixes

### Issue: "rustc: command not found"

**Solution:**
1. Close and reopen your terminal
2. Restart your computer
3. Verify with: `rustc --version`

### Issue: npm install hangs

**Solution:**
```bash
# Clear npm cache
npm cache clean --force

# Reinstall
npm install
```

### Issue: Whisper model won't download

**Manual Download:**
```bash
# Download from HuggingFace
# https://huggingface.co/ggerganov/whisper.cpp/tree/main

# Copy to:
# Windows: C:\Users\JESWI\AppData\Local\AuraScribe\models\
# Mac: ~/Library/Application Support/AuraScribe/models/
# Linux: ~/.local/share/AuraScribe/models/

# Filename should be: ggml-base.en.bin
```

### Issue: Audio not working

**Windows:**
1. Open Settings → Privacy →Microphone → Allow apps to access
2. Enable AuraScribe
3. Set device preference in System Tray

**Mac:**
1. System Settings → Privacy & Security →Microphone
2. Allow Terminal or your IDE
3. Check System Settings → Sound → Input

### Issue: Text injection not working

**Windows:**
1. Settings → Privacy →Accessibility
2. Enable "Show overlay window"
3. Allow "Graphite" or similar accessibility tools

**Mac:**
1. System Settings → Privacy & Security →Accessibility
2. Allow Terminal or your IDE

---

## 🧪 Testing Core Features

### Basic Dictation Test
1. Open Notepad (Windows) or TextEdit (Mac)
2. Press and hold `Ctrl+Space`
3. Speak: "Hello world, this is AuraScribe working"
4. Release the key
5. Your text should appear!

### AI Cleanup Test
1. Enable AI cleanup in settings
2. Set: `Remove fillers`, `Auto punctuation`
3. Press hotkey and speak: "um actually you know like i think we should do this"
4. Result (processed): "I think we should do this."

---

## 📊 File Locations

### User Data
```
Windows:
C:\Users\[YourName]\AppData\Local\AuraScribe\
  ├── aurascribe.db (encrypted database)
  ├── settings.json (app settings)
  └── models/
      └── ggml-[model].bin (Whisper models)
```

### Application Data
```
macOS:
~/.local/share/AuraScribe/

Linux:
~/.local/share/AuraScribe/

Windows (User-Folder):
%LOCALAPPDATA%\AuraScribe\
```

---

## 🎓 Learning Curve

### Beginner (5-10 mins)
- Run the app
- Try basic dictation
- Change hotkey

### Intermediate (15-20 mins)
- Experiment with Whisper models
- Enable AI cleanup
- Create app profiles

### Advanced (30-60 mins)
- Understand Whisper.cpp internals
- Customize injection methods
- Develop custom AI prompts

---

## 📞 Getting Help

### Documentation
- **README.md** - Overview and features
- **DEVELOPMENT.md** - For developers
- **OpenAI Whisper README** - Model details

### Community
- **GitHub Issues** - Report bugs
- **GitHub Discussions** - Ask questions

### Debug Mode
Add `.env.local` to enable detailed logging:
```bash
DEBUG=true
TAURI_LOG=trace
```

---

## 🔄 Updating

```bash
git pull
cd src-tauri
cargo update
npm run dev
```

---

## 💾 Backup Your Data

Your data is in:
```
[Data Directory]/aurascribe.db
```

Safe backup locations:
- External hard drive
- Cloud storage (encrypted)
- Git repository

---

## ✨ Tips for Best Experience

1. **Use a good microphone** - Better speech clarity = better transcription
2. **Speak clearly but naturally** - Emoji表情 works too!
3. **Enable AI cleanup** - Cleaner text without effort
4. **Choose the right model** - `base.en` for most users
5. **Test in focused apps** - VS Code, Chrome, Slack

---

## 🎯 Next Steps

After getting it working:

1. ✅ **Export your settings** - Save your preferences
2. ✅ **Try different apps** - See it work everywhere
3. ✅ **Experiment with models** - Find your favorite
4. ✅ **Enable AI cleanup** - Transform sloppy speech
5. ✅ **Share with friends** - Help spread privacy-first dictation

---

## 🌟 Success Criteria Checklist

You're ready when you can:

- [ ] Run the app without errors
- [ ] Download a Whisper model
- [ ] Dictate text into any app
- [ ] Insert text with hotkey
- [ ] See basic transcription
- [ ] Change Whisper model size
- [ ] Enable AI cleanup
- [ ] Adjust hotkey settings
- [ ] Access settings panel
- [ ] See system tray icon

🎉 **Congratulations! AuraScribe is working!**

---

Last updated: 2024 | Version: 1.0.0
Built with ❤️ by open-source community