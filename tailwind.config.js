/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{html,js,svelte,ts}'],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        "outline-variant": "#464554", "primary-fixed-dim": "#c2c1ff", "error-container": "#93000a", 
        "primary-container": "#5856d6", "error": "#ffb4ab", "on-secondary-fixed": "#161544", 
        "on-secondary-container": "#b1b0e7", "tertiary-fixed-dim": "#ffb785", "surface-variant": "#353535", 
        "inverse-surface": "#e2e2e2", "on-error-container": "#ffdad6", "surface-container-high": "#2a2a2a", 
        "on-secondary-fixed-variant": "#424272", "secondary-fixed": "#e2dfff", "surface": "#131313", 
        "surface-container-highest": "#353535", "inverse-primary": "#4f4ccd", "primary": "#c2c1ff", 
        "surface-container-lowest": "#0e0e0e", "secondary-fixed-dim": "#c3c1fa", "on-surface": "#e2e2e2", 
        "outline": "#918f9f", "surface-bright": "#393939", "secondary": "#c3c1fa", "on-error": "#690005", 
        "surface-dim": "#131313", "background": "#131313", "on-tertiary": "#502500", "on-background": "#e2e2e2", 
        "on-tertiary-container": "#ffe1cf", "on-primary-fixed": "#0c006a", "on-primary": "#1c0b9f", 
        "surface-container-low": "#1b1b1b", "primary-fixed": "#e2dfff", "surface-container": "#1f1f1f", 
        "on-primary-container": "#e7e4ff", "on-secondary": "#2b2b5a", "tertiary": "#ffb785", 
        "tertiary-container": "#a25100", "secondary-container": "#424272", "inverse-on-surface": "#303030", 
        "on-tertiary-fixed": "#301400", "surface-tint": "#c2c1ff", "on-primary-fixed-variant": "#3631b4", 
        "tertiary-fixed": "#ffdcc6", "on-tertiary-fixed-variant": "#713700", "on-surface-variant": "#c7c4d6"
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