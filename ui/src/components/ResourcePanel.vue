<script setup lang="ts">
import { ref, watch, onUnmounted } from 'vue'
import { Database, RefreshCw, Inbox } from 'lucide-vue-next'
import AppCard from './ui/AppCard.vue'

const props = defineProps<{
  serviceId: string
}>()

interface ResourceResponse {
  service: string
  resourceType: string
  resources: Record<string, unknown>[]
}

const data = ref<ResourceResponse | null>(null)
const loading = ref(false)
const error = ref<string | null>(null)
let pollTimer: ReturnType<typeof setInterval> | null = null

async function fetchResources() {
  loading.value = true
  error.value = null
  try {
    const resp = await fetch(`/api/dashboard/resources/${props.serviceId}`)
    if (!resp.ok) throw new Error(`HTTP ${resp.status}`)
    data.value = await resp.json()
  } catch (e: unknown) {
    error.value = (e as Error).message
  } finally {
    loading.value = false
  }
}

function startPolling() {
  stopPolling()
  fetchResources()
  pollTimer = setInterval(fetchResources, 5000)
}

function stopPolling() {
  if (pollTimer) {
    clearInterval(pollTimer)
    pollTimer = null
  }
}

watch(
  () => props.serviceId,
  () => startPolling(),
  { immediate: true },
)

onUnmounted(() => stopPolling())

function columnHeaders(resources: Record<string, unknown>[]): string[] {
  if (resources.length === 0) return []
  return Object.keys(resources[0])
}

function formatHeader(key: string): string {
  return key.replace(/([A-Z])/g, ' $1').replace(/^./, (s) => s.toUpperCase())
}
</script>

<template>
  <AppCard>
    <template #header>
      <Database class="h-4 w-4" />
      Resources
      <span v-if="data" class="text-muted-foreground font-normal ml-1">
        ({{ data.resources.length }} {{ data.resourceType }})
      </span>
      <button
        @click="fetchResources"
        class="ml-auto p-1 rounded hover:bg-accent/50 transition-colors text-muted-foreground hover:text-foreground"
        title="Refresh"
      >
        <RefreshCw class="h-3.5 w-3.5" :class="{ 'animate-spin': loading }" />
      </button>
    </template>

    <!-- Loading -->
    <div v-if="loading && !data" class="flex items-center justify-center h-24 text-sm text-muted-foreground">
      Loading resources...
    </div>

    <!-- Error -->
    <div v-else-if="error" class="flex items-center justify-center h-24 text-sm text-destructive">
      Failed to load: {{ error }}
    </div>

    <!-- Empty -->
    <div
      v-else-if="data && data.resources.length === 0"
      class="flex flex-col items-center justify-center h-24 gap-2 text-sm text-muted-foreground"
    >
      <Inbox class="h-6 w-6" />
      No resources found
    </div>

    <!-- Table -->
    <div v-else-if="data && data.resources.length > 0" class="overflow-x-auto">
      <table class="w-full text-sm">
        <thead>
          <tr class="border-b border-border">
            <th
              v-for="col in columnHeaders(data.resources)"
              :key="col"
              class="text-left p-2 text-xs font-medium text-muted-foreground uppercase tracking-wider"
            >
              {{ formatHeader(col) }}
            </th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="(resource, idx) in data.resources"
            :key="idx"
            class="border-b border-border/50 hover:bg-accent/30 transition-colors"
          >
            <td
              v-for="col in columnHeaders(data.resources)"
              :key="col"
              class="p-2 font-mono text-xs"
            >
              {{ resource[col] ?? '—' }}
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </AppCard>
</template>
</script>
