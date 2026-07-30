/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  images: {
    unoptimized: true,
  },
  output: 'export',
  trailingSlash: true,
  // Tauri expects the build output in `out` directory
  distDir: 'out',
}

module.exports = nextConfig