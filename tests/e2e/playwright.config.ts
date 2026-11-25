import { defineConfig, devices } from "@playwright/test";

const PHONE_VIEWPORT = { width: 393, height: 852 };
const LARGE_VIEWPORT = { width: 1600, height: 1000 };
const RTL_VIEWPORT = { width: 1280, height: 800 };

declare module "@playwright/test" {
  interface PlaywrightTestOptions {
    maxTextZoom: string | undefined;
  }
}

export default defineConfig({
  testDir: ".",
  outputDir: "../target/playwright",
  globalSetup: "./src/env.ts",
  fullyParallel: true,
  reporter: [["list"], ["html", { outputFolder: "../target/playwright-html", open: "never" }]],
  use: {
    baseURL: "http://127.0.0.1:4173",
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
  },
  webServer: {
    command:
      "pnpm --filter=@anybot/web exec vite preview --host 127.0.0.1 --port 4173 --strictPort",
    url: "http://127.0.0.1:4173",
    reuseExistingServer: false,
    timeout: 60_000,
  },
  projects: [
    {
      name: "phone-max-font",
      use: {
        ...devices["iPhone 15 Pro"],
        viewport: PHONE_VIEWPORT,
        deviceScaleFactor: 3,
        maxTextZoom: "1.7",
      },
    },
    {
      name: "desktop-large",
      use: { browserName: "chromium", viewport: LARGE_VIEWPORT },
    },
    {
      name: "rtl-arabic",
      use: { browserName: "chromium", viewport: RTL_VIEWPORT, locale: "ar" },
    },
    {
      name: "dark-mode",
      use: { browserName: "chromium", viewport: LARGE_VIEWPORT, colorScheme: "dark" },
    },
    {
      name: "lang-en",
      use: { browserName: "chromium", viewport: LARGE_VIEWPORT, locale: "en" },
    },
    {
      name: "lang-zh-TW",
      use: { browserName: "chromium", viewport: LARGE_VIEWPORT, locale: "zh-TW" },
    },
  ],
});
