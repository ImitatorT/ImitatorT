/** @type {import('tailwindcss').Config} */
export default {
  darkMode: 'class',
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // Slack-inspired colors
        slack: {
          purple: '#4A154B',
          blue: '#36C5F0',
          green: '#2EB67D',
          yellow: '#ECB22E',
          red: '#E01E5A',
        },
        // Telegram-inspired colors
        tg: {
          primary: '#0088cc',
          'primary-hover': '#0099e6',
          'primary-active': '#0077b3',
          bg: '#ffffff',
          'bg-secondary': '#f5f5f5',
          'bg-dark': '#17212b',
          'bg-dark-secondary': '#242f3d',
          message: {
            sent: '#effdde',
            'sent-dark': '#2b5278',
            received: '#ffffff',
            'received-dark': '#182533',
          }
        },
        // Chatscope customization
        chat: {
          primary: '#0088cc',
          secondary: '#6c757d',
          success: '#2eb67d',
          danger: '#e01e5a',
          warning: '#ecb22e',
          info: '#36c5f0',
        }
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
        mono: ['JetBrains Mono', 'monospace'],
      },
      animation: {
        'fade-in': 'fadeIn 0.2s ease-out',
        'slide-in': 'slideIn 0.3s ease-out',
        'pulse-slow': 'pulse 3s cubic-bezier(0.4, 0, 0.6, 1) infinite',
      },
      keyframes: {
        fadeIn: {
          '0%': { opacity: '0' },
          '100%': { opacity: '1' },
        },
        slideIn: {
          '0%': { transform: 'translateX(-10px)', opacity: '0' },
          '100%': { transform: 'translateX(0)', opacity: '1' },
        },
      },
    },
  },
  plugins: [],
}
