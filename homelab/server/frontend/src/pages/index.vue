<script setup lang="ts">
import type { ArcamActiveDetail, ArcamStandbyDetail, SonyDetail } from '~/types'
import { statusToDotColor } from '~/types'
import { useDeviceControl } from '~/composables/useDeviceControl'
import InfoPopoverVue from '~/components/InfoPopover.vue'

const {
  sonyStatus,
  arcamStatus,
  lastArcamOnDetail,
  lastArcamModel,
  isUnreachable,
  sonyDeviceInfo,
  arcamDeviceInfo,
  eversoloDevices,
  samsungDevices,
  eversoloStatus,
  samsungTvStatus,
  sonyPowerLock,
  arcamPowerLock,
  toggleSonyPower,
  toggleArcamPower,
  selectSonySource,
  setAutoShutdown,
  refreshDevices,
} = useDeviceControl()

// Effective dot colors (respecting optimistic locks)
const sonyDotColor = computed(() =>
  sonyPowerLock.isActive()
    ? sonyPowerLock.lockedValue.value!
    : statusToDotColor(sonyStatus.value.status),
)

const arcamDotColor = computed(() =>
  arcamPowerLock.isActive()
    ? arcamPowerLock.lockedValue.value!
    : statusToDotColor(arcamStatus.value.status),
)

// Sony detail typed
const sonyDetail = computed(() =>
  sonyStatus.value.detail as SonyDetail | undefined,
)

// Arcam detail typed
const arcamDetail = computed(() =>
  arcamStatus.value.detail as ArcamActiveDetail | ArcamStandbyDetail | undefined,
)

const arcamActiveDetail = computed(() =>
  arcamStatus.value.status === 'active' ? arcamDetail.value as ArcamActiveDetail : undefined,
)

const arcamStandbyDetail = computed(() =>
  arcamStatus.value.status === 'standby' ? arcamDetail.value as ArcamStandbyDetail : undefined,
)

// Sorted sources: Media Box and Game pinned first, then alphabetical
const sortedSources = computed(() => {
  const sources = sonyDetail.value?.sources
  if (!sources?.length) return []

  const pinned = ['Media Box', 'Game']
  return [...sources].sort((a, b) => {
    const ai = pinned.indexOf(a.name)
    const bi = pinned.indexOf(b.name)
    if (ai !== -1 && bi !== -1) return ai - bi
    if (ai !== -1) return -1
    if (bi !== -1) return 1
    return a.name.localeCompare(b.name)
  })
})

// Host display helpers
const sonyHostHtml = computed(() => {
  if (!sonyDeviceInfo.value) return ''
  return `${sonyDeviceInfo.value.host}<span class="port">:${sonyDeviceInfo.value.port}</span>`
})

const arcamHostHtml = computed(() => {
  if (!arcamDeviceInfo.value) return ''
  return `${arcamDeviceInfo.value.host}<span class="port">:${arcamDeviceInfo.value.port}</span>`
})

// Eversolo status
const eversoloDotColor = computed(() =>
  eversoloStatus.value ? statusToDotColor(eversoloStatus.value.status) : 'grey',
)

// Samsung TV status
const samsungDotColor = computed(() =>
  samsungTvStatus.value ? statusToDotColor(samsungTvStatus.value.status) : 'grey',
)

// Add-service modal
const showAddModal = ref(false)

// Popover refs
const sonyPopover = ref<InstanceType<typeof InfoPopoverVue> | null>(null)
const arcamPopover = ref<InstanceType<typeof InfoPopoverVue> | null>(null)
const arcamModePopover = ref<InstanceType<typeof InfoPopoverVue> | null>(null)
</script>

