<script setup lang="ts">
import { useSSE } from '../composables/useSSE'
import StatsCards from './StatsCards.vue'
import RequestTable from './RequestTable.vue'
import { Trash2 } from 'lucide-vue-next'

const { events, clearEvents } = useSSE()
</script>

<template>
  <div class="space-y-4">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <h1 class="text-lg font-semibold">Live API Logs</h1>
      <button
        @click="clearEvents"
        class="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground transition-colors"
      >
        <Trash2 class="h-3.5 w-3.5" />
        Clear
      </button>
    </div>

    <!-- Stats -->
    <StatsCards :events="events" />

    <!-- Request Log -->
    <div>
      <div class="flex items-center justify-between mb-2">
        <h2 class="text-sm font-medium text-muted-foreground">Live Requests</h2>
        <span class="text-xs text-muted-foreground tabular-nums">{{ events.length }} events</span>
      </div>
      <RequestTable :events="events" />
    </div>
  </div>
</template>
