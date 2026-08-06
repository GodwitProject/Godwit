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
        // Surface
        surface: '#f8f9fb',
        'surface-dim': '#d9dadc',
        'surface-bright': '#f8f9fb',
        'surface-container-lowest': '#ffffff',
        'surface-container-low': '#f3f4f6',
        'surface-container': '#edeef0',
        'surface-container-high': '#e7e8ea',
        'surface-container-highest': '#e1e2e4',
        
        // On Surface
        'on-surface': '#191c1e',
        'on-surface-variant': '#434655',
        
        // Primary (Godwit Cobalt Blue)
        primary: '#004ac6',
        'on-primary': '#ffffff',
        'primary-container': '#2563eb',
        'on-primary-container': '#eeefff',
        'primary-fixed': '#dbe1ff',
        'primary-fixed-dim': '#b4c5ff',
        
        // Secondary
        secondary: '#515f74',
        'on-secondary': '#ffffff',
        'secondary-container': '#d5e3fc',
        'on-secondary-container': '#57657a',
        
        // Tertiary
        tertiary: '#005a82',
        'on-tertiary': '#ffffff',
        'tertiary-container': '#0074a6',
        'on-tertiary-container': '#e4f2ff',
        
        // Error
        error: '#ba1a1a',
        'on-error': '#ffffff',
        'error-container': '#ffdad6',
        
        // Functional
        success: '#10b981',
        warning: '#f59e0b',
        info: '#3b82f6',
        
        // Borders
        outline: '#737686',
        'outline-variant': '#c3c6d7',
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
        mono: ['JetBrains Mono', 'monospace'],
      },
      fontSize: {
        'display-lg': ['30px', { lineHeight: '36px', fontWeight: '700', letterSpacing: '-0.02em' }],
        'headline-md': ['24px', { lineHeight: '32px', fontWeight: '700', letterSpacing: '-0.01em' }],
        'title-md': ['20px', { lineHeight: '28px', fontWeight: '700' }],
        'section-sm': ['18px', { lineHeight: '28px', fontWeight: '600' }],
        'body-base': ['16px', { lineHeight: '24px', fontWeight: '400' }],
        'label-sm': ['14px', { lineHeight: '20px', fontWeight: '500' }],
        'caption-xs': ['12px', { lineHeight: '16px', fontWeight: '400' }],
        'code-sm': ['13px', { lineHeight: '20px', fontWeight: '400' }],
      },
      spacing: {
        'base-unit': '4px',
        'gutter': '16px',
        'margin-mobile': '16px',
        'margin-desktop': '32px',
        'sidebar-width': '256px',
        'container-padding': '24px',
      },
      borderRadius: {
        'DEFAULT': '0.125rem',
        'lg': '0.25rem',
        'xl': '0.5rem',
        'full': '9999px',
      },
    },
  },
  plugins: [],
};
export default config;
