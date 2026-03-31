/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./src/**/*.rs", "./assets/index.html"],
  theme: {
    extend: {
      colors: {
        tmux: {
          bg: "#4e9a06",
          fg: "#0a0a0a",
          dim: "rgba(10, 10, 10, 0.55)",
          inv: {
            bg: "#0a0a0a",
            fg: "#8ae234",
          },
        },
      },
    },
  },
  plugins: [],
};
