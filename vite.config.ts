import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// Tauri 需要固定端口，且不能清屏（会吃掉 cargo 的输出）
export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      // `src-tauri` 是 Rust 侧（由 cargo 管，改它不该触发前端整页刷新）。
      //
      // `.cdp-*` 是 CDP 验证脚本给 headless Edge 用的 Chrome profile 目录。
      // **必须 ignore 掉**：Edge 运行期间会在里面持续写 Cookies / Network /
      // Extension State 等文件，被监听的话每一次写都会触发一次 page reload ——
      // 页面在挂载途中被反复重载，`#root` 永远空着、`readyState` 停在
      // `interactive`，看起来像「前端崩了」，而 `/` 和 `/src/main.tsx` 直连都是
      // 200。项目根目录残留的 `.cdp-f16` / `.cdp-gsw` 就是这么来的。
      //
      // 两道保险：脚本侧的 profile 路径已改到系统临时目录（见 MEMORY.md），
      // 这里再兜一层，防止以后有人又把 profile 建在项目里。
      ignored: ["**/src-tauri/**", "**/.cdp-*/**", ".cdp-*/**"],
    },
  },
  build: {
    // Tauri 用的 WebView2 基于较新的 Chromium，可以放心用现代语法
    target: "chrome110",
    minify: "esbuild",
    sourcemap: false,
  },
});
