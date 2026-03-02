<script setup lang="ts">
const props = defineProps<{
  volume: number
  maxVolume: number
  muted: boolean
}>()

const percentage = computed(() =>
  Math.min(100, Math.round((props.volume / (props.maxVolume || 100)) * 100)),
)
</script>

<template>
  <div class="volume-wrap">
    <div class="volume-track">
      <div
        class="volume-fill"
        :class="{ muted }"
        :style="{ width: `${percentage}%` }"
      />
    </div>
    <span class="volume-label">{{ volume }}</span>
  </div>
</template>

<style scoped>
.volume-wrap {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 140px;
}

.volume-track {
  flex: 1;
  height: 5px;
  background: var(--c-volume-bg);
  border-radius: 3px;
  overflow: hidden;
}

.volume-fill {
  height: 100%;
  border-radius: 3px;
  background: linear-gradient(90deg, var(--c-volume-fill-start), var(--c-volume-fill-end));
  transition: width 0.4s cubic-bezier(0.4, 0, 0.2, 1);
  box-shadow: 0 0 6px rgba(79, 195, 247, 0.27);
}

.volume-fill.muted {
  background: linear-gradient(90deg, #5a1a1a, var(--c-volume-muted));
  box-shadow: 0 0 6px rgba(204, 68, 68, 0.27);
}

.volume-label {
  font-size: 0.7em;
  color: var(--c-text-dim);
  min-width: 18px;
  text-align: right;
  font-variant-numeric: tabular-nums;
}
</style>
