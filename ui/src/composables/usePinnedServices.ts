import { ref, provide, inject } from 'vue'
import type { Ref } from 'vue'
import { SERVICE_BY_ID } from '../data/aws-services'
import type { AwsService } from '../data/aws-services'

const STORAGE_KEY = 'laws-pinned-services'
const PINNED_KEY = Symbol('pinned-services')

interface PinnedState {
  pinnedIds: Ref<Set<string>>
  pinnedServices: () => AwsService[]
  togglePin: (serviceId: string) => void
  isPinned: (serviceId: string) => boolean
}

function loadPinned(): Set<string> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) return new Set()
    const arr = JSON.parse(raw)
    if (Array.isArray(arr)) return new Set(arr.filter((id: string) => SERVICE_BY_ID.has(id)))
  } catch {
    // ignore
  }
  return new Set()
}

function savePinned(ids: Set<string>): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify([...ids]))
}

export function providePinnedServices(): void {
  const pinnedIds = ref(loadPinned())

  const togglePin = (serviceId: string) => {
    const next = new Set(pinnedIds.value)
    if (next.has(serviceId)) {
      next.delete(serviceId)
    } else {
      next.add(serviceId)
    }
    pinnedIds.value = next
    savePinned(next)
  }

  const isPinned = (serviceId: string): boolean => {
    return pinnedIds.value.has(serviceId)
  }

  const pinnedServices = (): AwsService[] => {
    return [...pinnedIds.value]
      .map((id) => SERVICE_BY_ID.get(id))
      .filter((s): s is AwsService => !!s)
  }

  provide(PINNED_KEY, { pinnedIds, pinnedServices, togglePin, isPinned } satisfies PinnedState)
}

export function usePinnedServices(): PinnedState {
  const state = inject<PinnedState>(PINNED_KEY)
  if (!state) {
    throw new Error('usePinnedServices must be used within a component that called providePinnedServices()')
  }
  return state
}
