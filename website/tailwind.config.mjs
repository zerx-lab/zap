/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{astro,html,js,jsx,ts,tsx,md,mdx}'],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        ink: {
          950: '#06070b',
          900: '#0a0c12',
          800: '#11141c',
          700: '#1a1e29',
          600: '#262b3a',
          500: '#3a4055',
        },
        brand: {
          50:  '#e6f4ff',
          100: '#bae0ff',
          200: '#91caff',
          300: '#69b1ff',
          400: '#4096ff',
          500: '#1677ff',
          600: '#0958d9',
          700: '#003eb3',
          800: '#002c8c',
          900: '#001d66',
        },
        accent: {
          pink: '#ff5fa2',
          cyan: '#46e0ff',
          violet: '#1677ff',
          lime:  '#a8ff5f',
        },
      },
      fontFamily: {
        sans: ['"Inter"', '"PingFang SC"', '"Hiragino Sans GB"', '"Microsoft YaHei"', 'system-ui', 'sans-serif'],
        mono: ['"JetBrains Mono"', '"Fira Code"', '"SF Mono"', 'Consolas', 'monospace'],
        display: ['"Inter"', '"PingFang SC"', 'system-ui', 'sans-serif'],
      },
      backgroundImage: {
        'gradient-hero':
          'radial-gradient(55% 45% at 50% -10%, rgba(22,119,255,0.14) 0%, rgba(6,7,11,0) 70%)',
        'gradient-brand':
          'linear-gradient(135deg, #1677ff 0%, #0958d9 100%)',
        'grid-faint':
          'linear-gradient(rgba(255,255,255,.04) 1px, transparent 1px), linear-gradient(90deg, rgba(255,255,255,.04) 1px, transparent 1px)',
      },
      boxShadow: {
        glow: '0 0 0 1px rgba(22,119,255,.18), 0 14px 40px -12px rgba(22,119,255,.30)',
        card: '0 1px 0 rgba(255,255,255,.04) inset, 0 30px 60px -30px rgba(0,0,0,.6)',
      },
      animation: {
        'pulse-slow': 'pulse 4s cubic-bezier(0.4, 0, 0.6, 1) infinite',
        'float': 'float 8s ease-in-out infinite',
        'float-slow': 'float 14s ease-in-out infinite',
        'orbit': 'orbit 22s linear infinite',
        'marquee': 'marquee 40s linear infinite',
        'shine': 'shine 6s ease-in-out infinite',
        'spin-slow': 'spin 30s linear infinite',
      },
      keyframes: {
        float: {
          '0%,100%': { transform: 'translate3d(0,0,0)' },
          '50%': { transform: 'translate3d(0,-24px,0)' },
        },
        orbit: {
          '0%':   { transform: 'translate3d(0,0,0)' },
          '25%':  { transform: 'translate3d(30px,-20px,0)' },
          '50%':  { transform: 'translate3d(-10px,-40px,0)' },
          '75%':  { transform: 'translate3d(-30px,-10px,0)' },
          '100%': { transform: 'translate3d(0,0,0)' },
        },
        marquee: {
          '0%':   { transform: 'translateX(0)' },
          '100%': { transform: 'translateX(-50%)' },
        },
        shine: {
          '0%,100%': { backgroundPosition: '0% 50%' },
          '50%':     { backgroundPosition: '100% 50%' },
        },
      },
    },
  },
  plugins: [],
};
