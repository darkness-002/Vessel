/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{html,js,svelte,ts}'],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        canvas: "var(--bg-canvas)",
        surface: "var(--bg-surface)",
        "surface-hover": "var(--bg-surface-hover)",
        "border-subtle": "var(--border-subtle)",
        "accent-primary": "var(--accent-primary)",
        "accent-fg": "var(--accent-fg)",
        "text-primary": "var(--text-primary)",
        "text-muted": "var(--text-muted)",
        "status-running": "var(--status-running)",
        "status-locked": "var(--status-locked)",
        "vessel-bg": "#0c0e12",
        "vessel-card": "#15181e",
        "vessel-card-hover": "#1c2028",
        "vessel-border": "#27272a",
        "vessel-border-subtle": "#1f232b",
        vessel: {
          bg: "#0c0e12",
          card: "#15181e",
          "card-hover": "#1c2028",
          border: "#27272a",
          "border-subtle": "#1f232b"
        }
      },
      fontFamily: {
        headline: ["Inter", "sans-serif"],
        body: ["Inter", "sans-serif"],
        label: ["Inter", "sans-serif"],
      }
    }
  },
  plugins: [],
}