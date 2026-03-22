<script setup lang="ts">
import { ref } from 'vue'
import { ChevronRight } from 'lucide-vue-next'
import AppBadge from './ui/AppBadge.vue'
import type { ApiRequestEvent } from '../types/request'

defineProps<{
  events: readonly ApiRequestEvent[]
}>()

const expandedId = ref<string | null>(null)

function toggleExpand(id: string) {
  expandedId.value = expandedId.value === id ? null : id
}

function formatTime(timestamp: string): string {
  try {
    const d = new Date(timestamp)
    if (isNaN(d.getTime())) return timestamp
    const h = String(d.getHours()).padStart(2, '0')
    const m = String(d.getMinutes()).padStart(2, '0')
    const s = String(d.getSeconds()).padStart(2, '0')
    const ms = String(d.getMilliseconds()).padStart(3, '0')
    return `${h}:${m}:${s}.${ms}`
  } catch {
    return timestamp
  }
}

type BadgeVariant = 'success' | 'info' | 'warning' | 'destructive' | 'secondary'

function methodVariant(method: string): BadgeVariant {
  const map: Record<string, BadgeVariant> = {
    GET: 'success',
    POST: 'info',
    PUT: 'warning',
    DELETE: 'destructive',
    PATCH: 'warning',
  }
  return map[method] || 'secondary'
}

function statusVariant(code: number): BadgeVariant {
  if (code < 300) return 'success'
  if (code < 400) return 'warning'
  return 'destructive'
}
</script>

<template>
  <div class="overflow-auto h-[calc(100vh-280px)] rounded-lg border">
    <div v-if="events.length === 0" class="flex items-center justify-center h-64 text-muted-foreground text-sm">
      Waiting for requests...
    </div>

    <table v-else class="w-full text-sm">
      <thead class="sticky top-0 bg-card border-b z-10">
        <tr class="text-left text-xs text-muted-foreground">
          <th class="px-2 py-2 w-8"></th>
          <th class="px-3 py-2 w-28">Time</th>
          <th class="px-3 py-2 w-16">Method</th>
          <th class="px-3 py-2">Path</th>
          <th class="px-3 py-2 w-36">Service</th>
          <th class="px-3 py-2 w-44">Action</th>
          <th class="px-3 py-2 w-16">Status</th>
          <th class="px-3 py-2 w-20 text-right">Duration</th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="event in events"
          :key="event.id"
          class="border-b border-border/50 group"
        >
          <td colspan="8" class="p-0">
            <!-- Summary row -->
            <div
              class="flex items-center cursor-pointer hover:bg-accent/50 transition-colors"
              @click="toggleExpand(event.id)"
            >
              <div class="px-2 py-1.5 w-8 flex-shrink-0">
                <ChevronRight
                  :class="[
                    'h-3.5 w-3.5 text-muted-foreground transition-transform duration-200',
                    expandedId === event.id ? 'rotate-90' : '',
                  ]"
                />
              </div>
              <div class="px-3 py-1.5 w-28 flex-shrink-0 font-mono text-xs text-muted-foreground">
                {{ formatTime(event.timestamp) }}
              </div>
              <div class="px-3 py-1.5 w-16 flex-shrink-0">
                <AppBadge :variant="methodVariant(event.method)" class="font-mono text-[10px] w-14 justify-center">
                  {{ event.method }}
                </AppBadge>
              </div>
              <div class="px-3 py-1.5 flex-1 font-mono text-xs truncate min-w-0" :title="event.path">
                {{ event.path }}
              </div>
              <div class="px-3 py-1.5 w-36 flex-shrink-0">
                <AppBadge variant="outline" class="text-[10px]">{{ event.service }}</AppBadge>
              </div>
              <div class="px-3 py-1.5 w-44 flex-shrink-0 text-xs text-muted-foreground truncate" :title="event.action">
                {{ event.action }}
              </div>
              <div class="px-3 py-1.5 w-16 flex-shrink-0">
                <AppBadge :variant="statusVariant(event.status_code)" class="font-mono text-[10px]">
                  {{ event.status_code }}
                </AppBadge>
              </div>
              <div class="px-3 py-1.5 w-20 flex-shrink-0 font-mono text-xs text-right text-muted-foreground tabular-nums">
                {{ event.duration_ms }}ms
              </div>
            </div>

            <!-- Detail panel -->
            <div v-if="expandedId === event.id" class="border-t border-border/30">
              <div class="grid grid-cols-2 gap-4 p-4 bg-card/50 text-sm">
                <!-- Request side -->
                <div class="space-y-3">
                  <h3 class="text-xs font-semibold text-primary uppercase tracking-wider">Request</h3>
                  <div>
                    <h4 class="text-xs font-medium text-muted-foreground mb-1">Headers</h4>
                    <div class="bg-background rounded-md p-3 border border-border/50 max-h-48 overflow-y-auto">
                      <span v-if="Object.keys(event.request_headers).length === 0" class="text-muted-foreground italic text-xs">No headers</span>
                      <div v-else class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-0.5 text-xs">
                        <template v-for="(value, key) in event.request_headers" :key="key">
                          <span class="text-muted-foreground font-mono">{{ key }}</span>
                          <span class="font-mono text-foreground break-all">{{ value }}</span>
                        </template>
                      </div>
                    </div>
                  </div>
                  <div>
                    <h4 class="text-xs font-medium text-muted-foreground mb-1">Body</h4>
                    <span v-if="!event.request_body" class="text-muted-foreground italic text-xs">Empty</span>
                    <pre v-else class="text-xs font-mono bg-background rounded-md p-3 overflow-x-auto max-h-64 overflow-y-auto whitespace-pre-wrap break-all border border-border/50">{{ event.request_body }}</pre>
                  </div>
                </div>

                <!-- Response side -->
                <div class="space-y-3">
                  <h3 class="text-xs font-semibold text-primary uppercase tracking-wider">Response</h3>
                  <div>
                    <h4 class="text-xs font-medium text-muted-foreground mb-1">Headers</h4>
                    <div class="bg-background rounded-md p-3 border border-border/50 max-h-48 overflow-y-auto">
                      <span v-if="Object.keys(event.response_headers).length === 0" class="text-muted-foreground italic text-xs">No headers</span>
                      <div v-else class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-0.5 text-xs">
                        <template v-for="(value, key) in event.response_headers" :key="key">
                          <span class="text-muted-foreground font-mono">{{ key }}</span>
                          <span class="font-mono text-foreground break-all">{{ value }}</span>
                        </template>
                      </div>
                    </div>
                  </div>
                  <div>
                    <h4 class="text-xs font-medium text-muted-foreground mb-1">Body</h4>
                    <span v-if="!event.response_body" class="text-muted-foreground italic text-xs">Empty</span>
                    <pre v-else class="text-xs font-mono bg-background rounded-md p-3 overflow-x-auto max-h-64 overflow-y-auto whitespace-pre-wrap break-all border border-border/50">{{ event.response_body }}</pre>
                  </div>
                </div>
              </div>
            </div>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>
