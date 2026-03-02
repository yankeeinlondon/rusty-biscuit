<script setup lang="ts">
import type { SonySource } from '~/types'

defineProps<{
  source: SonySource
}>()

defineEmits<{
  select: [category: string, uri: string]
}>()
</script>

<template>
  <span
    class="badge"
    :class="{
      'badge-active': source.active,
      'badge-dim': !source.active,
      clickable: !!source.uri && !source.active,
    }"
    @click="source.uri && !source.active && $emit('select', source.category, source.uri)"
  >
    {{ source.name }}
  </span>
</template>

<style scoped>
.badge {
  background: #0f3460;
  border: 1px solid #1a4a7a;
  border-radius: 4px;
  padding: 4px 10px;
  font-size: 0.75em;
  color: #a8c8e8;
  white-space: nowrap;
  letter-spacing: 0.02em;
  transition: filter 0.2s ease, transform 0.2s ease, background 0.2s, border-color 0.2s, color 0.2s;
}

.badge.clickable {
  cursor: pointer;
}

.badge.clickable:hover {
  background: rgba(15, 52, 96, 0.4);
  border-color: #1a5a8a;
  color: #7ab8e0;
}

.badge-active {
  cursor: default;
}

.badge-dim {
  background: rgba(15, 52, 96, 0.15);
  border-color: #1a3050;
  color: #4a6278;
}

html.light .badge {
  background: var(--c-badge-dim);
  border-color: rgba(0, 0, 0, 0.12);
  color: var(--c-badge-dim-text);
}

html.light .badge-active {
  background: var(--c-badge-active);
  color: var(--c-badge-active-text);
}

html.light .badge.clickable:hover {
  background: var(--c-badge-hover);
  color: var(--c-badge-hover-text);
}
</style>
