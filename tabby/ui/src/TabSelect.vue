<script setup lang="ts">
import { ref } from 'vue'
import { motion } from 'motion-v'

const tabs = ["Home", "React", "Vue", "Svelte"]
const selectedTab = ref(0)

const setSelectedTab = (index: number) => {
  selectedTab.value = index
}
</script>

<template>
  <nav class="container">
    <ul>
      <li
        v-for="(name, index) in tabs"
        :key="index"
        :class="{ selected: selectedTab === index }"
        role="tab"
        :aria-selected="selectedTab === index"
      >
        <motion.div
          v-if="selectedTab === index"
          layout-id="selected-indicator"
          class="selected-indicator"
        />
        <motion.button
          @press-start="() => setSelectedTab(index)"
          :while-press="{ scale: 0.9 }"
          :while-focus="{ backgroundColor: 'var(--accent-transparent)' }"
        >
          {{ name }}
        </motion.button>
      </li>
    </ul>
  </nav>
</template>

<style>
.container {
  background-color: #0b1011;
  border-radius: 10px;
  border: 1px solid #1d2628;
  padding: 5px;
}

.container ul {
  display: flex;
  gap: 5px;
  flex-direction: row;
  align-items: center;
  justify-content: center;
}

.container li {
  color: #f5f5f5;
  position: relative;
}

.container .selected-indicator {
  background-color: #ff0088;
  position: absolute;
  top: 0;
  left: 0;
  bottom: 0;
  right: 0;
  z-index: 1;
  border-radius: 5px;
}

.container button {
  z-index: 2;
  position: relative;
  cursor: pointer;
  padding: 10px 14px;
  border-radius: 5px;
}
</style>
