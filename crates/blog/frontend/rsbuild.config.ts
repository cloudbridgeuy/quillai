import { defineConfig } from '@rsbuild/core';
import { pluginReact } from '@rsbuild/plugin-react';

const config = defineConfig({
  server: {
    proxy: {
      context: ["/api"],
      target: 'http://localhost:5150',
      changeOrigin: true,
    }
  },
  plugins: [pluginReact()],
});

export default config;
