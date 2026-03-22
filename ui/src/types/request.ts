export interface ApiRequestEvent {
  id: string
  timestamp: string
  method: string
  path: string
  service: string
  action: string
  status_code: number
  duration_ms: number
  request_headers: Record<string, string>
  request_body: string | null
  response_headers: Record<string, string>
  response_body: string | null
}
