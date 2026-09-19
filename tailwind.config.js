/** @type {import('tailwindcss').Config} */
export default {
  darkMode: 'class',
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      // Every colour resolves through a CSS variable so the two themes are
      // defined in one place (index.css) rather than duplicated here.
      colors: {
        canvas: 'rgb(var(--c-canvas) / <alpha-value>)',
        surface: 'rgb(var(--c-surface) / <alpha-value>)',
        raised: 'rgb(var(--c-raised) / <alpha-value>)',
        sunken: 'rgb(var(--c-sunken) / <alpha-value>)',
        line: 'rgb(var(--c-line) / <alpha-value>)',
        'line-strong': 'rgb(var(--c-line-strong) / <alpha-value>)',
        ink: 'rgb(var(--c-ink) / <alpha-value>)',
        'ink-soft': 'rgb(var(--c-ink-soft) / <alpha-value>)',
        'ink-faint': 'rgb(var(--c-ink-faint) / <alpha-value>)',
        accent: 'rgb(var(--c-accent) / <alpha-value>)',
        'accent-hover': 'rgb(var(--c-accent-hover) / <alpha-value>)',
        'accent-soft': 'rgb(var(--c-accent-soft) / <alpha-value>)',
        'on-accent': 'rgb(var(--c-on-accent) / <alpha-value>)',
        danger: 'rgb(var(--c-danger) / <alpha-value>)',
        'danger-soft': 'rgb(var(--c-danger-soft) / <alpha-value>)',
        warn: 'rgb(var(--c-warn) / <alpha-value>)',
        success: 'rgb(var(--c-success) / <alpha-value>)',
      },
      fontFamily: {
        sans: ['var(--font-sans)'],
        mono: ['var(--font-mono)'],
      },
      fontSize: {
        // Driven by the Appearance → font size setting.
        body: ['var(--fs-body)', { lineHeight: 'var(--lh-body)' }],
      },
      borderRadius: {
        sm: '4px',
        DEFAULT: '6px',
        md: '8px',
        lg: '10px',
        xl: '14px',
      },
      boxShadow: {
        subtle: '0 1px 2px rgb(0 0 0 / 0.04), 0 1px 1px rgb(0 0 0 / 0.03)',
        raised: '0 4px 12px rgb(0 0 0 / 0.08), 0 1px 3px rgb(0 0 0 / 0.06)',
        overlay: '0 16px 48px rgb(0 0 0 / 0.18), 0 2px 8px rgb(0 0 0 / 0.10)',
      },
      transitionDuration: { DEFAULT: '120ms' },
      keyframes: {
        'fade-in': { from: { opacity: '0' }, to: { opacity: '1' } },
        'slide-up': {
          from: { opacity: '0', transform: 'translateY(4px)' },
          to: { opacity: '1', transform: 'translateY(0)' },
        },
        caret: { '0%, 100%': { opacity: '1' }, '50%': { opacity: '0.15' } },
      },
      animation: {
        'fade-in': 'fade-in 120ms ease-out',
        'slide-up': 'slide-up 140ms ease-out',
        caret: 'caret 1s steps(1) infinite',
      },
    },
  },
  plugins: [],
};