<template>
  <div class="dashboard">
    <ThemeToggle />

    <h1>Homelab Server</h1>
    <p class="subtitle">AV Device Control</p>

    <!-- Sony Receiver -->
    <DeviceCard
      name="Sony Receiver"
      :label="sonyStatus.label"
      :host="sonyHostHtml"
      :dot-color="sonyDotColor"
      dot-anchor-name="--sony-dot"
      @power-toggle="toggleSonyPower()"
      @dot-enter="sonyPopover?.startShow()"
      @dot-leave="sonyPopover?.startHide()"
    >
      <template #instruments>
        <VolumeBar
          v-if="sonyDetail?.volume !== undefined"
          :volume="sonyDetail.volume"
          :max-volume="sonyDetail.max_volume ?? 100"
          :muted="sonyDetail.muted ?? false"
        />
      </template>
      <template #extra>
        <div class="sources" :class="{ visible: sortedSources.length > 0 }">
          <BadgeRow>
            <SourceBadge
              v-for="source in sortedSources"
              :key="source.category"
              :source="source"
              @select="selectSonySource"
            />
          </BadgeRow>
        </div>
      </template>
    </DeviceCard>

    <!-- Arcam Amplifier -->
    <DeviceCard
      name="Arcam Amplifier"
      :label="arcamStatus.label"
      :host="arcamHostHtml"
      :dot-color="arcamDotColor"
      dot-anchor-name="--arcam-dot"
      @power-toggle="toggleArcamPower()"
      @dot-enter="arcamPopover?.startShow()"
      @dot-leave="arcamPopover?.startHide()"
    >
      <template #instruments>
        <!-- Standby: model + heartbeat -->
        <template v-if="arcamStandbyDetail?.model">
          <span class="device-model">{{ arcamStandbyDetail.model }}</span>
          <span class="heartbeat" title="Heartbeat keepalive active">&#x2764;&#xFE0F;</span>
        </template>
        <!-- Active: badges -->
        <ArcamBadges
          v-if="arcamActiveDetail"
          :detail="arcamActiveDetail"
          @auto-shutdown-change="setAutoShutdown"
        />
      </template>
    </DeviceCard>

    <!-- Eversolo Streamers -->
    <DeviceCard
      v-for="device in eversoloDevices"
      :key="`eversolo-${device.name}`"
      :name="`Eversolo — ${device.name}`"
      :label="eversoloStatus?.label ?? 'Loading...'"
      :host="`${device.host}<span class='port'>:${device.port}</span>`"
      :dot-color="eversoloDotColor"
    />

    <!-- Samsung TVs -->
    <DeviceCard
      v-for="device in samsungDevices"
      :key="`samsung-${device.name}`"
      :name="`Samsung TV — ${device.name}`"
      :label="samsungTvStatus?.label ?? 'Loading...'"
      :host="`${device.host}<span class='port'>:${device.port}</span>`"
      :dot-color="samsungDotColor"
    />

    <p class="explore">
      Try interacting with the API by using the <a href="/explore">explore</a> UI.
    </p>

    <!-- Popovers -->
    <InfoPopover ref="sonyPopover" popover-id="pop-sony-power" anchor-name="--sony-dot">
      <h4>Sony Receiver &mdash; Power States</h4>
      <dl>
        <dt>Active</dt>
        <dd>Powered on and fully operational. All features and controls are available.</dd>
        <dt>Standby</dt>
        <dd>Low-power network standby. Not truly &ldquo;off&rdquo;&hairsp;&mdash;&hairsp;the receiver maintains network connectivity so the API can wake it.</dd>
        <dt>Unreachable</dt>
        <dd>Truly powered off or unplugged. The API cannot communicate with the device in this state.</dd>
      </dl>
      <p class="note">Turning &ldquo;off&rdquo; via this UI puts the receiver into Standby, not a full power-off.</p>
    </InfoPopover>

    <InfoPopover ref="arcamPopover" popover-id="pop-arcam-power" anchor-name="--arcam-dot">
      <h4>Arcam Amplifier &mdash; Power States</h4>
      <dl>
        <dt>On</dt>
        <dd>Powered on and fully operational. All features and controls are available.</dd>
        <dt>Standby</dt>
        <dd>Low-power network standby. The amplifier maintains network connectivity; a heartbeat signal is sent every 10&nbsp;minutes to keep the interface alive.</dd>
        <dt>Unreachable</dt>
        <dd>Truly powered off or unplugged. The API cannot communicate with the device in this state.</dd>
      </dl>
      <p class="note">Turning &ldquo;off&rdquo; via this UI puts the amplifier into Standby, not a full power-off.</p>
    </InfoPopover>

    <InfoPopover ref="arcamModePopover" popover-id="pop-arcam-mode" anchor-name="--arcam-mode">
      <h4>Arcam Amplifier &mdash; Mode</h4>
      <dl>
        <dt>Stereo</dt>
        <dd>Standard two-channel operation. Left and right speakers receive independent audio signals.</dd>
        <dt>Bridged</dt>
        <dd>Both amplifier channels are combined to drive a single speaker pair, roughly doubling the output wattage.</dd>
        <dt>Dual Mono</dt>
        <dd>Both channels receive the same mono signal, amplified independently. Used for bi-amping a single speaker pair.</dd>
      </dl>
    </InfoPopover>

    <FrostOverlay :visible="isUnreachable" />
    <AddServiceFab @click="showAddModal = true" />
    <AddServiceModal
      :visible="showAddModal"
      @close="showAddModal = false"
      @created="refreshDevices()"
    />
  </div>
</template>

<style scoped>
.dashboard {
  max-width: 600px;
  margin: 0 auto;
  padding: 30px 20px 60px;
}

h1 {
  margin: 0;
  font-size: 1.4em;
  font-weight: 700;
  color: var(--c-text);
}

.subtitle {
  color: var(--c-text-muted);
  font-size: 0.85em;
  margin: 4px 0 24px;
}

.sources {
  display: grid;
  grid-template-rows: 0fr;
  transition: grid-template-rows 0.35s ease, margin-top 0.35s ease, padding-top 0.35s ease;
  overflow: hidden;
  margin-top: 0;
  padding-top: 0;
}

.sources > :deep(.badge-row) {
  min-height: 0;
  justify-content: flex-end;
}

.sources.visible {
  grid-template-rows: 1fr;
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px solid var(--c-popover-border);
}

.device-model {
  color: var(--c-text-dim);
  font-size: 0.8em;
  font-weight: 500;
  letter-spacing: 0.03em;
  line-height: 1;
}

.heartbeat {
  display: inline-block;
  animation: pulse 2s ease-in-out infinite;
  line-height: 1;
  font-size: 0.9em;
}

@keyframes pulse {
  0%, 100% { opacity: 0.3; }
  50% { opacity: 1; }
}

.explore {
  margin-top: 24px;
  color: var(--c-text-muted);
}

.explore a {
  color: var(--c-accent);
  text-decoration: none;
}

.explore a:hover {
  text-decoration: underline;
}
</style>
