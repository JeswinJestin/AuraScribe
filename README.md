# AuraScribe

Free, open-source, privacy-first voice dictation that works everywhere. Based on Whisper.cpp with AI-powered text cleanup.

![AuraScribe](https://img.shields.io/badge/version-1.0.0-blue)
![License](https://img.shields.io/badge/license-MIT-green)
![Platform](https://img.shields.io/badge/platform-Windows%20|%20macOS%20|%20Linux-lightgrey)

## ✨ Features

- 🎤 **Real-time Transcription**: Stream speech-to-text with Whisper.cpp (39MB - smallest model)
- 🤖 **AI Text Cleanup**: Automated punctuation, grammar fixing, and filler removal with OpenRouter
- 🔒 **Privacy First**: Everything runs locally. No data leaves your device
- ⚡ **Zero Latency**: Uses Silero VAD for responsive voice detection
- 🎨 **Customizable**: Create app profiles, snippets, and custom keyboard shortcuts
- 💾 **Auto-save**: All settings and transcripts stored in encrypted SQLite database

## 🚀 Quick Start

### Prerequisites

- **Node.js** (18+ version)
- **Rust** (stable, with Cargo)
- **npm** or **yarn** for package management

### Installation

```bash
# Clone the repository
git clone https://github.com/JeswinJestin/AuraScribe.git
cd AuraScribe

# Install dependencies
npm install

# Build and run the application
npm run build
```

### First Run

1. **Download Whisper Model**: The app will download the base model (~74MB) automatically on first use
2. **Configure Hotkey**: Press `Ctrl+Space` (default) to start dictation
3. **Enable AI Cleanup** (Optional): Get a free API key from [OpenRouter](https://openrouter.ai/keys) for enhanced text cleanup

## 🎯 Usage

### Basic Dictation
1. Open any text editor/IDE/browser
2. Press your hotkey (Ctrl+Space) and hold
3. Speak naturally
4. Release the hotkey to insert your text

### Advanced Features

**AI Text Cleanup**
```bash
npm run build
```

Or run dev mode:
```bash
npm run dev
```

### First Setup
1. Open preferences (Settings tab)
2. Select your preferred Whisper model:
   - `base.en` (74MB) - Recommended for English
   - `small` (244MB) - Better accuracy, bilingual
3. Enable AI cleanup for automated text enhancement

## 📦 Build Setup

```bash
# Build for production
npm run build

# Build only frontend
npm run build:frontend

# Test
npm run test
```

## 🏗️ Architecture

AuraScribe uses a modern tech stack:

- **Frontend**: Next.js 14 + React 18 + TypeScript
- **Backend**: Tauri 2 + Rust
- **Core Technologies**: Whisper.cpp, Silero VAD, OpenRouter API

### Project Structure

```
aurascribe/
├── src-tauri/           # Rust backend
│   ├── src/
│   │   ├── asr.rs       # Whisper.cpp integration
│   │   ├── vad.rs       # Voice Activity Detection
│   │   ├── audio.rs     # Audio capture
│   │   ├── injection.rs # Text injection into apps
│   │   ├── db.rs        # SQLite database
│   │   └── commands.rs  # Tauri commands
├── src/app/             # Next.js frontend
│   ├── app/             # App router (page.tsx, layout.tsx)
│   └── lib/             # Shared utilities
└── README.md
```

## 🎨 Models

### Whisper.cpp Models

Each model can be downloaded once and used offline:

| Model | Size | Language | Speed | Quality | Status |
|-------|------|----------|-------|---------|--------|
| `tiny.en` | 39MB | English | Fastest | Good | ✅ Recommended |
| `base.en` | 74MB | English | Fast | Better | ✅ Default |
| `small` | 244MB | Bilingual | Balanced | Great | 🌟 Advanced |
| `medium` | 769MB | Multilingual | Slow | Excellent | 🔥 Pro |

**Storage**: Models are downloaded to `~/.local/share/AuraScribe/models/` (or equivalent)

### OpenRouter Free Models

For AI text cleanup, use these free models:
- **Nemotron 3 Ultra** (4K context)
- **Llama 3.1 8B** (8K context)
- **Gemma 2 9B** (8K context)

All models have free tiers perfect for personal use.

## 🔧 Configuration

### Environment Variables

Create `.env.local` for frontend configuration:

```bash
# UI Configuration
NEXT_PUBLIC_APP_NAME=AuraScribe
NEXT_PUBLIC_DEFAULT_THEME=dark
```

### Tauri Configuration

Edit `src-tauri/tauri.conf.json` to customize:

```json
{
  "bundle": {
    "identifier": "dev.aurascribe.aurascribe",
    "resources": []
  },
  "security": {
    "csp": null
  }
}
```

## 📝 API Integration

### OpenRouter Setup

1. Get free API key at [OpenRouter](https://openrouter.ai/keys)
2. Store securely in app settings (encrypted locally)

### Rate Limiting

OpenRouter free models have generous limits (~10RPM). For production use:
- Cache generated text locally
- Batch multiple words for cleaner results
- Use native Whisper transcription first

## 🐛 Troubleshooting

**Issue: Model won't download**
```bash
# Manually download model from HuggingFace:
# https://huggingface.co/ggerganov/whisper.cpp/tree/main

# Then copy to: 
# ~/.local/share/AuraScribe/models/
```

**Issue: Audio not working**
1. Check system microphone permissions
2. Verify audio device selected in system settings
3. Ensure both `audio` and `sys-uio` permissions granted

**Issue: Text injection not working**
1. Ensure accessibility permissions granted (Windows)
2. Check app is in window focus
3. Try different injection method (keyboard vs pasteboard)

## 🤝 Contributing

We welcome contributions! Here's how to get started:

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/amazing-new-feature`
3. Make your changes
4. Run tests: `npm test`
5. Build: `npm run build`
6. Commit: `git commit -am 'Add amazing-new-feature'`
7. Push: `git push origin feature/amazing-new-feature`
8. Open a Pull Request

### Development Guidelines

- Use TypeScript strict mode
- Follow existing code style
- Add tests for new features
- Update documentation where needed

## 📄 License

This project is licensed under the MIT License - see [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [Whisper.cpp](https://github.com/openai/whisper.cpp) - High-performance Whisper implementation
- [Silero VAD](https://github.com/snakers4/silero-vad) - Voice Activity Detection algorithm
- [Tauri](https://tauri.app/) - Secure desktop framework
- [Next.js](https://nextjs.org/) - React framework
- [OpenRouter](https://openrouter.ai/) - Access to open AI models

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/JeswinJestin/AuraScribe/issues)
- **Discussions**: [GitHub Discussions](https://github.com/JeswinJestin/AuraScribe/discussions)
- **Email**: [jeswinjestin@example.com](mailto:jeswinjestin@example.com)

## 🌟 Star History

If you find this project useful, please consider giving it a star ⭐

**Made with ❤️ by the open-source community**