<script setup lang="ts">
import type { DotColor } from '~/types'

defineProps<{
  name: string
  label: string
  host?: string
  dotColor: DotColor
  dotAnchorName?: string
}>()

defineEmits<{
  powerToggle: []
  dotEnter: []
  dotLeave: []
}>()
</script>

<template>
  <div class="device">
    <div class="device-row">
      <StatusDot
        :color="dotColor"
        :anchor-name="dotAnchorName"
        @click="$emit('powerToggle')"
        @mouseenter="$emit('dotEnter')"
        @mouseleave="$emit('dotLeave')"
      />
      <div class="device-info">
        <div class="device-name">{{ name }}</div>
        <div class="device-detail">{{ label }}</div>
        <div v-if="host" class="host" v-html="host" />
      </div>
      <div class="instruments">
        <slot name="instruments" />
      </div>
    </div>
    <slot name="extra" />
  </div>
</template>

<style scoped>
.device {
  background: var(--c-card);
  border-radius: 12px;
  padding: 20px 24px;
  margin-bottom: 16px;
}

.device-row {
  display: flex;
  align-items: center;
  gap: 16px;
}

.device-info {
  flex: 1;
  min-width: 0;
}

.device-name {
  font-weight: 600;
}

.device-detail {
  color: var(--c-text-muted);
  font-size: 0.85em;
}

.host {
  color: var(--c-text-dim);
  font-size: 0.8em;
}

.host :deep(.port) {
  opacity: 0.5;
}

.instruments {
  display: flex;
  flex-direction: column;
  gap: 2px;
  align-items: flex-end;
  flex-shrink: 0;
  justify-content: center;
}
</style>
