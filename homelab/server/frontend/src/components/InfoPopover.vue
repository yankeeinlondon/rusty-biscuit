<script setup lang="ts">
const props = defineProps<{
  popoverId: string
  anchorName?: string
}>()

const popoverRef = ref<HTMLElement | null>(null)
let showTimer: ReturnType<typeof setTimeout> | undefined
let hideTimer: ReturnType<typeof setTimeout> | undefined
const created = Date.now()
const canHover = typeof window !== 'undefined' && window.matchMedia('(hover: hover)').matches

function startShow() {
  if (!canHover) return
  if (Date.now() - created < 100) return
  clearTimeout(hideTimer)
  showTimer = setTimeout(() => {
    try { popoverRef.value?.showPopover() } catch {}
  }, 300)
}

function startHide() {
  clearTimeout(showTimer)
  hideTimer = setTimeout(() => {
    try { popoverRef.value?.hidePopover() } catch {}
  }, 150)
}

function cancelHide() {
  clearTimeout(hideTimer)
}

function immediateHide() {
  try { popoverRef.value?.hidePopover() } catch {}
}

defineExpose({
  startShow,
  startHide,
  popoverRef,
})
</script>

<template>
  <div
    :id="popoverId"
    ref="popoverRef"
    popover="manual"
    class="info-popover"
    :style="anchorName ? { positionAnchor: anchorName } : {}"
    @mouseenter="cancelHide"
    @mouseleave="immediateHide"
  >
    <slot />
  </div>
</template>

<style scoped>
.info-popover {
  margin: 0;
  border: 1px solid var(--c-popover-border);
  border-radius: 8px;
  padding: 14px 18px;
  background: var(--c-popover-bg);
  color: var(--c-popover-text);
  font-size: 0.85em;
  line-height: 1.6;
  max-width: 320px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  position-area: bottom;
  position-try-fallbacks: flip-block;
}

.info-popover,
.info-popover:popover-open {
  opacity: 1;
  transform: translateY(0);
  transition: opacity 0.2s ease, transform 0.2s ease, overlay 0.2s allow-discrete, display 0.2s allow-discrete;
}

.info-popover:not(:popover-open) {
  opacity: 0;
  transform: translateY(-4px);
}

@starting-style {
  .info-popover:popover-open {
    opacity: 0;
    transform: translateY(-4px);
  }
}

.info-popover :deep(h4) {
  margin: 0 0 8px;
  color: var(--c-text);
  font-size: 0.95em;
}

.info-popover :deep(dl) {
  margin: 0;
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 4px 12px;
}

.info-popover :deep(dt) {
  color: var(--c-accent);
  font-weight: 600;
  white-space: nowrap;
}

.info-popover :deep(dd) {
  margin: 0;
  color: var(--c-text-muted);
}

.info-popover :deep(.note) {
  margin-top: 8px;
  padding-top: 8px;
  border-top: 1px solid var(--c-popover-border);
  color: var(--c-text-dim);
  font-size: 0.9em;
  font-style: italic;
}
</style>
