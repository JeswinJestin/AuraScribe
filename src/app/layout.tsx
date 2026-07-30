import type { Metadata, Viewport } from 'next'
import { Inter } from 'next/font/google'
import './globals.css'

const inter = Inter({
  subsets: ['latin'],
  variable: '--font-inter',
  display: 'swap',
})

export const metadata: Metadata = {
  title: 'AuraScribe - Voice Input Layer for Everyone',
  description: 'Free, open-source, privacy-first voice dictation that works everywhere. Your voice, your data, your rules.',
  keywords: ['voice dictation', 'speech to text', 'whisper', 'open source', 'privacy', 'accessibility'],
  authors: [{ name: 'AuraScribe Contributors' }],
  creator: 'AuraScribe Contributors',
  publisher: 'AuraScribe',
  robots: 'index, follow',
  openGraph: {
    type: 'website',
    locale: 'en_US',
    url: 'https://aurascribe.dev',
    title: 'AuraScribe - Voice Input Layer for Everyone',
    description: 'Free, open-source, privacy-first voice dictation that works everywhere.',
    siteName: 'AuraScribe',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'AuraScribe',
    description: 'Free, open-source, privacy-first voice dictation that works everywhere.',
  },
  verification: {
    other: {
      'github': 'aurascribe/aurascribe',
    },
  },
}

export const viewport: Viewport = {
  themeColor: [
    { media: '(prefers-color-scheme: light)', color: '#ffffff' },
    { media: '(prefers-color-scheme: dark)', color: '#0f172a' },
  ],
  width: 'device-width',
  initialScale: 1,
  maximumScale: 5,
}

export default function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <html lang="en" suppressHydrationWarning className={inter.variable}>
      <head>
        <link rel="preconnect" href="https://fonts.googleapis.com" />
        <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      </head>
      <body className="min-h-screen bg-background font-sans antialiased">
        {children}
      </body>
    </html>
  )
}