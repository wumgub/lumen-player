import type { LumenApi } from './index'

declare global {
  interface Window {
    lumen: LumenApi
  }
}

export {}
