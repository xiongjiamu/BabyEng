/* BabyEng Service Worker（离线缓存静态资源，弱网可用）
   策略：构建产物缓存优先（cache-first），API 请求始终走网络（network-only）。
   站点存储可能被 iOS 回收（9.2）——缓存丢失后可静默重建。 */

const CACHE = 'babyeng-v1'
const STATIC = [
  '/',
  '/index.html',
  '/manifest.webmanifest',
  '/icons/icon.svg',
]

self.addEventListener('install', (event) => {
  self.skipWaiting()
  event.waitUntil(
    caches
      .open(CACHE)
      .then((c) => c.addAll(STATIC))
      .catch(() => {})
  )
})

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((k) => k !== CACHE).map((k) => caches.delete(k))))
  )
  self.clients.claim()
})

self.addEventListener('fetch', (event) => {
  const url = new URL(event.request.url)

  // API 永不缓存（5.4：录音/结果实时性优先）
  if (url.pathname.startsWith('/api/')) return

  // 导航请求：网络优先，失败回退缓存首页
  if (event.request.mode === 'navigate') {
    event.respondWith(
      fetch(event.request)
        .then((resp) => {
          const copy = resp.clone()
          caches.open(CACHE).then((c) => c.put('/index.html', copy)).catch(() => {})
          return resp
        })
        .catch(() => caches.match('/index.html'))
    )
    return
  }

  // 静态资源：缓存优先，未命中回源并缓存
  event.respondWith(
    caches.match(event.request).then((hit) => {
      if (hit) return hit
      return fetch(event.request).then((resp) => {
        if (resp.ok && (url.origin === location.origin)) {
          const copy = resp.clone()
          caches.open(CACHE).then((c) => c.put(event.request, copy)).catch(() => {})
        }
        return resp
      })
    })
  )
})
