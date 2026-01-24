import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import starlightThemeGalaxy from "starlight-theme-galaxy";
import starlightClientMermaid from "@pasqal-io/starlight-client-mermaid";

export default defineConfig({
  // site and base are set via CLI args in CI (from actions/configure-pages)
  integrations: [
    starlight({
      title: "Toss",
      description: "Cross-platform clipboard sharing with end-to-end encryption",
      plugins: [starlightThemeGalaxy(), starlightClientMermaid()],
      customCss: ["./src/styles/custom.css"],
      social: [
        { icon: "github", label: "GitHub", href: "https://github.com/rennerdo30/toss-share" },
      ],
      sidebar: [
        { label: "Home", slug: "index" },
        {
          label: "Getting Started",
          items: [
            { label: "Quick Start", slug: "getting-started/quick-start" },
            { label: "Installation", slug: "getting-started/installation" },
            { label: "Development Setup", slug: "getting-started/development-setup" },
          ],
        },
        {
          label: "User Guide",
          items: [
            { label: "Overview", slug: "user-guide/overview" },
            { label: "Pairing Devices", slug: "user-guide/pairing" },
            { label: "Using Toss", slug: "user-guide/using-toss" },
          ],
        },
        {
          label: "Developer Guide",
          items: [
            { label: "Architecture", slug: "developer-guide/architecture" },
            { label: "API Reference", slug: "developer-guide/api-reference" },
            { label: "Platform Support", slug: "developer-guide/platform-support" },
            { label: "Testing", slug: "developer-guide/testing" },
          ],
        },
        {
          label: "Platform-Specific",
          items: [
            { label: "Overview", slug: "platform-specific/overview" },
            { label: "macOS", slug: "platform-specific/macos" },
            { label: "Windows", slug: "platform-specific/windows" },
            { label: "Linux", slug: "platform-specific/linux" },
            { label: "iOS", slug: "platform-specific/ios" },
            { label: "Android", slug: "platform-specific/android" },
            { label: "iOS & Android Guide", slug: "platform-specific/ios-android" },
          ],
        },
        { label: "Future Enhancements", slug: "future-enhancements" },
      ],
    }),
  ],
});
