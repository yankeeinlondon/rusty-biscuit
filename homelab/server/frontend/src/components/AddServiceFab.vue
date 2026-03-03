<script setup lang="ts">
defineEmits<{
  click: []
}>()
</script>

<template>
  <button class="fab" aria-label="Add service" @click="$emit('click')">
    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
      <line x1="12" y1="5" x2="12" y2="19" />
      <line x1="5" y1="12" x2="19" y2="12" />
    </svg>
  </button>
</template>

<style scoped>
.fab {
  position: fixed;
  bottom: 24px;
  right: 24px;
  z-index: 100;
  width: 56px;
  height: 56px;
  border-radius: 50%;
  border: none;
  background: var(--c-accent);
  color: #fff;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow:
    0 4px 16px rgba(79, 195, 247, 0.35),
    0 2px 6px rgba(0, 0, 0, 0.3);
  transition: transform 0.2s ease, box-shadow 0.2s ease;
}

.fab:hover {
  transform: scale(1.08);
  box-shadow:
    0 6px 24px rgba(79, 195, 247, 0.5),
    0 2px 8px rgba(0, 0, 0, 0.4);
}

.fab:active {
  transform: scale(0.95);
}
</style>
