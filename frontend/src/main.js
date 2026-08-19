import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import router from './router'
import './styles/tokens.css'
import './styles/app.css'

// PWA Service Worker（PRD 9.2：离线缓存静态资源，弱网可用）
if ('serviceWorker' in navigator) {
  window.addEventListener('load', () => {
    navigator.serviceWorker.register('/sw.js').catch(() => {
      /* 开发环境忽略注册失败 */
    })
  })
}

createApp(App).use(createPinia()).use(router).mount('#app')
