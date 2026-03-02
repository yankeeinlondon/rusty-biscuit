<template>
  <div class="badge-row">
    <slot />
  </div>
</template>

<style scoped>
.badge-row {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}
</style>
