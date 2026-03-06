/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // Backgrounds
        'bg-primary': '#0f0f0f',
        'bg-secondary': '#1a1a1a',
        'bg-tertiary': '#252525',
        'bg-hover': '#2d2d2d',

        // Text
        'text-primary': '#ffffff',
        'text-secondary': '#a0a0a0',
        'text-muted': '#666666',

        // Accent
        'accent-primary': '#6366f1',
        'accent-secondary': '#8b5cf6',
        'accent-success': '#22c55e',
        'accent-warning': '#f59e0b',
        'accent-error': '#ef4444',

        // Borders
        'border-color': '#333333',
        'border-focus': '#6366f1',
      },
      fontFamily: {
        sans: ['Inter', '-apple-system', 'BlinkMacSystemFont', 'sans-serif'],
      },
    },
  },
  plugins: [],
}
