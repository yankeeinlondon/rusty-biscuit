import {
  defineConfig,
  presetAttributify,
  presetIcons,
  presetWind4,
} from 'unocss'

export default defineConfig({
  presets: [
    presetWind4(),
    presetAttributify(),
    presetIcons({
      scale: 1.2,
      warn: true,
    }),
  ],
  shortcuts: {
    'device-card': 'bg-[var(--c-card)] rounded-2xl p-6 shadow-lg',
    'badge': 'px-3 py-1.5 rounded-full text-sm font-medium transition-all duration-200',
    'badge-active': 'bg-[var(--c-badge-active)] text-[var(--c-badge-active-text)]',
    'badge-dim': 'bg-[var(--c-badge-dim)] text-[var(--c-badge-dim-text)]',
    'badge-hover': 'hover:bg-[var(--c-badge-hover)] hover:text-[var(--c-badge-hover-text)]',
    'btn-icon': 'cursor-pointer border-none bg-transparent text-[var(--c-text-muted)] hover:text-[var(--c-text)] transition-colors duration-200',
  },
})
