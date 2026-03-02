<script setup lang="ts">
import type { ArcamActiveDetail } from '~/types'
import AutoShutdownPopoverVue from '~/components/AutoShutdownPopover.vue'

const props = defineProps<{
  detail: ArcamActiveDetail
}>()

const emit = defineEmits<{
  autoShutdownChange: [value: number]
}>()

const autoShutdownPopover = ref<InstanceType<typeof AutoShutdownPopoverVue> | null>(null)

const autoShutdownCls = computed(() =>
  props.detail.auto_shutdown === 0 ? 'badge-auto-shutdown-off' : 'badge-auto-shutdown',
)
</script>

<template>
  <BadgeRow>
    <span
      v-if="detail.mode"
      id="arcam-mode-badge"
      class="badge"
      style="anchor-name: --arcam-mode; cursor: help"
    >
      {{ detail.mode }}
    </span>
    <span
      v-if="detail.muted"
      class="badge badge-muted"
    >
      MUTED
    </span>
    <span
      v-if="detail.auto_shutdown !== undefined"
      id="arcam-auto-shutdown-badge"
      class="badge"
      :class="autoShutdownCls"
      style="anchor-name: --arcam-auto-shutdown; cursor: pointer"
      @click="autoShutdownPopover?.toggle()"
    >
      Auto Shutdown: {{ detail.auto_shutdown_label }}
    </span>
  </BadgeRow>

  <AutoShutdownPopover
    ref="autoShutdownPopover"
    :current-value="detail.auto_shutdown ?? 0"
    @change="emit('autoShutdownChange', $event)"
  />
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
  transition: filter 0.2s ease, transform 0.2s ease;
}

.badge-muted {
  background: #4a2e0a;
  border-color: #7a5a1a;
  color: #f0a030;
}

.badge-auto-shutdown {
  background: #1a3a2e;
  border-color: #2a5a4e;
  color: #80c8a0;
}

.badge-auto-shutdown-off {
  background: #2a2a2a;
  border-color: #3a3a3a;
  color: #888;
}

html.light .badge {
  background: var(--c-badge-dim);
  border-color: rgba(0, 0, 0, 0.12);
  color: var(--c-badge-dim-text);
}

html.light .badge-muted {
  background: rgba(240, 160, 48, 0.15);
  border-color: rgba(240, 160, 48, 0.3);
  color: #c07000;
}

html.light .badge-auto-shutdown {
  background: rgba(67, 160, 71, 0.1);
  border-color: rgba(67, 160, 71, 0.2);
  color: #2e7d32;
}

html.light .badge-auto-shutdown-off {
  background: rgba(0, 0, 0, 0.04);
  border-color: rgba(0, 0, 0, 0.08);
  color: #999;
}
</style>
