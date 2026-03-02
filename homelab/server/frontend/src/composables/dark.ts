import { useDark, useToggle } from '@vueuse/core'

export const isDark = useDark({
  // Dark is the default. VueUse adds 'light' class when switching to light mode.
  valueDark: '',
  valueLight: 'light',
})

export const toggleDark = useToggle(isDark)
