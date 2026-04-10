/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        surface: {
          0: "#0a0a0f",
          1: "#12121a",
          2: "#1a1a25",
          3: "#22222f",
          4: "#2a2a38",
        },
        accent: {
          DEFAULT: "#6366f1",
          hover: "#818cf8",
          dim: "#4f46e5",
        },
        success: "#22c55e",
        danger: "#ef4444",
        warning: "#f59e0b",
        muted: "#64748b",
      },
    },
  },
  plugins: [],
};
