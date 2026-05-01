/** @type {import('tailwindcss').Config} */
export default {
  content: ["./src/**/*.{astro,html,js,jsx,ts,tsx,md,mdx}"],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        ink: {
          950: "#05050a",
          900: "#090a12",
          800: "#10111d",
          700: "#191b2a",
          600: "#25283a",
          500: "#3b4057",
        },
        brand: {
          50: "#f3f1ff",
          100: "#e4dfff",
          200: "#ccc2ff",
          300: "#a999ff",
          400: "#8d7dff",
          500: "#725cff",
          600: "#5b43e6",
          700: "#4934bd",
          800: "#352885",
          900: "#241d59",
        },
        accent: {
          pink: "#ff5fa2",
          cyan: "#46e0ff",
          violet: "#8d7dff",
          lime: "#b9ff5a",
        },
      },
      fontFamily: {
        sans: [
          '"Inter"',
          '"PingFang SC"',
          '"Hiragino Sans GB"',
          '"Microsoft YaHei"',
          "system-ui",
          "sans-serif",
        ],
        mono: [
          '"JetBrains Mono"',
          '"Fira Code"',
          '"SF Mono"',
          "Consolas",
          "monospace",
        ],
        display: ['"Inter"', '"PingFang SC"', "system-ui", "sans-serif"],
      },
      backgroundImage: {
        "gradient-hero":
          "radial-gradient(70% 55% at 50% -15%, rgba(141,125,255,0.28) 0%, rgba(5,5,10,0) 68%), radial-gradient(45% 45% at 85% 12%, rgba(70,224,255,0.13) 0%, rgba(5,5,10,0) 70%), radial-gradient(45% 45% at 10% 24%, rgba(255,95,162,0.13) 0%, rgba(5,5,10,0) 72%)",
        "gradient-brand":
          "linear-gradient(135deg, #8d7dff 0%, #ff5fa2 48%, #46e0ff 100%)",
        "grid-faint":
          "linear-gradient(rgba(255,255,255,.055) 1px, transparent 1px), linear-gradient(90deg, rgba(255,255,255,.055) 1px, transparent 1px)",
      },
      boxShadow: {
        glow: "0 0 0 1px rgba(141,125,255,.24), 0 24px 80px -28px rgba(141,125,255,.55)",
        card: "0 1px 0 rgba(255,255,255,.06) inset, 0 36px 90px -44px rgba(0,0,0,.85)",
      },
      animation: {
        "pulse-slow": "pulse 4s cubic-bezier(0.4, 0, 0.6, 1) infinite",
        float: "float 8s ease-in-out infinite",
        "float-slow": "float 14s ease-in-out infinite",
        orbit: "orbit 22s linear infinite",
        marquee: "marquee 40s linear infinite",
        shine: "shine 6s ease-in-out infinite",
        "spin-slow": "spin 30s linear infinite",
      },
      keyframes: {
        float: {
          "0%,100%": { transform: "translate3d(0,0,0)" },
          "50%": { transform: "translate3d(0,-24px,0)" },
        },
        orbit: {
          "0%": { transform: "translate3d(0,0,0)" },
          "25%": { transform: "translate3d(30px,-20px,0)" },
          "50%": { transform: "translate3d(-10px,-40px,0)" },
          "75%": { transform: "translate3d(-30px,-10px,0)" },
          "100%": { transform: "translate3d(0,0,0)" },
        },
        marquee: {
          "0%": { transform: "translateX(0)" },
          "100%": { transform: "translateX(-50%)" },
        },
        shine: {
          "0%,100%": { backgroundPosition: "0% 50%" },
          "50%": { backgroundPosition: "100% 50%" },
        },
      },
    },
  },
  plugins: [],
};
