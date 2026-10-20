/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{html,js,svelte,ts}'],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
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