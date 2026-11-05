import tailwindcss from "@tailwindcss/vite";
import type { StorybookConfig } from "@storybook/react-vite";

const config: StorybookConfig = {
  stories: ["../../app/src/ui/**/*.stories.tsx", "../*.stories.tsx"],
  addons: ["@storybook/addon-docs"],
  framework: "@storybook/react-vite",
  core: { disableTelemetry: true },
  viteFinal: (vite) => ({
    ...vite,
    plugins: [...(vite.plugins ?? []), tailwindcss()],
    resolve: { ...vite.resolve, dedupe: ["react", "react-dom"] },
    server: { ...vite.server, fs: { ...vite.server?.fs, allow: [...(vite.server?.fs?.allow ?? []), ".", "../app/src"] } },
  }),
};

export default config;
