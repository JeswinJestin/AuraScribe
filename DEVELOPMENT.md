# Development Guide for AuraScribe

## 🛠️ Development Setup

### System Requirements
- **Node.js** (18+ with npm/pnpm/yarn)
- **Rust** (stable latest)
- **Git** for version control
- **Make** (optional, for build automation)
- **VS Code** (recommended)

### Installation Steps

```bash
# 1. Install Rust (if not already installed)
# Install Rust using rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Install Node.js (version 18+)
# Download from: https://nodejs.org/

# 3. Clone and set up project
git clone https://github.com/JeswinJestin/AuraScribe.git
cd AuraScribe

# 4. Install frontend dependencies
npm install

# 5. Install Rust dependencies (in separate terminal)
cd src-tauri
cargo install cargo-audit
cargo install cargo-expand
cargo install cargo-edit

# 6. Build and run
cd ..
npm run dev
```

## 🚀 Development Workflow

### Start Development Server

```bash
# In AuraScribe root directory
npm run dev
```

This will:
1. Start Next.js development server (http://localhost:3000)
2. Build Tauri backend with live reload
3. Open desktop application

### Build for Production

```bash
# Full build
npm run build

# Build only frontend (for testing)
npm run build:frontend
```

### Code Quality Checks

```bash
# TypeScript type checking
npm run typecheck

# Linting
npm run lint

# Run tests
npm run test
```

## 📦 Package Management

### Common NPM Commands

```bash
# Start development
npm run dev

# Build all packages
npm run build

# Clean build artifacts
npm run clean

# Format code
npm run format

# Check for vulnerabilities
cargo audit
```

## 🐛 Debugging

### Frontend Debugging

```javascript
// Add console.log everywhere
console.log('Debug message', data);

// Use React DevTools for component analysis
// Chrome: F12 → React tab
```

### Backend Debugging

```rust
// Rust Tauri commands
#[tauri::command]
async fn example_function(app: tauri::AppHandle) -> Result<String> {
    tracing::info!("Function called");
    // Your code here
    Ok("Hello".to_string())
}
```

### Toggle Debug Mode

Create `.env.local` for environment-specific settings:
```bash
DEBUG=true
ZUSE_DEVELOPMENT=true
```

## 🔧 Configuration

### Environment Variables

Create `.env.local` in root for frontend configuration:

```bash
# UI Settings
NEXT_PUBLIC_APP_NAME=AuraScribe
NEXT_PUBLIC_DEFAULT_THEME=dark
NEXT_PUBLIC_DEFAULT_MODEL=base.en

# Development vs Production
NODE_ENV=development
```

### Tauri Configuration

Edit `src-tauri/tauri.conf.json` to customize:

```json
{
  "bundle": {
    "active": true,
    "icon": ["icons/icon.png"],
    "identifier": "dev.aurascribe.aurascribe"
  },
  "security": {
    "csp": null,
    "dangerousRemoteDomainIpcAccess": []
  }
}
```

### Database Configuration

Database location: `~/.local/share/AuraScribe/aurascribe.db`

Connection string in `src-tauri/src/db.rs`:

```rust
let db_path = data_dir.join("aurascribe.db");
let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
```

## 🔍 Troubleshooting Common Issues

### Issue: Rust dependencies failing to install
```bash
# Reset Rust registry
rustup update

# Clean cargo cache
cargo clean

# Reinstall dependencies
cargo update

# Try fresh install
cargo install --verbose [package-name]
```

### Issue: TypeScript type errors
```bash
# Install missing types
npm install --save-dev @types/node @types/react @types/react-dom

# Clear build cache
npm run clean
npm install
```

### Issue: Build fails due to too large models
```bash
# Delete models directory
rm -rf ~/.local/share/AuraScribe/models/

# Re-download only base model
# (app will auto-download on next start)
```

### Issue: Tauri icons not showing
```bash
# Convert SVG to PNG
# Use: https://cloudconvert.com/svg-to-png

# Update tauri.conf.json paths
"icon": "icons/icon.png"
```

## 📊 Performance Optimization

### Frontend Optimization

```javascript
// Debounce expensive operations
const debounced = _.debounce(callback, 300);

// Lazy load heavy components
const Component = React.lazy(() => import('./HeavyComponent'));
```

### Backend Optimization

```rust
// Use Arc for shared state (already implemented)
let context: Arc<Mutex<SomeState>> = Arc::new(Mutex::new(init_state));

// Use async for I/O operations
async fn process_data() -> Result<()> { ... }
```

### Model Optimization

```rust
// Choose appropriate model size based on needs
// base.en (74MB) - Recommended for development
// small (244MB) - Better accuracy
```

## 🧪 Testing

### Unit Tests

```javascript
// In component files
it('should render correctly', () => {
  render(<Dashboard />);
  expect(screen.getByText('Test')).toBeInTheDocument();
});
```

### Feature Testing

```bash
# Run all tests
npm run test

# Run specific test file
npm run test test/some-test-file
```

### Manual Testing Checklist

- [ ] Model downloads correctly
- [ ] Transcription works in different apps
- [ ] AI cleanup processes text properly
- [ ] Settings persist correctly
- [ ] Hotkey works as expected
- [ ] Database encryption/decryption works
- [ ] Error handling displays properly

## 🤝 Code Style

### Frontend (TypeScript/React)

- Follow existing code style in project
- Use TypeScript strict mode
- Add JSDoc comments for complex functions
- Keep components functional and declarative

### Backend (Rust)

- Use `rustfmt` for formatting: `cargo fmt`
- Run clippy: `cargo clippy`
- Follow Rust API guidelines
- Add detailed docs in comments

## 📝 Git Workflow

### Branch Strategy

```bash
# Main development branch
main

# Feature branches
feature/[summary]
fix/[bug-description]
docs/[documentation-update]
test/[testing-addition]
```

### Commit Messages

```bash
# Good commit message
feat: add model caching for faster transcription
fix: resolve audio capture permission issue
docs: update setup instructions
```

## 🚀 Deployment Steps

1. **Clean build**: `npm run clean && npm run build`
2. **Run tests**: `npm run test`
3. **Run linter**: `npm run lint`
4. **Create dist**: Check `dist/` folder exists
5. **Package**: `npm run tauri build`
6. **Sign builds**: Configure certificate in `tauri.conf.json`
7. **Test**: Deploy to staging environment
8. **Release**: Create GitHub release with binaries

## 🔗 Useful Links

- **Tauri Docs**: https://tauri.app/v1/guides/
- **Whisper.cpp Issues**: https://github.com/openai/whisper.cpp/issues
- **Next.js Docs**: https://nextjs.org/docs
- **Rust Book**: https://doc.rust-lang.org/book/
- **SQLCipher**: https://www.zetetic.net/sqlcipher/

## 📧 Getting Help

Before submitting issues, please:
1. Search existing issues
2. Check troubleshooting section above
3. Provide error logs and reproduction steps
4. Include system information (OS, Node version, Rust version)

**Good issue template**:
```markdown
**System Info**
- OS: [e.g., Windows 11]
- Node: [e.g., 18.17.0]
- Rust: [e.g., stable (1.70.0)]

**Steps to Reproduce**
1. Do this...
2. Then do this...

**Expected Behavior**
Text should appear...

**Actual Behavior**
Error: [error message]

**Log Output**
[actual logs]
```