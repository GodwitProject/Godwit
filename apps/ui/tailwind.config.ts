import type { Config } from "tailwindcss";

const config: Config = {
  content: [
    "./src/pages/**/*.{js,ts,jsx,tsx,mdx}",
    "./src/components/**/*.{js,ts,jsx,tsx,mdx}",
    "./src/app/**/*.{js,ts,jsx,tsx,mdx}",
  ],
  theme: {
    extend: {
      colors: {
        // Surfaces (cold neutral, from prototype)
        bg: 'oklch(98% 0.005 250)',
        surface: 'oklch(100% 0 0)',
        'sidebar-bg': 'oklch(97% 0.006 250)',
        'surface-2': 'oklch(93% 0.006 250)',
        'surface-2_5': 'oklch(97.5% 0.004 250)',

        // Foreground
        fg: 'oklch(22% 0.02 240)',
        muted: 'oklch(50% 0.018 240)',
        border: 'oklch(90% 0.008 240)',

        // Accent (green)
        accent: 'oklch(58% 0.16 145)',
        'accent-strong': 'oklch(50% 0.17 145)',
        'on-accent': '#0c1f12',

        // Functional
        success: 'oklch(55% 0.15 155)',
        warn: 'oklch(60% 0.15 80)',
        danger: 'oklch(55% 0.19 25)',
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', '-apple-system', 'Segoe UI', 'sans-serif'],
        mono: ['"JetBrains Mono"', '"IBM Plex Mono"', 'ui-monospace', 'Menlo', 'monospace'],
      },
      fontSize: {
        'display-lg': ['22px', { lineHeight: '28px', fontWeight: '600', letterSpacing: '-0.02em' }],
        'headline-md': ['24px', { lineHeight: '32px', fontWeight: '700', letterSpacing: '-0.01em' }],
        'title-md': ['20px', { lineHeight: '28px', fontWeight: '600' }],
        'title-sm': ['15px', { lineHeight: '22px', fontWeight: '600', letterSpacing: '-0.01em' }],
        'section-sm': ['13px', { lineHeight: '20px', fontWeight: '560', letterSpacing: '-0.01em' }],
        'body-base': ['13.5px', { lineHeight: '20px', fontWeight: '450' }],
        'label-sm': ['12.5px', { lineHeight: '20px', fontWeight: '510' }],
        'caption-xs': ['11px', { lineHeight: '16px', fontWeight: '510', letterSpacing: '0.06em' }],
        'code-sm': ['11.5px', { lineHeight: '20px', fontWeight: '400' }],
      },
      borderRadius: {
        'DEFAULT': '6px',
        'lg': '6px',
        'xl': '10px',
        '2xl': '12px',
        'full': '9999px',
      },
      spacing: {
        'sidebar-width': '232px',
        'gutter': '16px',
        'margin-mobile': '16px',
        'margin-desktop': '32px',
        'container-padding': '16px',
      },
      boxShadow: {
        'ambient': '0 1px 2px oklch(90% 0.01 250), inset 0 0 0 1px oklch(90% 0.008 240)',
        'drawer': '0 24px 60px oklch(20% 0.02 250 / 0.25)',
      },
    },
  },
  plugins: [],
};
export default config;
