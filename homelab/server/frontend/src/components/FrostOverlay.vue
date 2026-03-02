<script setup lang="ts">
defineProps<{
  visible: boolean
}>()
</script>

<template>
  <Teleport to="body">
    <div class="frost-overlay" :class="{ visible }">
      <div class="frost-modal">
        <h2>Homelab Server is unreachable</h2>
        <p>Waiting for connection&hellip;</p>
        <div class="spinner" />
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.frost-overlay {
  position: fixed;
  inset: 0;
  z-index: 9000;
  backdrop-filter: blur(6px) saturate(0.3);
  -webkit-backdrop-filter: blur(6px) saturate(0.3);
  background: var(--c-frost-bg);
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0;
  pointer-events: none;
  transition: opacity 0.6s ease;
}

.frost-overlay.visible {
  opacity: 1;
  pointer-events: auto;
}

.frost-modal {
  background: var(--c-card);
  border: 1px solid var(--c-popover-border);
  border-radius: 12px;
  padding: 28px 36px;
  text-align: center;
  box-shadow: 0 12px 48px rgba(0, 0, 0, 0.6);
  max-width: 340px;
}

.frost-modal h2 {
  color: var(--c-text);
  font-size: 1.1em;
  margin: 0 0 10px;
  font-weight: 600;
}

.frost-modal p {
  color: var(--c-text-dim);
  font-size: 0.85em;
  margin: 0;
  line-height: 1.5;
}

.spinner {
  display: inline-block;
  width: 18px;
  height: 18px;
  border: 2px solid var(--c-popover-border);
  border-top-color: var(--c-frost-spinner);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  margin-top: 14px;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>
