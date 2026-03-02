<script setup lang="ts">
import { isDark, toggleDark } from '~/composables/dark'
</script>

<template>
  <button
    class="theme-toggle"
    :title="isDark ? 'Switch to light mode' : 'Switch to dark mode'"
    @click="toggleDark()"
  >
    <div v-if="isDark" class="i-carbon-moon" />
    <div v-else class="i-carbon-sun" />
  </button>
</template>

<style scoped>
.theme-toggle {
  position: fixed;
  top: 16px;
  right: 16px;
  z-index: 100;
  cursor: pointer;
  border: none;
  background: transparent;
  color: var(--c-text-muted);
  font-size: 1.3em;
  padding: 8px;
  border-radius: 8px;
  transition: color 0.2s, background-color 0.2s;
}

.theme-toggle:hover {
  color: var(--c-text);
  background: rgba(127, 127, 127, 0.1);
}
</style>
