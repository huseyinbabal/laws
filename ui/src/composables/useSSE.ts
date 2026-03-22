import { ref, onMounted, onUnmounted, provide, inject, readonly } from 'vue'
import type { Ref, DeepReadonly } from 'vue'
import type { ApiRequestEvent } from '../types/request'

const MAX_EVENTS = 200

interface SSEState {
  events: DeepReadonly<Ref<ApiRequestEvent[]>>
  connected: DeepReadonly<Ref<boolean>>
  clearEvents: () => void
}

const SSE_KEY = Symbol('sse')

export function provideSSE(): void {
  const events = ref<ApiRequestEvent[]>([])
  const connected = ref(false)
  let es: EventSource | null = null

  const clearEvents = () => {
    events.value = []
  }

  onMounted(() => {
    es = new EventSource('/api/dashboard/events')

    es.addEventListener('request', (e) => {
      const event: ApiRequestEvent = JSON.parse(e.data)
      const next = [event, ...events.value]
      events.value = next.length > MAX_EVENTS ? next.slice(0, MAX_EVENTS) : next
    })

    es.onopen = () => {
      connected.value = true
    }

    es.onerror = () => {
      connected.value = false
    }
  })

  onUnmounted(() => {
    es?.close()
  })

  provide(SSE_KEY, {
    events: readonly(events),
    connected: readonly(connected),
    clearEvents,
  } satisfies SSEState)
}

export function useSSE(): SSEState {
  const state = inject<SSEState>(SSE_KEY)
  if (!state) {
    throw new Error('useSSE must be used within a component that called provideSSE()')
  }
  return state
}
