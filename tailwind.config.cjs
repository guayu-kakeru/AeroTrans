/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./index.html", "./src/**/*.{vue,ts}"],
  theme: {
    extend: {
      colors: {
        ink: "#0f172a",
        card: "#ffffff",
        accent: "#0ea5a5",
        warm: "#f59e0b"
      },
      boxShadow: {
        float: "0 24px 60px rgba(15, 23, 42, 0.18)"
      }
    }
  },
  plugins: []
};
