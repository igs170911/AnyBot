import { defineConfig, devices } from "@playwright/test";

const PHONE_VIEWPORT = { width: 375, height: 667 };
const LARGE_VIEWPORT = { width: 1600, height: 1000 };
const RTL_VIEWPORT = { width: 1280, height: 800 };

declare module "@playwright/test" {
  interface PlaywrightTestOptions {
    maxTextZoom: string | undefined;
    wideFont: string | undefined;
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
    screenshot: "on",
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
        ...devices["iPhone SE (3rd gen)"],
        viewport: PHONE_VIEWPORT,
        maxTextZoom: "3.1",
      },
    },
    {
      name: "phone-wide-font",
      use: {
        ...devices["iPhone SE (3rd gen)"],
        viewport: PHONE_VIEWPORT,
        maxTextZoom: "3.1",
        wideFont: "1",
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
