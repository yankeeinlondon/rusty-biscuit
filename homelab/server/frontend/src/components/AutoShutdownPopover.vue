<script setup lang="ts">
import { AUTO_SHUTDOWN_OPTIONS } from '~/types'

const props = defineProps<{
  currentValue: number
}>()

const emit = defineEmits<{
  change: [value: number]
}>()

const popoverRef = ref<HTMLElement | null>(null)
const selectValue = ref(props.currentValue.toString())

watch(() => props.currentValue, (val) => {
  selectValue.value = val.toString()
})

function toggle() {
  try {
    if (popoverRef.value?.matches(':popover-open')) {
      popoverRef.value?.hidePopover()
    }
    else {
      popoverRef.value?.showPopover()
      // Sync with server on open
      fetch('/arcam/auto-shutdown')
        .then(r => r.json())
        .then((data) => {
          if (!data.error) {
            selectValue.value = data.value.toString()
          }
        })
        .catch(() => {})
    }
  }
  catch {}
}

function onChange() {
  const value = parseInt(selectValue.value, 10)
  emit('change', value)
  try { popoverRef.value?.hidePopover() } catch {}
}

defineExpose({ toggle })
</script>

<template>
  <div
    id="pop-arcam-auto-shutdown"
    ref="popoverRef"
    popover="auto"
    class="info-popover"
    style="position-anchor: --arcam-auto-shutdown; margin-top: 10px"
  >
    <h4>Auto Shutdown</h4>
    <p>When enabled, the amplifier monitors its audio input. If no signal is detected for the configured period, it enters low-power standby mode.</p>
    <select
      v-model="selectValue"
      class="auto-shutdown-select"
      @change="onChange"
    >
      <option
        v-for="(label, value) in AUTO_SHUTDOWN_OPTIONS"
        :key="value"
        :value="value.toString()"
      >
        {{ label }}
      </option>
    </select>
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

.info-popover h4 {
  margin: 0 0 8px;
  color: var(--c-text);
  font-size: 0.95em;
}

.info-popover p {
  margin: 0;
}

.auto-shutdown-select {
  margin: 0.5em 0 0;
  padding: 8px 12px;
  font-size: 0.9em;
  background: #0f1a2e;
  color: var(--c-popover-text);
  border: 1px solid var(--c-popover-border);
  border-radius: 6px;
  cursor: pointer;
  width: 100%;
  appearance: none;
  -webkit-appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='8' viewBox='0 0 12 8'%3E%3Cpath fill='%237cc4ff' d='M1 1l5 5 5-5'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 10px center;
}

.auto-shutdown-select:hover {
  border-color: #2a6aaa;
}

.auto-shutdown-select:focus {
  outline: none;
  border-color: var(--c-accent);
  box-shadow: 0 0 0 2px rgba(124, 196, 255, 0.15);
}

.auto-shutdown-select option {
  background: #0f1a2e;
  color: var(--c-popover-text);
  padding: 6px;
}

html.light .auto-shutdown-select {
  background-color: #f8f8f8;
  border-color: rgba(0, 0, 0, 0.15);
}

html.light .auto-shutdown-select option {
  background: #fff;
  color: #333;
}
</style>
