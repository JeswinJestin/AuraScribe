# public/ — static assets

Anything in this folder is served from the app's root path.

**Drop background images for the "Glass" appearance here.** For example, a file named
`glass-bg.jpg` in this folder is referenced in CSS/JS as `/glass-bg.jpg` (leading slash, no
`public/`).

Recommended for the Glass backdrop:
- A wide image (≥ 1920px) so it looks sharp on large screens.
- A calm, bluish image — it sits *behind* the frosted glass, so busy images fight the UI.
- `.jpg` or `.webp` keeps the installer small (these ship inside the app).

Once your image is here, tell me the filename and I'll wire it into the Glass appearance in
`src/app/globals.css` (the `.glass-bg body` rule), replacing the CSS gradient.
