<script setup lang="ts">
import type { CreateDeviceRequest, ServiceKindMeta } from '~/types'
import { SERVICE_KINDS } from '~/types'

const props = defineProps<{
  visible: boolean
}>()

const emit = defineEmits<{
  close: []
  created: []
}>()

const { createDevice } = useApi()

// Step state
const step = ref<'pick' | 'configure'>('pick')
const selectedKind = ref<ServiceKindMeta | null>(null)

// Form state
const name = ref('')
const host = ref('')
const port = ref<number | undefined>(undefined)
const error = ref('')
const submitting = ref(false)

// Name validation: lowercase alphanumeric, hyphens, underscores
const namePattern = /^[a-z0-9_-]+$/

const nameValid = computed(() =>
  name.value.length > 0 && namePattern.test(name.value),
)

const canSubmit = computed(() =>
  nameValid.value && host.value.length > 0 && !submitting.value,
)

function selectKind(kind: ServiceKindMeta) {
  selectedKind.value = kind
  step.value = 'configure'
  error.value = ''
}

function goBack() {
  step.value = 'pick'
  error.value = ''
}

function resetAndClose() {
  step.value = 'pick'
  selectedKind.value = null
  name.value = ''
  host.value = ''
  port.value = undefined
  error.value = ''
  submitting.value = false
  emit('close')
}

async function submit() {
  if (!canSubmit.value || !selectedKind.value) return

  submitting.value = true
  error.value = ''

  const body: CreateDeviceRequest = {
    name: name.value,
    host: host.value,
    port: port.value || undefined,
  }

  try {
    await createDevice(selectedKind.value.apiPath, body)
    emit('created')
    resetAndClose()
  }
  catch (err) {
    error.value = err instanceof Error ? err.message : 'Request failed'
  }
  finally {
    submitting.value = false
  }
}

function onBackdropClick(e: MouseEvent) {
  if ((e.target as HTMLElement).classList.contains('modal-overlay')) {
    resetAndClose()
  }
}
</script>

<template>
  <Teleport to="body">
    <div class="modal-overlay" :class="{ visible: props.visible }" @click="onBackdropClick">
      <div class="modal-card">
        <!-- Step 1: Kind Picker -->
        <template v-if="step === 'pick'">
          <div class="modal-header">
            <h2>Add Service</h2>
            <button class="close-btn" aria-label="Close" @click="resetAndClose">&times;</button>
          </div>
          <div class="kind-list">
            <button
              v-for="kind in SERVICE_KINDS"
              :key="kind.kind"
              class="kind-option"
              @click="selectKind(kind)"
            >
              <span class="kind-label">{{ kind.label }}</span>
              <span class="kind-desc">{{ kind.description }}</span>
            </button>
          </div>
        </template>

        <!-- Step 2: Configure -->
        <template v-else>
          <div class="modal-header">
            <h2>{{ selectedKind!.label }}</h2>
            <button class="close-btn" aria-label="Close" @click="resetAndClose">&times;</button>
          </div>
          <button class="back-link" @click="goBack">&larr; Back</button>

          <form class="config-form" @submit.prevent="submit">
            <label class="field">
              <span class="field-label">Name</span>
              <input
                v-model="name"
                type="text"
                placeholder="e.g. living-room"
                autocomplete="off"
                :class="{ invalid: name.length > 0 && !nameValid }"
              >
              <span v-if="name.length > 0 && !nameValid" class="field-error">
                Lowercase letters, numbers, hyphens, underscores only
              </span>
            </label>

            <label class="field">
              <span class="field-label">Host</span>
              <input
                v-model="host"
                type="text"
                placeholder="e.g. 192.168.1.100"
                autocomplete="off"
              >
            </label>

            <label class="field">
              <span class="field-label">Port</span>
              <input
                v-model.number="port"
                type="number"
                :placeholder="String(selectedKind!.defaultPort)"
              >
            </label>

            <div v-if="error" class="form-error">{{ error }}</div>

            <button type="submit" class="submit-btn" :disabled="!canSubmit">
              <template v-if="submitting">Adding&hellip;</template>
              <template v-else>Add Service</template>
            </button>
          </form>
        </template>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.modal-overlay {
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
  transition: opacity 0.3s ease;
}

.modal-overlay.visible {
  opacity: 1;
  pointer-events: auto;
}

.modal-card {
  background: var(--c-card);
  border: 1px solid var(--c-popover-border);
  border-radius: 12px;
  padding: 24px 28px;
  box-shadow: 0 12px 48px rgba(0, 0, 0, 0.6);
  width: 360px;
  max-width: calc(100vw - 40px);
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}

.modal-header h2 {
  color: var(--c-text);
  font-size: 1.1em;
  font-weight: 600;
  margin: 0;
}

.close-btn {
  background: none;
  border: none;
  color: var(--c-text-muted);
  font-size: 1.4em;
  cursor: pointer;
  padding: 0 4px;
  line-height: 1;
}

.close-btn:hover {
  color: var(--c-text);
}

/* Kind picker */
.kind-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.kind-option {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 2px;
  padding: 12px 16px;
  border: 1px solid var(--c-popover-border);
  border-radius: 8px;
  background: transparent;
  cursor: pointer;
  text-align: left;
  transition: background 0.15s ease, border-color 0.15s ease;
}

.kind-option:hover {
  background: var(--c-badge-hover);
  border-color: var(--c-accent);
}

.kind-label {
  color: var(--c-text);
  font-size: 0.95em;
  font-weight: 600;
}

.kind-desc {
  color: var(--c-text-muted);
  font-size: 0.8em;
}

/* Back link */
.back-link {
  background: none;
  border: none;
  color: var(--c-accent);
  font-size: 0.8em;
  cursor: pointer;
  padding: 0;
  margin-bottom: 16px;
}

.back-link:hover {
  text-decoration: underline;
}

/* Config form */
.config-form {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.field-label {
  color: var(--c-text-muted);
  font-size: 0.8em;
  font-weight: 500;
}

.field input {
  background: var(--c-bg);
  border: 1px solid var(--c-popover-border);
  border-radius: 6px;
  padding: 8px 12px;
  color: var(--c-text);
  font-size: 0.9em;
  outline: none;
  transition: border-color 0.15s ease;
}

.field input::placeholder {
  color: var(--c-text-dim);
}

.field input:focus {
  border-color: var(--c-accent);
}

.field input.invalid {
  border-color: var(--c-dot-red);
}

.field-error {
  color: var(--c-dot-red);
  font-size: 0.75em;
}

.form-error {
  color: var(--c-dot-red);
  font-size: 0.85em;
  padding: 8px 12px;
  background: rgba(244, 67, 54, 0.1);
  border-radius: 6px;
}

.submit-btn {
  background: var(--c-accent);
  color: #fff;
  border: none;
  border-radius: 8px;
  padding: 10px 16px;
  font-size: 0.9em;
  font-weight: 600;
  cursor: pointer;
  transition: opacity 0.15s ease;
}

.submit-btn:hover:not(:disabled) {
  opacity: 0.9;
}

.submit-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
</style>
