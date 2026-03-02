<script setup lang="ts">
import type { DotColor } from '~/types'

const props = defineProps<{
  color: DotColor
  anchorName?: string
}>()

defineEmits<{
  click: []
}>()

const isInteractive = computed(() => props.color !== 'grey')
</script>

<template>
  <div
    class="status-dot"
    :class="[color, { interactive: isInteractive }]"
    :style="anchorName ? { anchorName } : {}"
    :id="anchorName ? anchorName.replace('--', '') : undefined"
    @click="isInteractive && $emit('click')"
  />
</template>

<style scoped>
.status-dot {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  flex-shrink: 0;
  transition: box-shadow 0.3s, background-color 0.3s;
}

.status-dot.interactive {
  cursor: pointer;
}

.status-dot.interactive:active {
  animation: dot-press 0.3s ease;
}

.status-dot.green {
  background-color: var(--c-dot-green);
  box-shadow: 0 0 10px var(--c-dot-green);
}

.status-dot.amber {
  background-color: var(--c-dot-amber);
  box-shadow: 0 0 10px var(--c-dot-amber);
}

.status-dot.red {
  background-color: var(--c-dot-red);
  box-shadow: 0 0 10px var(--c-dot-red);
}

.status-dot.grey {
  background-color: var(--c-dot-grey);
  box-shadow: none;
  cursor: default;
}

@keyframes dot-press {
  0% { transform: scale(1); box-shadow: 0 0 10px currentColor; }
  30% { transform: scale(0.8); box-shadow: 0 0 20px currentColor; }
  100% { transform: scale(1); box-shadow: 0 0 10px currentColor; }
}
</style>
